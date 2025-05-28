use std::process::Command;
use std::thread;
use std::cell::RefCell;
use std::os::raw::c_int;

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
    static STARTPLASMA_PID: RefCell<u32> = RefCell::new(0);
}

extern "C" fn sigterm_handler(signal: c_int) {
    println!("Received SIGTERM signal: {signal}");
    // You can add cleanup code here if needed

    unsafe { libc::kill(STARTPLASMA_PID.take() as i32, libc::SIGTERM) };
}

impl Runner for PlasmaRunner {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut child = self.command.spawn()?;
        STARTPLASMA_PID.set(child.id());

        unsafe {
            // Set the signal handler for SIGTERM
            let result = libc::signal(libc::SIGTERM, sigterm_handler as *const () as *const libc::c_void as libc::sighandler_t);
            if result == 0 {
                eprintln!("Failed to set signal handler");
            }
        }

        let result = child.wait()?;
        if !result.success() {
            panic!("plasma failed with {result}")
        } else {
            println!("plasma exited with {result}")
        }

        let exit_status = result.code();

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
