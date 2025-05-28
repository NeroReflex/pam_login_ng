use crate::{execve_wrapper, find_program_path, runner::Runner};
use signal_hook::{
    consts::{SIGKILL, SIGTERM},
    iterator::Signals,
};
use std::ffi::{CString, OsStr};
use std::io::{BufReader, Read};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::thread;
use std::{path::PathBuf, process::Command};

pub fn mktemp<S>(n: S) -> String
where
    S: AsRef<OsStr>,
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

pub fn mktemp_dir<S, Q>(path: S, dir: Q) -> String
where
    S: AsRef<OsStr>,
    Q: AsRef<OsStr>,
{
    // Call the mktemp command
    let output = Command::new("mktemp")
        .arg("-p")
        .arg(path)
        .arg("-d")
        .arg("-t")
        .arg(dir)
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

pub fn mkfifo<S>(n: S) -> ()
where
    S: AsRef<OsStr>,
{
    // Call the mktemp command
    let output = Command::new("mkfifo")
        .arg("--")
        .arg(n)
        .output()
        .expect("Failed to execute mktemp");

    // Check if the command was successful
    if output.status.success() {
        return ();
    }

    // Handle the error
    let error_message = str::from_utf8(&output.stderr).expect("Invalid UTF-8 error output");
    panic!("Error in mkfifo: {}", error_message)
}

pub struct GamescopeRunner {
    command: Command,
    shared_env: Vec<(String, String)>,
    socket: PathBuf,
    stats: PathBuf,
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

        let tmp_dir = PathBuf::from(mktemp_dir(&xdg_runtime_dir, "gamescope.XXXXXXX"));
        let socket = tmp_dir.join("startup.socket");
        let stats = tmp_dir.join("stats.pipe");

        mkfifo(&socket);
        mkfifo(&stats);

        let mangohud_configfile = mktemp(&xdg_runtime_dir.join("mangohud.XXXXXXXX"));
        std::fs::write(PathBuf::from(&mangohud_configfile), b"no_display").unwrap();

        let radv_force_vrs_config_filec = mktemp(xdg_runtime_dir.join("radv_vrs.XXXXXXXX"));
        std::fs::write(PathBuf::from(&radv_force_vrs_config_filec), b"1x1").unwrap();

        // These are copied from gamescope-session-plus
        let shared_env = vec![
            (
                String::from("GAMESCOPE_STATS"),
                String::from(stats.to_string_lossy()),
            ),
            (
                String::from("RADV_FORCE_VRS_CONFIG_FILE"),
                radv_force_vrs_config_filec,
            ),
            (String::from("MANGOHUD_CONFIGFILE"), mangohud_configfile),
            // Force Qt applications to run under xwayland
            (String::from("QT_QPA_PLATFORM"), String::from("xcb")),
            // Expose vram info from radv
            (String::from("WINEDLLOVERRIDES"), String::from("dxgi=n")),
            (
                String::from("SDL_VIDEO_MINIMIZE_ON_FOCUS_LOSS"),
                String::from("0"),
            ),
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

        command.arg("-R");
        command.arg(&socket);
        command.arg("-T");
        command.arg(&stats);

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
            command,
            socket,
            stats,
        }
    }
}

impl Runner for GamescopeRunner {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut child = self.command.spawn()?;
        let pid = child.id();

        let mut signals = Signals::new([SIGTERM])?;

        let should_exit = Arc::new(Mutex::new(false));

        let mangoapp_pid = Arc::new(Mutex::new(None));

        let mangoapp_pid_clone = mangoapp_pid.clone();
        thread::spawn(move || {
            for sig in signals.forever() {
                println!("Received signal {:?}", sig);

                // kill the running mangoapp
                let mut mangoapp_pid_guard = mangoapp_pid_clone.lock().unwrap();
                match mangoapp_pid_guard.deref() {
                    Some(pid) => unsafe {
                        libc::kill(*pid as i32, SIGKILL as i32);
                    },
                    None => {}
                }
                *mangoapp_pid_guard = None;

                // gracefully terminate gamescope
                unsafe { libc::kill(pid as i32, SIGTERM as i32) };
            }
        });

        let mut mangoapp_cmd = Command::new("mangoapp");
        for (key, val) in self.shared_env.iter() {
            mangoapp_cmd.env(key, val);
        }

        // gamescope won't start unless we read data from the socket:
        let file = std::fs::File::open(&self.socket).unwrap();
        let mut reader = BufReader::new(file);

