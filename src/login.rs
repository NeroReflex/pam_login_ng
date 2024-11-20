use std::sync::{Arc, Mutex};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LoginResult {
    Success,
    Failure,
}

pub trait LoginUserInteractionHandler {

    fn provide_username(&mut self, username: &String);

    fn prompt_secret(&mut self, msg: &String) -> Option<String>;

    fn prompt_plain(&mut self, msg: &String) -> Option<String>;

    fn print_info(&mut self, msg: &String);

    fn print_error(&mut self, msg: &String);

}

pub trait LoginExecutor {

    fn prompt(&self) -> Arc<Mutex<dyn LoginUserInteractionHandler>>;

    fn execute(&mut self, maybe_username: &Option<String>, cmd: &std::string::String) -> Result<LoginResult, Box<dyn std::error::Error>>;

}
