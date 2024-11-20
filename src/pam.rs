use std::{os::unix::process::CommandExt, process::Command, sync::{Arc, Mutex}};

use pam_client2::{Context, Flag};
use thiserror::Error;

use crate::{conversation::ProxyLoginUserInteractionHandlerConversation, login::*};

use users::{get_user_by_name, User};

#[derive(Debug, Error)]
pub enum PamLoginError {
    #[error("Runtime error setting login prompt")]
    SetPrompt,

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Validation error: ")]
    Validation(String),

    #[error("Command execution error: ")]
    Execution(String),

    #[error("Unable to find the user id (unknown uid)")]
    UnknownUid,

}


pub struct PamLoginExecutor {
    conversation: ProxyLoginUserInteractionHandlerConversation
}

impl PamLoginExecutor {
    pub fn new (conversation: ProxyLoginUserInteractionHandlerConversation) -> Self {
        Self {
            conversation
        }
    }
}

impl LoginExecutor for PamLoginExecutor {

    fn prompt(&self) -> Arc<Mutex<dyn crate::login::LoginUserInteractionHandler>> {
        //Arc::new(Mutex::new(self.conversation.clone()))
        todo!()
    }

    fn execute(&mut self, maybe_username: &Option<String>, cmd: &String) -> Result<LoginResult, Box<dyn std::error::Error>> {

        let user_prompt = Some("username: ");

        let mut context = Context::new(
            "system-login",
            maybe_username.as_ref().map(|a| a.as_str()),
            self.conversation.clone()
        ).expect("Failed to initialize PAM context");
    
        context.set_user_prompt(user_prompt).map_err(|_err| PamLoginError::SetPrompt)?;
    
        // Authenticate the user (ask for password, 2nd-factor token, fingerprint, etc.)
        context.authenticate(Flag::NONE).map_err(|err| PamLoginError::Authentication(err.to_string()))?;
    
        // Validate the account (is not locked, expired, etc.)
        context.acct_mgmt(Flag::NONE).map_err(|err| PamLoginError::Validation(err.to_string()))?;
        
        // Get resulting user name and map to a user id
        let username = context.user()?;
        let logged_user = get_user_by_name(&username).ok_or(PamLoginError::UnknownUid)?;

        // Open session and initialize credentials
        let session = context.open_session(Flag::NONE).expect("Session opening failed");

        // Run a process in the PAM environment
        let _result = Command::new(cmd)
            .env_clear()
            .envs(session.envlist().iter_tuples())
            .uid(logged_user.uid())
            //.groups(logged_user.groups().unwrap_or(vec![]).iter().map(|g| g.gid()).collect::<Vec<u32>>().as_slice())
            .status()
            .map_err(|err| PamLoginError::Execution(err.to_string()))?;

        Ok(LoginResult::Success)
    }
    
}
