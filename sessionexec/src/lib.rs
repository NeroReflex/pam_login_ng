use std::{error::Error, process::Command};

pub mod runner;
pub mod execve;
pub mod plasma;
pub mod gamescope;

pub(crate) fn find_program_path(program: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("which").arg(program).output()?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(path)
    } else {
        Err(format!("Program '{}' not found in PATH", program).into())
    }
}
