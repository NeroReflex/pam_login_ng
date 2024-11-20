use crate::login::*;

use std::os::unix::net::UnixStream;

use greetd_ipc::{codec::SyncCodec, AuthMessageType, ErrorType, Request, Response};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GreetdLoginError {
    #[error("No username provided")]
    NoUsernameProvided,

    #[error("Mutex error")]
    MutexError,
}

pub struct GreetdLoginExecutor {
    
    greetd_sock: String,

    prompter: std::sync::Arc<std::sync::Mutex<dyn crate::login::LoginUserInteractionHandler>>,

}

impl GreetdLoginExecutor {

    pub fn new(
        greetd_sock: String,
        prompter: std::sync::Arc<std::sync::Mutex<dyn crate::login::LoginUserInteractionHandler>>
    ) -> Self {
        Self {
            greetd_sock,
            prompter
        }
    }

}

impl LoginExecutor for GreetdLoginExecutor {
    fn prompt(&self) -> std::sync::Arc<std::sync::Mutex<dyn crate::login::LoginUserInteractionHandler>> {
        self.prompter.clone()
    }

    fn execute(&mut self, maybe_username: &Option<String>, cmd: &String) -> Result<LoginResult, Box<dyn std::error::Error>> {
        let mut stream = UnixStream::connect(&self.greetd_sock)?;
    
        let mutexed_prompter = self.prompt();

        let mut prompter = mutexed_prompter.lock().map_err(|_| GreetdLoginError::MutexError)?;

        let username = match maybe_username {
            Some(username) => username.clone(),
            None => prompter.prompt_plain(&String::from("login: ")).ok_or(GreetdLoginError::NoUsernameProvided)?
        };

        prompter.provide_username(&username);

        let mut next_request = Request::CreateSession { username };
        let mut starting = false;
        loop {
            next_request.write_to(&mut stream)?;
    
            match Response::read_from(&mut stream)? {
                Response::AuthMessage {
                    auth_message,
                    auth_message_type,
                } => {
                    let response = match auth_message_type {
                        AuthMessageType::Visible => prompter.prompt_plain(&auth_message),
                        AuthMessageType::Secret => prompter.prompt_secret(&auth_message),
                        AuthMessageType::Info => {
                            eprintln!("info: {}", auth_message);
                            None
                        }
                        AuthMessageType::Error => {
                            eprintln!("error: {}", auth_message);
                            None
                        }
                    };
    
                    next_request = Request::PostAuthMessageResponse { response };
                }
                Response::Success => {
                    if starting {
                        return Ok(LoginResult::Success);
                    } else {
                        starting = true;

                        next_request = Request::StartSession {
                            env: vec![],
                            cmd: vec![cmd.to_string()],
                        }
                    }
                }
                Response::Error {
                    error_type,
                    description,
                } => {
                    Request::CancelSession.write_to(&mut stream)?;
                    match error_type {
                        ErrorType::AuthError => return Ok(LoginResult::Failure),
                        ErrorType::Error => {
                            return Err(format!("login error: {:?}", description).into())
                        }
                    }
                }
            }
        }
    }
}
