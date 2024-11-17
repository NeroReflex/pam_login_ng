use std::{
    env,
    os::unix::process::CommandExt,
};

use getopts::Options;

use std::process::Command;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optopt("c", "cmd", format!("command to run, defaults to {}", login_ng::DEFAULT_CMD).as_str(), "COMMAND");
    opts.optopt(
        "f",
        "max-failures",
        "maximum number of accepted failures",
        "FAILURES",
    );
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            println!("{}", f);
            print_usage(&program, opts);
            std::process::exit(1);
        }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        std::process::exit(0);
    }

    let _cmd_res = Command::new("sh")
        .arg("-c")
        .arg("echo hello")
        .exec();

    //if cmd_res == io::Error::
}