use std::{string::String, env};

use std::os::unix::net::UnixStream;

use greetd_ipc::{codec::SyncCodec, AuthMessageType, ErrorType, Request, Response};

pub enum LoginResult {
    Success,
    Failure,
}

pub struct Login<T> {
    username: String,
    cmd: String,

    prompt: Box<dyn Fn(&String, T) -> Result<String, Box<dyn std::error::Error>>>,
}

impl<T> Login<T>
where
    T: Clone {

    pub fn new(
        username: String,
        cmd: String,
        data_prompt: impl Fn(&String, T) -> Result<String, Box<dyn std::error::Error>> + 'static
    ) -> Self {
        Self {
            username,
            cmd,
            prompt: Box::new(data_prompt)
        }
    }

    pub fn execute(
        &self,
        param: T
    ) -> Result<LoginResult, Box<dyn std::error::Error>> {
        let mut stream = UnixStream::connect(env::var("GREETD_SOCK")?)?;
    
        let username = self.username.clone();
    
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
                        AuthMessageType::Visible => Some((self.prompt)(&auth_message, param.clone())?),
                        AuthMessageType::Secret => Some((self.prompt)(&auth_message, param.clone())?),
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
                            cmd: vec![self.cmd.clone().to_string()],
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
