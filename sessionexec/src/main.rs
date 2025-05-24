use ini::Ini;
use std::error::Error;
use std::process::Command;
use std::{env, ffi::CString, path::PathBuf};

fn find_program_path(program: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("which").arg(program).output()?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(path)
    } else {
        Err(format!("Program '{}' not found in PATH", program).into())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let param = if args.len() < 2 {
        match std::env::home_dir() {
            Some(home_dir) => {
                let path = home_dir.join(".config").join("sessionexec").join("default");
                if path.exists() && path.is_file() {
                    match std::fs::read_to_string(path) {
                        Ok(session) => session.trim().to_string(),
                        Err(err) => {
                            println!("error reading file ~/.config/sessionexec/default: {err}");
                            String::from("game-mode.desktop")
                        }
                    }
                } else {
                    println!(
                        "file ~/.config/sessionexec/default not found: using game-mode.desktop"
                    );
                    String::from("game-mode.desktop")
                }
            }
            None => String::from("game-mode.desktop"),
        }
    } else {
        args[1].to_string()
    };

    let path = PathBuf::from("/usr/share/wayland-sessions/").join(param);

    let conf = Ini::load_from_file(path).unwrap();

    let section = conf.section(Some("Desktop Entry")).unwrap();
    let exec = section.get("Exec").unwrap();

    let splitted = exec
        .split_whitespace()
        .map(|a| a.to_string())
        .collect::<Vec<String>>();

    if splitted.is_empty() {
        panic!("No command specified!");
    }

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

    let argv = argv_data
        .iter()
        .map(|e| e.as_ptr())
        .chain(std::iter::once(std::ptr::null()))
        .collect::<Vec<*const libc::c_char>>();

    let mut envp_data: Vec<CString> = vec![];
    for (idx, (key, value)) in env::vars().enumerate() {
        let env_var = format!("{}={}", key, value);
        println!("envp[{idx}]: {env_var}");
        let c_string = CString::new(env_var)?;
        envp_data.push(c_string);
    }

    let envp = envp_data
        .iter()
        .map(|e| e.as_ptr())
        .chain(std::iter::once(std::ptr::null()))
        .collect::<Vec<*const libc::c_char>>();

    let execve_err = unsafe { libc::execve(prog.as_ptr(), argv.as_ptr(), envp.as_ptr()) };

    if execve_err == -1 {
        return Err(format!("execve failed: {}", std::io::Error::last_os_error()).into());
    }

    unreachable!()
}
