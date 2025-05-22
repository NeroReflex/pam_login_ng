use ini::Ini;
use std::error::Error;
use std::{env, ffi::CString, path::PathBuf, ptr::null};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!("No session specified")
    }

    let param = args[1].to_string();

    let path = PathBuf::from("/usr/share/wayland-sessions/").join(param);

    let conf = Ini::load_from_file(path).unwrap();

    let section = conf.section(Some("Desktop Entry")).unwrap();
    let exec = section.get("Exec").unwrap();

    let splitted = exec
        .split_whitespace()
        .into_iter()
        .map(|a| a.to_string())
        .collect::<Vec<String>>();

    if splitted.is_empty() {
        panic!("No command specified!");
    }

    let mut argv: Vec<*const libc::c_char> = vec![];
    let mut prog = CString::new("false").unwrap();

    for (idx, val) in splitted.iter().enumerate() {
        let c_string = CString::new(val.as_str()).expect("CString::new failed");
        if idx == 0 {
            prog = c_string;
        } else {
            argv.push(c_string.as_ptr());
        }
    }
    argv.push(null());

    let mut envp: Vec<*const libc::c_char> = vec![];
    for (key, value) in env::vars() {
        let env_var = format!("{}={}", key, value);
        let c_string = CString::new(env_var)?;
        envp.push(c_string.as_ptr());
    }
    envp.push(null());

    let execve_err = unsafe { libc::execve(prog.as_ptr(), argv.as_ptr(), envp.as_ptr()) };

    if execve_err == -1 {
        return Err(format!("execve failed: {}", std::io::Error::last_os_error()).into());
    }

    Ok(())
}
