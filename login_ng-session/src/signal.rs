use std::fmt;

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum SignalParseError {
    #[error("Invalid signal name: {0}")]
    Invalid(String),
}

#[repr(i32)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signal {
    SIGHUP = libc::SIGHUP,
    SIGINT = libc::SIGINT,
    SIGQUIT = libc::SIGQUIT,
    SIGILL = libc::SIGILL,
    SIGTRAP = libc::SIGTRAP,
    SIGABRT = libc::SIGABRT,
    SIGBUS = libc::SIGBUS,
    SIGFPE = libc::SIGFPE,
    SIGKILL = libc::SIGKILL,
    SIGUSR1 = libc::SIGUSR1,
    SIGSEGV = libc::SIGSEGV,
    SIGUSR2 = libc::SIGUSR2,
    SIGPIPE = libc::SIGPIPE,
    SIGALRM = libc::SIGALRM,
    SIGTERM = libc::SIGTERM,
    SIGCHLD = libc::SIGCHLD,
    SIGCONT = libc::SIGCONT,
    SIGSTOP = libc::SIGSTOP,
    SIGTSTP = libc::SIGTSTP,
    SIGTTIN = libc::SIGTTIN,
    SIGTTOU = libc::SIGTTOU,
    SIGURG = libc::SIGURG,
    SIGVTALRM = libc::SIGVTALRM,
    SIGXCPU = libc::SIGXCPU,
    SIGXFSZ = libc::SIGXFSZ,
}

impl Signal {
    pub fn send_to(&self, pid: i32) -> Result<(), i32> {
        let res = unsafe { libc::kill(pid, *self as i32) };

        if res != 0 {
            return Err(unsafe { *libc::__errno_location() });
        }

        Ok(())
    }
}

impl TryFrom<&str> for Signal {
    type Error = SignalParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "SIGHUP" => Ok(Signal::SIGHUP),
            "SIGINT" => Ok(Signal::SIGINT),
            "SIGQUIT" => Ok(Signal::SIGQUIT),
            "SIGILL" => Ok(Signal::SIGILL),
            "SIGTRAP" => Ok(Signal::SIGTRAP),
            "SIGABRT" => Ok(Signal::SIGABRT),
            "SIGBUS" => Ok(Signal::SIGBUS),
            "SIGFPE" => Ok(Signal::SIGFPE),
            "SIGKILL" => Ok(Signal::SIGKILL),
            "SIGUSR1" => Ok(Signal::SIGUSR1),
            "SIGSEGV" => Ok(Signal::SIGSEGV),
            "SIGUSR2" => Ok(Signal::SIGUSR2),
            "SIGPIPE" => Ok(Signal::SIGPIPE),
            "SIGALRM" => Ok(Signal::SIGALRM),
            "SIGTERM" => Ok(Signal::SIGTERM),
            "SIGCHLD" => Ok(Signal::SIGCHLD),
            "SIGCONT" => Ok(Signal::SIGCONT),
            "SIGSTOP" => Ok(Signal::SIGSTOP),
            "SIGTSTP" => Ok(Signal::SIGTSTP),
            "SIGTTIN" => Ok(Signal::SIGTTIN),
            "SIGTTOU" => Ok(Signal::SIGTTOU),
            "SIGURG" => Ok(Signal::SIGURG),
            "SIGVTALRM" => Ok(Signal::SIGVTALRM),
            "SIGXCPU" => Ok(Signal::SIGXCPU),
            "SIGXFSZ" => Ok(Signal::SIGXFSZ),
            _ => Err(SignalParseError::Invalid(String::from(value))),
        }
    }
}

// Implement Display for pretty printing
impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Signal::SIGHUP => "SIGHUP",
                Signal::SIGINT => "SIGINT",
                Signal::SIGQUIT => "SIGQUIT",
                Signal::SIGILL => "SIGILL",
                Signal::SIGTRAP => "SIGTRAP",
                Signal::SIGABRT => "SIGABRT",
                Signal::SIGBUS => "SIGBUS",
                Signal::SIGFPE => "SIGFPE",
                Signal::SIGKILL => "SIGKILL",
                Signal::SIGUSR1 => "SIGUSR1",
                Signal::SIGSEGV => "SIGSEGV",
                Signal::SIGUSR2 => "SIGUSR2",
                Signal::SIGPIPE => "SIGPIPE",
                Signal::SIGALRM => "SIGALRM",
                Signal::SIGTERM => "SIGTERM",
                Signal::SIGCHLD => "SIGCHLD",
                Signal::SIGCONT => "SIGCONT",
                Signal::SIGSTOP => "SIGSTOP",
                Signal::SIGTSTP => "SIGTSTP",
                Signal::SIGTTIN => "SIGTTIN",
                Signal::SIGTTOU => "SIGTTOU",
                Signal::SIGURG => "SIGURG",
                Signal::SIGVTALRM => "SIGVTALRM",
                Signal::SIGXCPU => "SIGXCPU",
                Signal::SIGXFSZ => "SIGXFSZ",
            }
        )
    }
}
