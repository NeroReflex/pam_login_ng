use std::error::Error;

pub trait Runner {
    fn run(&mut self) -> Result<(), Box<dyn Error>>;
}