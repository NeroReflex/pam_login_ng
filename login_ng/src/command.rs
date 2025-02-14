#[derive(Debug, Clone, PartialEq)]
pub struct SessionCommand {
    command: String,
    args: Vec<String>,
}

impl SessionCommand {
    pub fn new(command: String, args: Vec<String>) -> Self {
        Self { command, args }
    }

    pub fn command(&self) -> String {
        self.command.clone()
    }

    pub fn args(&self) -> Vec<String> {
        self.args.clone()
    }
}