        let mut response = String::new();
        let (response_x_display, response_wl_display) = match reader.read_to_string(&mut response) {
            Ok(read_result) => {
                println!("Read response ({read_result}): {response}");

                let split = response
                    .split_whitespace()
                    .into_iter()
                    .map(|w| String::from(w))
                    .collect::<Vec<String>>();

                if split.len() != 2 {
                    panic!("Invalid read from socket!");
                }

                (split[0].clone(), split[1].clone())
            }
            Err(err) => {
                panic!("Error reading read_wl_display_result: {err}")
            }
        };

        let mangoapp_pid_clone = mangoapp_pid.clone();
        let mangoapp_should_exit = should_exit.clone();
        let mangoapp_spawner = thread::spawn(move || loop {
            let should_exit_guard = mangoapp_should_exit.lock().unwrap();
            if *should_exit_guard.deref() {
                break;
            }

            mangoapp_cmd.env("DISPLAY", &response_x_display);
            mangoapp_cmd.env("GAMESCOPE_WAYLAND_DISPLAY", &response_wl_display);

            match mangoapp_cmd.spawn() {
                Ok(mut mangoapp_child) => {
                    let mut pid_guard = mangoapp_pid_clone.lock().unwrap();
                    *pid_guard = Some(mangoapp_child.id());
                    match mangoapp_child.wait() {
                        Ok(mangoapp_result) => {
                            println!("mangoapp terminated: {mangoapp_result}")
                        }
                        Err(err) => {
                            eprint!("Error in mangoapp: {err}")
                        }
                    }
                }
                Err(err) => {
                    eprint!("Error spawning mangoapp: {err}");
                    thread::sleep(std::time::Duration::from_secs(5));
                }
            }
        });

        let result = child.wait()?;

        {
            // make other threads exit
            let mut should_exit_guard = should_exit.lock().unwrap();
            *should_exit_guard = true;

            // kill the running mangoapp
            let mut mangoapp_pid_guard = mangoapp_pid.lock().unwrap();
            match mangoapp_pid_guard.deref() {
                Some(pid) => unsafe {
                    libc::kill(*pid as i32, SIGKILL as i32);
                },
                None => {}
            }
            *mangoapp_pid_guard = None;
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

            println!("Awaiting {wait_cmd} to exit...");

            // Check if the command is running
            let output = Command::new("pgrep")
                .arg("-u")
                .arg(whoami::username()) // Get the current username
                .arg(wait_cmd)
                .output()
                .expect("Failed to execute pgrep");

            if output.status.success() {
                println!("{wait_cmd} still running...");

                thread::sleep(std::time::Duration::from_millis(250));

                continue;
            }

            break;
        }

        match exit_status {
            Some(exit_code) => unsafe { libc::exit(exit_code) },
            None => Ok(()),
        }
    }
}

pub struct GamescopeExecveRunner {
    gamescope_prog: CString,
    gamescope_argv_data: Vec<CString>,
    gamescope_envp_data: Vec<CString>,
    shared_env: Vec<(String, String)>,
    socket: PathBuf,
    stats: PathBuf,
}

