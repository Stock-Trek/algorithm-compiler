use crate::{
    error::{ACError, ACResult},
    timeouts,
};
use std::{
    io::Read,
    path::Path,
    process::{Child, Command, ExitStatus, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

/// How often a running command is polled for completion.
const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub struct Program;

impl Program {
    pub fn run(program: &str, args: &[&str], cwd: &Path) -> ACResult<String> {
        Self::run_with_timeout(program, args, cwd, timeouts::command())
    }

    pub fn run_with_timeout(
        program: &str,
        args: &[&str],
        cwd: &Path,
        timeout: Duration,
    ) -> ACResult<String> {
        let output = Self::output_with_timeout(program, args, cwd, timeout)?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into())
        } else {
            Err(ACError::CommandOutput(
                String::from_utf8_lossy(&output.stderr).into(),
            ))
        }
    }

    pub fn output_with_timeout(
        program: &str,
        args: &[&str],
        cwd: &Path,
        timeout: Duration,
    ) -> ACResult<Output> {
        let child = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(ACError::CommandRun)?;
        Self::collect_output(child, program, timeout)
    }

    pub fn pipe(commands: &mut [Command]) -> ACResult<Output> {
        Self::pipe_with_timeout(commands, timeouts::command())
    }

    pub fn pipe_with_timeout(commands: &mut [Command], timeout: Duration) -> ACResult<Output> {
        if commands.is_empty() {
            return Err(ACError::InternalServer(
                "pipe needs at least one command".into(),
            ));
        }
        let command_len = commands.len();
        let mut children: Vec<Child> = Vec::with_capacity(command_len);
        let mut programs: Vec<String> = Vec::with_capacity(command_len);
        let mut prev_stdout: Option<std::process::ChildStdout> = None;
        for cmd in commands.iter_mut() {
            programs.push(cmd.get_program().to_string_lossy().into_owned());
            if let Some(out) = prev_stdout.take() {
                cmd.stdin(out);
            }
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
            match cmd.spawn() {
                Ok(mut child) => {
                    prev_stdout = child.stdout.take();
                    children.push(child);
                }
                Err(error) => {
                    for child in children.iter_mut() {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                    return Err(ACError::FileSystem(error));
                }
            }
        }

        let mut stderr_handles = Vec::with_capacity(command_len);
        for child in children.iter_mut() {
            let stderr = child
                .stderr
                .take()
                .ok_or_else(|| ACError::InternalServer("missing stderr pipe".into()))?;
            stderr_handles.push(thread::spawn(move || read_to_end(stderr)));
        }
        let last_stdout_handle =
            prev_stdout.map(|stdout| thread::spawn(move || read_to_end(stdout)));

        let mut statuses: Vec<Option<ExitStatus>> = (0..command_len).map(|_| None).collect();
        let start = Instant::now();
        loop {
            let mut all_finished = true;
            for (index, child) in children.iter_mut().enumerate() {
                if statuses[index].is_none() {
                    statuses[index] = child.try_wait().map_err(ACError::FileSystem)?;
                }
                all_finished &= statuses[index].is_some();
            }
            if all_finished {
                break;
            }
            if start.elapsed() >= timeout {
                for child in children.iter_mut() {
                    let _ = child.kill();
                }
                for child in children.iter_mut() {
                    let _ = child.wait();
                }
                return Err(ACError::Timeout(format!(
                    "{} exceeded timeout of {timeout:?}",
                    programs.join(" | ")
                )));
            }
            thread::sleep(COMMAND_POLL_INTERVAL);
        }

        let mut last_stdout = match last_stdout_handle {
            Some(handle) => join_reader(handle)?,
            None => Vec::new(),
        };
        let mut stderrs = Vec::with_capacity(command_len);
        for handle in stderr_handles {
            stderrs.push(join_reader(handle)?);
        }

        let last = command_len - 1;
        let mut outputs = Vec::with_capacity(command_len);
        for (index, (status, stderr)) in statuses.into_iter().zip(stderrs).enumerate() {
            let status = status
                .ok_or_else(|| ACError::InternalServer("missing command exit status".into()))?;
            let stdout = if index == last {
                std::mem::take(&mut last_stdout)
            } else {
                Vec::new()
            };
            outputs.push(Output {
                status,
                stdout,
                stderr,
            });
        }
        for (program, output) in programs.iter().zip(&outputs) {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(ACError::CommandOutput(format!(
                    "{program} failed with {}: {}",
                    output.status,
                    stderr.trim()
                )));
            }
        }
        outputs
            .pop()
            .ok_or_else(|| ACError::InternalServer("pipe produced no output".into()))
    }

    fn collect_output(mut child: Child, program: &str, timeout: Duration) -> ACResult<Output> {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ACError::InternalServer("missing stdout pipe".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ACError::InternalServer("missing stderr pipe".into()))?;
        let stdout_handle = thread::spawn(move || read_to_end(stdout));
        let stderr_handle = thread::spawn(move || read_to_end(stderr));
        let status = Self::wait_with_timeout(&mut child, program, timeout)?;
        let stdout = join_reader(stdout_handle)?;
        let stderr = join_reader(stderr_handle)?;
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }

    fn wait_with_timeout(
        child: &mut Child,
        program: &str,
        timeout: Duration,
    ) -> ACResult<ExitStatus> {
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().map_err(ACError::FileSystem)? {
                return Ok(status);
            }
            if start.elapsed() >= timeout {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ACError::Timeout(format!(
                    "{program} exceeded timeout of {timeout:?}"
                )));
            }
            thread::sleep(COMMAND_POLL_INTERVAL);
        }
    }
}

fn read_to_end<R: Read>(mut reader: R) -> std::io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn join_reader(handle: thread::JoinHandle<std::io::Result<Vec<u8>>>) -> ACResult<Vec<u8>> {
    handle
        .join()
        .map_err(|_| ACError::InternalServer("process output reader panicked".into()))?
        .map_err(ACError::FileSystem)
}
