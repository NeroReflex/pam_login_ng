use std::sync::{Arc, Mutex};

use thiserror::Error;

use crate::{greetd::GreetdLoginError, pam::PamLoginError};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LoginResult {
    Success,
    Failure,
}

#[derive(Debug, Error)]
pub enum LoginError {
    #[error("Error with greetd: {0}")]
    GreetdError(GreetdLoginError),

    #[error("Error with pam: {0}")]
    PamError(PamLoginError),

    #[error("Error in username discovery")]
    UserDiscoveryError,
}

pub trait LoginUserInteractionHandler {

    fn provide_username(&mut self, username: &String);

    fn prompt_secret(&mut self, msg: &String) -> Option<String>;

    fn prompt_plain(&mut self, msg: &String) -> Option<String>;

    fn print_info(&mut self, msg: &String);

    fn print_error(&mut self, msg: &String);

}

/// Interface that allows a user to authenticate and perform actions
pub trait LoginExecutor {

    fn prompt(&self) -> Arc<Mutex<dyn LoginUserInteractionHandler>>;

    /// Authenticate the user and execute the given command, or launch shell if one is not being provided.
    fn execute(&mut self, maybe_username: &Option<String>, cmd: &Option<String>) -> Result<LoginResult, LoginError>;

}
