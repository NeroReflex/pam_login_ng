use std::process::Command;
use std::thread;
use signal_hook::{consts::SIGTERM, iterator::Signals};

use crate::{find_program_path, runner::Runner};

pub struct PlasmaRunner {
    command: Command,
}

impl PlasmaRunner {
    pub fn new(splitted: Vec<String>) -> Self {
        let mut argv_data: Vec<String> = vec![];
        let mut prog = String::new();

        for (idx, val) in splitted.iter().enumerate() {
            let string = val.clone();
            match idx {
                0 => prog = string,
                _ => argv_data.push(string),
            }
        }

        let mut command = Command::new(prog);
        for arg in argv_data.iter() {
            command.arg(arg);
        }

        for (key, val) in std::env::vars() {
            command.env(key, val);
        }

        Self {
            command
        }
    }
}

impl Runner for PlasmaRunner {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut child = self.command.spawn()?;
        let pid = child.id();

        let mut signals = Signals::new([SIGTERM])?;

        thread::spawn(move || {
            for sig in signals.forever() {
                println!("Received signal {:?}", sig);
                unsafe { libc::kill(pid as i32, SIGTERM as i32)};
            }
        });

        let result = child.wait()?;

        if !result.success() {
            panic!("plasma exited with {result}")
        }

        let exit_status = result.code();

        // Main application loop
        loop {
            let wait_cmd = "kwin_wayland";
            println!("Awaiting {wait_cmd} to exit...");

            thread::sleep(std::time::Duration::from_millis(250));

            // Check if the command is running
            let output = Command::new("pgrep")
                .arg("-u")
                .arg(whoami::username()) // Get the current username
                .arg(wait_cmd)
                .output()
                .expect("Failed to execute pgrep");

            if output.status.success() {
                println!("{wait_cmd} still running...");
            } else {
                break;
            }
        }

        match exit_status {
            Some(exit_code) =>  unsafe { libc::exit(exit_code) },
            None => Ok(()),
        }
    }
}