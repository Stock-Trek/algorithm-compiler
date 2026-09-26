use crate::error::{ACError, ACResult};
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
};

pub struct Program;

impl Program {
    pub fn run(program: &str, args: &[&str], cwd: &Path) -> ACResult<String> {
        let output = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .output()
            .map_err(ACError::CommandRun)?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into())
        } else {
            Err(ACError::CommandOutput(
                String::from_utf8_lossy(&output.stderr).into(),
            ))
        }
    }

    pub fn pipe(commands: &mut [Command]) -> ACResult<std::process::Output> {
        if commands.is_empty() {
            return Err(ACError::InternalServer(
                "pipe needs at least one command".into(),
            ));
        }
        let command_len = commands.len();
        let mut children: Vec<std::process::Child> = Vec::with_capacity(command_len);
        let mut programs: Vec<String> = Vec::with_capacity(command_len);
        let mut prev_stdout: Option<std::process::ChildStdout> = None;
        for cmd in commands.iter_mut() {
            programs.push(cmd.get_program().to_string_lossy().into_owned());
            if let Some(out) = prev_stdout.take() {
                cmd.stdin(out);
            }
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
            let mut child = cmd.spawn().map_err(ACError::FileSystem)?;
            prev_stdout = child.stdout.take();
            children.push(child);
        }
        let mut stderr_handles = Vec::with_capacity(command_len);
        for child in children.iter_mut() {
            let mut stderr = child
                .stderr
                .take()
                .ok_or_else(|| ACError::InternalServer("missing stderr pipe".into()))?;
            stderr_handles.push(thread::spawn(move || {
                let mut buffer = Vec::new();
                let _ = stderr.read_to_end(&mut buffer);
                buffer
            }));
        }
        let mut last_stdout = Vec::new();
        if let Some(mut stdout) = prev_stdout {
            stdout
                .read_to_end(&mut last_stdout)
                .map_err(ACError::FileSystem)?;
        }
        let last = command_len - 1;
        let mut outputs = Vec::with_capacity(command_len);
        for (i, (mut child, stderr_handle)) in children.into_iter().zip(stderr_handles).enumerate()
        {
            let status = child.wait().map_err(ACError::FileSystem)?;
            let stderr = stderr_handle
                .join()
                .map_err(|_| ACError::InternalServer("stderr reader panicked".into()))?;
            let stdout = if i == last {
                std::mem::take(&mut last_stdout)
            } else {
                Vec::new()
            };
            outputs.push(std::process::Output {
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
}
