use std::ffi::{CStr, CString};

use crate::{prompt_stderr, prompt_password};

use pam_client2::{ConversationHandler, ErrorCode};

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum ConversationInteraction {
    EchoOn { prompt: String, response: String },
    EchoOff { prompt: String, response: String }
}

pub trait ConversationRecorder {
    fn record_echo_on(&mut self, prompt: String, response: String);

    fn record_echo_off(&mut self, prompt: String, response: String);

    fn recorded_username(&self, user_prompt: &Option<&str>,) -> Option<String>;

    fn recorded_password(&self) -> Option<String>;
}

pub struct SimpleConversationRecorder {
    recording: Vec<ConversationInteraction>,
}

impl SimpleConversationRecorder {
    #[must_use]
	pub fn new() -> Self {
		Self {
            recording: vec![]
        }
	}
}

impl ConversationRecorder for SimpleConversationRecorder {
    fn record_echo_on(&mut self, prompt: String, response: String) {
        self.recording.push(ConversationInteraction::EchoOn { prompt: prompt, response: response } );
    }

    fn record_echo_off(&mut self, prompt: String, response: String) {
        self.recording.push(ConversationInteraction::EchoOff { prompt: prompt, response: response } );
    }

    fn recorded_username(&self, user_prompt: &Option<&str>,) -> Option<String> {
        for r in self.recording.iter().rev() {
            if let ConversationInteraction::EchoOn { prompt, response } = r {
                match user_prompt {
                    Some(expected_prompt) => if expected_prompt == prompt { return Some(response.clone()) },
                    None => if prompt.contains("login:") { return Some(response.clone()) },
                }
            }
        }

        None
    }

    fn recorded_password(&self) -> Option<String> {
        for r in self.recording.iter().rev() {
            if let ConversationInteraction::EchoOff { prompt: _, response } = r {
                return Some(response.clone())
            }
        }

        None
    }
}

pub trait ConversationPromptAnswerer {
    
    fn echo_on_prompt(&mut self, prompt: &String) -> Option<String>;

    fn echo_off_prompt(&mut self, prompt: &String) -> Option<String>;

}

pub struct SimpleConversationPromptAnswerer {
    plain: Option<String>,
    hidden: Option<String>
}

impl SimpleConversationPromptAnswerer {
    pub fn new(
        plain: Option<String>,
        hidden: Option<String>
    ) -> Self {
        Self {
            plain,
            hidden
        }
    }
}

impl ConversationPromptAnswerer for SimpleConversationPromptAnswerer {

    fn echo_on_prompt(&mut self, _prompt: &String) -> Option<String> {
        self.plain.clone()
    }

    fn echo_off_prompt(&mut self, _prompt: &String) -> Option<String> {
        self.hidden.clone()
    }

}

pub struct Conversation {
    answerer: Option<Arc<Mutex<dyn ConversationPromptAnswerer>>>,
    recorder: Option<Arc<Mutex<dyn ConversationRecorder>>>,
}

impl Conversation {
    /// Creates a new null conversation handler
	#[must_use]
	pub fn new(
        answerer: Option<Arc<Mutex<dyn ConversationPromptAnswerer>>>,
        recorder: Option<Arc<Mutex<dyn ConversationRecorder>>>
    ) -> Self {
		Self {
            answerer,
            recorder
        }
	}

    pub fn attach_recorder(&mut self, recorder: Arc<Mutex<dyn ConversationRecorder>>) {
        self.recorder = Some(recorder)
    }
}

impl Default for Conversation {
	fn default() -> Self {
		Self::new(None, None)
	}
}

impl ConversationHandler for Conversation {
	fn prompt_echo_on(&mut self, msg: &CStr) -> Result<CString, ErrorCode> {
        let prompt = format!("{}", msg.to_string_lossy());

		let response: String = match self.answerer {
            Some(ref ans) => match ans.lock() {
                Ok(mut guard) => match guard.echo_on_prompt(&prompt) {
                    Some(answer) => answer,
                    None => prompt_stderr(prompt.as_str()).map_err(|_err| ErrorCode::CONV_ERR)?
                },
                Err(_) => prompt_stderr(prompt.as_str()).map_err(|_err| ErrorCode::CONV_ERR)?
            },
            None => prompt_stderr(prompt.as_str()).map_err(|_err| ErrorCode::CONV_ERR)?
        };

        if let Some(recorder) = &self.recorder {
            if let Ok(mut guard) = recorder.lock() {
                guard.record_echo_on(prompt, response.clone());
            }
        }

        Ok(CString::new(response).map_err(|_err| ErrorCode::CONV_ERR)?)
	}

	fn prompt_echo_off(&mut self, msg: &CStr) -> Result<CString, ErrorCode> {
		let prompt = format!("{}", msg.to_string_lossy());

        let response: String = match self.answerer {
            Some(ref ans) => match ans.lock() {
                Ok(mut guard) => match guard.echo_off_prompt(&prompt) {
                    Some(answer) => answer,
                    None => prompt_password(prompt.as_str()).map_err(|_err| ErrorCode::CONV_ERR)?
                },
                Err(_) => prompt_password(prompt.as_str()).map_err(|_err| ErrorCode::CONV_ERR)?
            },
            None => prompt_password(prompt.as_str()).map_err(|_err| ErrorCode::CONV_ERR)?
        };

        if let Some(recorder) = &self.recorder {
            if let Ok(mut guard) = recorder.lock() {
                guard.record_echo_off(prompt, response.clone());
            }
        }

        Ok(CString::new(response).map_err(|_err| ErrorCode::CONV_ERR)?)
	}

	fn text_info(&mut self, msg: &CStr) {
        let string = format!("{}", msg.to_string_lossy());

        println!("{}", string);
    }

	fn error_msg(&mut self, msg: &CStr) {
        let string = format!("{}", msg.to_string_lossy());

        eprintln!("{}", string);
    }
}