impl GamescopeExecveRunner {
    pub fn new(splitted: Vec<String>) -> Self {
        let xdg_runtime_dir = PathBuf::from(match std::env::var("XDG_RUNTIME_DIR") {
            Ok(env) => env,
            Err(err) => {
                eprint!("Error in fetching XDG_RUNTIME_DIR: {err}");

                String::from("/tmp/")
            }
        });

        let tmp_dir = PathBuf::from(mktemp_dir(&xdg_runtime_dir, "gamescope.XXXXXXX"));
        let socket = tmp_dir.join("startup.socket");
        let stats = tmp_dir.join("stats.pipe");

        mkfifo(&socket);
        mkfifo(&stats);

        let mangohud_configfile = mktemp(&xdg_runtime_dir.join("mangohud.XXXXXXXX"));
        std::fs::write(PathBuf::from(&mangohud_configfile), b"no_display").unwrap();

        let radv_force_vrs_config_filec = mktemp(xdg_runtime_dir.join("radv_vrs.XXXXXXXX"));
        std::fs::write(PathBuf::from(&radv_force_vrs_config_filec), b"1x1").unwrap();

        // These are copied from gamescope-session-plus
        let shared_env = vec![
            (
                String::from("GAMESCOPE_STATS"),
                String::from(stats.to_string_lossy()),
            ),
            (
                String::from("RADV_FORCE_VRS_CONFIG_FILE"),
                radv_force_vrs_config_filec,
            ),
            (String::from("MANGOHUD_CONFIGFILE"), mangohud_configfile),
            // Force Qt applications to run under xwayland
            (String::from("QT_QPA_PLATFORM"), String::from("xcb")),
            // Expose vram info from radv
            (String::from("WINEDLLOVERRIDES"), String::from("dxgi=n")),
            (
                String::from("SDL_VIDEO_MINIMIZE_ON_FOCUS_LOSS"),
                String::from("0"),
            ),
            // Temporary crutch until dummy plane interactions / etc are figured out
            //(String::from("GAMESCOPE_DISABLE_ASYNC_FLIPS"), String::from("1")),
        ];

        // here build the gamescope command
        let mut gamescope_argv_data: Vec<CString> = vec![];
        let mut gamescope_prog = CString::new("false").unwrap();

        for (idx, val) in splitted.iter().enumerate() {
            let c_string = CString::new(val.as_str()).expect("CString::new failed");
            if idx == 0 {
                gamescope_prog = match find_program_path(val.as_str()) {
                    Ok(program_path) => CString::new(program_path.as_str()).unwrap(),
                    Err(err) => {
                        println!("Error searching for the specified program: {err}");
                        c_string.clone()
                    }
                };

                gamescope_argv_data.push(c_string);
                gamescope_argv_data.push(CString::new("-R").unwrap());
                gamescope_argv_data.push(
                    CString::new(socket.as_os_str().to_string_lossy().to_string().as_str())
                        .unwrap(),
                );
                gamescope_argv_data.push(CString::new("-T").unwrap());
                gamescope_argv_data.push(
                    CString::new(stats.as_os_str().to_string_lossy().to_string().as_str()).unwrap(),
                );
            } else {
                gamescope_argv_data.push(c_string);
            }
        }

        let mut gamescope_envp_data: Vec<CString> = vec![];
        for (key, value) in std::env::vars() {
            let env_var = format!("{}={}", key, value);
            let c_string = CString::new(env_var).unwrap();
            gamescope_envp_data.push(c_string);
        }

        for (key, val) in shared_env.iter() {
            let env_var = format!("{}={}", key, val);
            let c_string = CString::new(env_var).unwrap();
            gamescope_envp_data.push(c_string);
        }

        Self {
            gamescope_prog,
            gamescope_argv_data,
            gamescope_envp_data,
            shared_env,
            socket,
            stats,
        }
    }

    fn start_mangoapp(&self) -> Result<(), Box<dyn std::error::Error>> {
        // gamescope won't start unless we read data from the socket:
        let file = std::fs::File::open(&self.socket).unwrap();
        let mut reader = BufReader::new(file);

        let mut response = String::new();
        let (response_x_display, response_wl_display) = match reader.read_to_string(&mut response) {
            Ok(read_result) => {
                println!("Read response ({read_result}): {response}");

                let split = response
                    .split_whitespace()
                    .into_iter()
                    .map(|w| String::from(w))
                    .collect::<Vec<String>>();

                if split.len() != 2 {
                    panic!("Invalid read from socket!");
                }

                (split[0].clone(), split[1].clone())
            }
            Err(err) => {
                panic!("Error reading read_wl_display_result: {err}")
            }
        };

        let mut mangoapp_envp_data: Vec<CString> = vec![];
        let mut mangoapp_argv_data: Vec<CString> = vec![];
        let mangoapp_prog = match find_program_path("mangoapp") {
            Ok(program_path) => CString::new(program_path.as_str()).unwrap(),
            Err(err) => {
                println!("Error searching for the specified program: {err}");
                CString::new("mangoapp").unwrap()
            }
        };

        mangoapp_argv_data.push(mangoapp_prog.clone());

        for (key, value) in std::env::vars() {
            let env_var = format!("{}={}", key, value);
            let c_string = CString::new(env_var).unwrap();
            mangoapp_envp_data.push(c_string);
        }

        for (key, val) in self.shared_env.iter() {
            let env_var = format!("{}={}", key, val);
            let c_string = CString::new(env_var).unwrap();
            mangoapp_envp_data.push(c_string);
        }

        mangoapp_envp_data.push(CString::new(format!("DISPLAY={response_x_display}")).unwrap());
        mangoapp_envp_data.push(
            CString::new(format!("GAMESCOPE_WAYLAND_DISPLAY={response_wl_display}")).unwrap(),
        );

        execve_wrapper(mangoapp_prog, mangoapp_argv_data, mangoapp_envp_data)
    }

    fn start_gamescope(&self) -> Result<(), Box<dyn std::error::Error>> {
        execve_wrapper(
            self.gamescope_prog.clone(),
            self.gamescope_argv_data.clone(),
            self.gamescope_envp_data.clone(),
        )
    }
}

impl Runner for GamescopeExecveRunner {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut mangoapp_cmd = Command::new("mangoapp");
        for (key, val) in self.shared_env.iter() {
            mangoapp_cmd.env(key, val);
        }

        let fork_res = unsafe { libc::fork() };
        if fork_res < 0 {
            panic!("Could not fork the process");
        } else if fork_res == 0 {
            self.start_mangoapp()
        } else {
            self.start_gamescope()
        }
    }
}
