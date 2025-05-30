use std::cell::RefCell;
use std::os::raw::c_int;
use std::process::Command;
use std::thread;

use crate::runner::Runner;

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

        Self { command }
    }
}

thread_local! {
    static STARTPLASMA_PID: RefCell<u32> = const { RefCell::new(0) };
}

extern "C" fn sigterm_handler(signal: c_int) {
    let pid = STARTPLASMA_PID.take();
    let result = unsafe { libc::kill(pid as i32, signal) };

    if result == 0 {
        // TODO: to avoid issue do not do I/O here
        print!("Signal {signal} propagated to PID {pid}: {result}");
    } else {
        eprintln!("Error propagating signal {signal} to PID {pid}: {result}");
    }
}

impl Runner for PlasmaRunner {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut child = self.command.spawn()?;
        STARTPLASMA_PID.set(child.id());

        // Set the signal handler for SIGTERM
        let result = unsafe {
            libc::signal(
                libc::SIGTERM,
                sigterm_handler as *const () as libc::sighandler_t,
            )
        };
        match result {
            libc::SIG_ERR => eprintln!("Failed to set signal handler: {}", unsafe {
                *libc::__errno_location()
            }),
            _ => println!("signal handler setup correctly, was previously {result}"),
        }

        let mut exit_status = None;
        loop {
            match child.try_wait() {
                Ok(res) => match res {
                    Some(result) => {
                        if !result.success() {
                            panic!("plasma failed with {result}")
                        } else {
                            println!("plasma exited with {result}")
                        }

                        exit_status = result.code();
                        break;
                    }
                    None => {
                        std::thread::sleep(std::time::Duration::from_millis(750));
                        continue;
                    }
                },
                Err(err) => eprintln!("Error waiting for termination: {err}"),
            }
        }

        // wait for the drm to be free (safeguard to avoid gamescope to fail)
        loop {
            let wait_cmd = "kwin_wayland";
            println!("Awaiting {wait_cmd} to exit...");

            thread::sleep(std::time::Duration::from_millis(250));

            // Check if the command is running
            let output = Command::new("pgrep")
                .arg("-u")
                .arg(whoami::username())
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
            Some(exit_code) => unsafe { libc::exit(exit_code) },
            None => Ok(()),
        }
    }
}
