use crate::{execve_wrapper, find_program_path, runner::Runner};
use std::ffi::{CString, OsStr};
use std::io::{BufReader, Read};
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

pub struct GamescopeExecveRunner {
    gamescope_prog: CString,
    gamescope_argv_data: Vec<CString>,
    gamescope_envp_data: Vec<CString>,
    shared_env: Vec<(String, String)>,
    socket: PathBuf,
    _stats: PathBuf,
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
            _stats: stats,
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
