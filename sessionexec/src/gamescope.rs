use std::ffi::OsStr;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::{path::PathBuf, process::Command};
use std::thread;
use crate::runner::Runner;
use signal_hook::{consts::SIGTERM, iterator::Signals};

pub struct GamescopeRunner {
    command: Command,
    shared_env: Vec<(String, String)>,
}

pub fn mktemp<S>(n: S) -> String
where
    S: AsRef<OsStr>
{
    // Call the mktemp command
    let output = Command::new("mktemp")
        .arg(n)
        .output()
        .expect("Failed to execute mktemp");

    // Check if the command was successful
    if output.status.success() {
        // Convert the output to a string
        let temp_file_path = str::from_utf8(&output.stdout).expect("Invalid UTF-8 output");
        
        // Print the path of the temporary file
        String::from(temp_file_path.trim())
    } else {
        // Handle the error
        let error_message = str::from_utf8(&output.stderr).expect("Invalid UTF-8 error output");
        panic!("Error: {}", error_message)
    }
}

impl GamescopeRunner {
    pub fn new(splitted: Vec<String>) -> Self {
        let xdg_runtime_dir = PathBuf::from(match std::env::var("XDG_RUNTIME_DIR") {
            Ok(env) => env,
            Err(err) => {
                eprint!("Error in fetching XDG_RUNTIME_DIR: {err}");

                String::from("/tmp/")
            }
        });

        let mangohud_configfile = mktemp(xdg_runtime_dir.join("mangohud.XXXXXXXX"));
        std::fs::write(PathBuf::from(&mangohud_configfile), b"no_display").unwrap();

        let radv_force_vrs_config_filec = mktemp(xdg_runtime_dir.join("radv_vrs.XXXXXXXX"));
        std::fs::write(PathBuf::from(&radv_force_vrs_config_filec), b"1x1").unwrap();

        // These are copied from gamescope-session-plus
        let shared_env = vec![
            (String::from("RADV_FORCE_VRS_CONFIG_FILE"), radv_force_vrs_config_filec),
            (String::from("MANGOHUD_CONFIGFILE"), mangohud_configfile),
            // Force Qt applications to run under xwayland
            (String::from("QT_QPA_PLATFORM"), String::from("xcb")),
            // Expose vram info from radv
            (String::from("WINEDLLOVERRIDES"), String::from("dxgi=n")),
            (String::from("SDL_VIDEO_MINIMIZE_ON_FOCUS_LOSS"), String::from("0")),
            // Temporary crutch until dummy plane interactions / etc are figured out
            //(String::from("GAMESCOPE_DISABLE_ASYNC_FLIPS"), String::from("1")),
        ];

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

        for (key, val) in shared_env.iter() {
            command.env(key, val);
        }

        Self {
            shared_env,
            command
        }
    }
}

impl Runner for GamescopeRunner {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut child = self.command.spawn()?;
        let pid = child.id();

        let mut signals = Signals::new([SIGTERM])?;

        let should_exit = Arc::new(Mutex::new(false));

        thread::spawn(move || {
            for sig in signals.forever() {
                println!("Received signal {:?}", sig);
                unsafe { libc::kill(pid as i32, SIGTERM as i32)};
            }
        });

        let mut mangoapp_cmd = Command::new("mangoapp");
        for (key, val) in self.shared_env.iter() {
            mangoapp_cmd.env(key, val);
        }
        
        let mangoapp_should_exit = should_exit.clone();
        let mangoapp_spawner = thread::spawn(move || {
            loop {
                let should_exit_guard = mangoapp_should_exit.lock().unwrap();
                if *should_exit_guard.deref() {
                    break;
                }

                match mangoapp_cmd.spawn() {
                    Ok(mut mangoapp_child) => match mangoapp_child.wait() {
                        Ok(mangoapp_result) => {},
                        Err(err) => {
                            eprint!("Error in mangoapp: {err}")
                        }
                    },
                    Err(err) => {
                        eprint!("Error spawning mangoapp: {err}");
                        thread::sleep(std::time::Duration::from_secs(5));
                    }
                }
            }
        });

        let result = child.wait()?;

        {
            // make other threads exit
            let mut should_exit_guard = should_exit.lock().unwrap();
            *should_exit_guard = true;
        }

        if !result.success() {
            panic!("plasma exited with {result}")
        }

        let exit_status = result.code();

        // here join the mangoapp handle
        mangoapp_spawner.join().unwrap();

        // Main application loop
        loop {
            let wait_cmd = "gamescope";

            // Check if the command is running
            let output = Command::new("pgrep")
                .arg("-u")
                .arg(whoami::username()) // Get the current username
                .arg(wait_cmd)
                .output()
                .expect("Failed to execute pgrep");

            if output.status.success() {
                println!("{wait_cmd} still running...");

                println!("Awaiting {wait_cmd} to exit...");

                thread::sleep(std::time::Duration::from_millis(250));

                continue;
            }

            break;
        }

        match exit_status {
            Some(exit_code) =>  unsafe { libc::exit(exit_code) },
            None => Ok(()),
        }
    }
}