use serde::ser;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
	#[error("{0}")]
	Message(String),
	#[error("io error: {0}")]
	Io(#[from] std::io::Error),
	#[error("sequence length required")]
	LengthRequired,
}

impl ser::Error for Error {
	fn custom<T>(msg: T) -> Self
	where
		T: std::fmt::Display,
	{
		Self::Message(msg.to_string())
	}
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
