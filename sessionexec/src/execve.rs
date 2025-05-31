use crate::{cstr::CStr, execve_wrapper, find_program_path, runner::Runner};

pub struct ExecveRunner {
    prog: CStr,
    argv_data: Vec<CStr>,
    envp_data: Vec<CStr>,
}

impl ExecveRunner {
    pub fn new(splitted: Vec<String>) -> Self {
        let mut argv_data: Vec<CStr> = vec![];
        let mut prog = CStr::new(splitted[0].as_str()).unwrap();

        for (idx, val) in splitted.iter().enumerate() {
            let c_string = CStr::new(val.as_str()).expect("CStr::new failed");
            if idx == 0 {
                prog = match find_program_path(val.as_str()) {
                    Ok(program_path) => CStr::new(program_path.as_str()).unwrap(),
                    Err(err) => {
                        eprintln!("Error searching for the specified program: {err}");
                        c_string
                    }
                }
            }

            argv_data.push(CStr::new(val.as_str()).expect("CStr::new failed"));
        }

        let mut envp_data: Vec<CStr> = vec![];
        for (key, value) in std::env::vars() {
            let c_string = CStr::new(format!("{key}={value}").as_str()).unwrap();
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
        execve_wrapper(&self.prog, &self.argv_data, &self.envp_data)
    }
}
