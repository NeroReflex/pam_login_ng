use std::ffi::CString;

use crate::{find_program_path, runner::Runner};

pub struct ExecveRunner {
    prog: CString,
    argv_data: Vec<CString>,
    envp_data: Vec<CString>,
}

impl ExecveRunner {
    pub fn new(splitted: Vec<String>) -> Self {
        let mut argv_data: Vec<CString> = vec![];
        let mut prog = CString::new("false").unwrap();

        for (idx, val) in splitted.iter().enumerate() {
            let c_string = CString::new(val.as_str()).expect("CString::new failed");
            if idx == 0 {
                prog = match find_program_path(val.as_str()) {
                    Ok(program_path) => CString::new(program_path.as_str()).unwrap(),
                    Err(err) => {
                        println!("Error searching for the specified program: {err}");
                        c_string.clone()
                    }
                }
            }

            println!("argv[{idx}]: {val}");

            argv_data.push(c_string);
        }

        let mut envp_data: Vec<CString> = vec![];
        for (idx, (key, value)) in std::env::vars().enumerate() {
            let env_var = format!("{}={}", key, value);
            println!("envp[{idx}]: {env_var}");
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
        let prog = self.prog.as_ptr();

        let argv = self.argv_data
            .iter()
            .map(|e| e.as_ptr())
            .chain(std::iter::once(std::ptr::null()))
            .collect::<Vec<*const libc::c_char>>();

        let envp = self.envp_data
            .iter()
            .map(|e| e.as_ptr())
            .chain(std::iter::once(std::ptr::null()))
            .collect::<Vec<*const libc::c_char>>();

        let execve_err = unsafe { libc::execve(prog, argv.as_ptr(), envp.as_ptr()) };

        if execve_err == -1 {
            return Err(format!("execve failed: {}", std::io::Error::last_os_error()).into());
        }

        unreachable!()
    }
}