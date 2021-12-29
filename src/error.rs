use std::error::Error;
use std::fmt;
use std::num::TryFromIntError;

#[derive(Debug)]
pub struct CommandError {
	details: String,
}

impl CommandError {
	pub fn new(msg: &str) -> CommandError {
		CommandError{details: msg.to_string()}
	}
}

impl fmt::Display for CommandError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"{}",self.details)
	}
}

impl Error for CommandError {
	fn description(&self) -> &str {
		&self.details
	}
}

impl From<TryFromIntError> for CommandError {
	fn from(err: TryFromIntError) -> Self {
		CommandError::new(&err.to_string())
	}
}
