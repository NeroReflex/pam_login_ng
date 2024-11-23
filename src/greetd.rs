use crate::login::*;

use std::{
    os::unix::net::UnixStream,
    sync::{Arc, Mutex},
};

use greetd_ipc::{codec::SyncCodec, AuthMessageType, ErrorType, Request, Response};

use thiserror::Error;
use users::{get_user_by_name, os::unix::UserExt};

#[derive(Debug, Error)]
pub enum GreetdLoginError {
    #[error("Error connecting to greetd: {0}")]
    GreetdConnectionError(#[from] std::io::Error),

    #[error("Error in greetd connection: {0}")]
    GreetdIpcError(#[from] greetd_ipc::codec::Error),

    #[error("Unknown error in greetd: {0}")]
    GreetdUnknownError(String),

    #[error("No username provided")]
    NoUsernameProvided,

    #[error("Mutex error")]
    MutexError,
}

pub struct GreetdLoginExecutor {
    greetd_sock: String,

    prompter: Arc<Mutex<dyn crate::login::LoginUserInteractionHandler>>,
}

impl GreetdLoginExecutor {
    pub fn new(
        greetd_sock: String,
        prompter: Arc<Mutex<dyn crate::login::LoginUserInteractionHandler>>,
    ) -> Self {
        Self {
            greetd_sock,
            prompter,
        }
    }
}

impl LoginExecutor for GreetdLoginExecutor {
    fn prompt(&self) -> Arc<Mutex<dyn crate::login::LoginUserInteractionHandler>> {
        self.prompter.clone()
    }

    fn execute(
        &mut self,
        maybe_username: &Option<String>,
        cmd: &Option<String>,
    ) -> Result<LoginResult, LoginError> {
        let mut stream = UnixStream::connect(&self.greetd_sock)
            .map_err(|err| LoginError::GreetdError(GreetdLoginError::GreetdConnectionError(err)))?;

        let mutexed_prompter = self.prompt();

        let mut prompter = mutexed_prompter
            .lock()
            .map_err(|_| LoginError::GreetdError(GreetdLoginError::MutexError))?;

        let username =
            match maybe_username {
                Some(username) => username.clone(),
                None => prompter.prompt_plain(&String::from("login: ")).ok_or(
                    LoginError::GreetdError(GreetdLoginError::NoUsernameProvided),
                )?,
            };

        prompter.provide_username(&username);

        let mut next_request = Request::CreateSession {
            username: username.clone(),
        };
        let mut starting = false;
        loop {
            next_request
                .write_to(&mut stream)
                .map_err(|err| LoginError::GreetdError(GreetdLoginError::GreetdIpcError(err)))?;

            match Response::read_from(&mut stream)
                .map_err(|err| LoginError::GreetdError(GreetdLoginError::GreetdIpcError(err)))?
            {
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

                        let logged_user =
                            get_user_by_name(&username).ok_or(LoginError::UserDiscoveryError)?;

                        let command = match &cmd {
                            Some(cmd) => cmd.clone(),
                            None => format!(
                                "{}",
                                logged_user
                                    .shell()
                                    .to_str()
                                    .map_or(String::from(crate::DEFAULT_CMD), |shell| shell
                                        .to_string())
                            ),
                        };

                        next_request = Request::StartSession {
                            env: vec![],
                            cmd: vec![command],
                        }
                    }
                }
                Response::Error {
                    error_type,
                    description,
                } => {
                    Request::CancelSession
                        .write_to(&mut stream)
                        .map_err(|err| {
                            LoginError::GreetdError(GreetdLoginError::GreetdIpcError(err))
                        })?;
                    match error_type {
                        ErrorType::AuthError => return Ok(LoginResult::Failure),
                        ErrorType::Error => {
                            return Err(LoginError::GreetdError(
                                GreetdLoginError::GreetdUnknownError(description),
                            ))
                        }
                    }
                }
            }
        }
    }
}
