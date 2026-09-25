use crate::error::{ACError, ACResult};
use std::{
    path::Path,
    process::{Command, Stdio},
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
        assert!(!commands.is_empty(), "pipe needs at least one command");
        let command_len = commands.len();
        let mut children: Vec<std::process::Child> = Vec::with_capacity(command_len);
        let mut prev_stdout: Option<std::process::ChildStdout> = None;
        for (i, cmd) in commands.iter_mut().enumerate() {
            if let Some(out) = prev_stdout.take() {
                cmd.stdin(out);
            }
            if i + 1 < command_len {
                cmd.stdout(Stdio::piped());
            }
            let mut child = cmd.spawn().map_err(ACError::FileSystem)?;
            if i + 1 < command_len {
                prev_stdout = child.stdout.take();
            }
            children.push(child);
        }
        let last = children.pop().unwrap();
        let output = last.wait_with_output().map_err(ACError::FileSystem)?;
        for mut c in children {
            c.wait().map_err(ACError::FileSystem)?;
        }
        Ok(output)
    }
}
