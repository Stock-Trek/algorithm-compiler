use crate::dto::errors::{StockTrekCompileAlgorithmError, internal_server, internal_server_e};
use std::{path::Path, process::Command};

pub struct Program;

impl Program {
    pub fn run(
        program: &str,
        args: &[&str],
        cwd: &Path,
    ) -> Result<(), StockTrekCompileAlgorithmError> {
        let output = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .output()
            .map_err(|e| internal_server_e(&format!("Failed to run {program}"), e))?;
        if !output.status.success() {
            return Err(internal_server(&format!(
                "{} {} failed: {}",
                program,
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }
}
