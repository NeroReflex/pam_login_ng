use std::ffi::CString;

use crate::{execve_wrapper, find_program_path, runner::Runner};

pub struct ExecveRunner {
    prog: CString,
    argv_data: Vec<CString>,
    envp_data: Vec<CString>,
}

impl ExecveRunner {
    pub fn new(splitted: Vec<String>) -> Self {
        let mut argv_data: Vec<CString> = vec![];
        let mut prog = CString::new(splitted[0].clone()).unwrap();

        for (idx, val) in splitted.iter().enumerate() {
            let c_string = CString::new(val.as_str()).expect("CString::new failed");
            if idx == 0 {
                prog = match find_program_path(val.as_str()) {
                    Ok(program_path) => CString::new(program_path.as_str()).unwrap(),
                    Err(err) => {
                        eprintln!("Error searching for the specified program: {err}");
                        c_string.clone()
                    }
                }
            }

            argv_data.push(c_string);
        }

        let mut envp_data: Vec<CString> = vec![];
        for (key, value) in std::env::vars() {
            let env_var = format!("{}={}", key, value);
            let c_string = CString::new(env_var).unwrap();
            envp_data.push(c_string);
        }

        Self {
            prog,
            argv_data,
            envp_data,
        }
    }
}

impl Runner for ExecveRunner {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        execve_wrapper(
            &self.prog,
            &self.argv_data,
            &self.envp_data,
        )
    }
}
