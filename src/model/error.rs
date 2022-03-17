use std::{num::ParseIntError, str::Utf8Error};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
	#[error("{0}")]
	InvalidData(resp::error::Error<'static>),
	#[error("Invalid data format: {0}")]
	InvalidFormat(&'static str),
	#[error("Missing element {0} in array response")]
	MissingElement(usize),
	#[error("Expected integer")]
	NotInteger(#[from] ParseIntError),
	#[error("Expected string")]
	NotString(#[from] Utf8Error),
}

impl<'a> From<resp::error::Error<'a>> for Error {
	fn from(err: resp::error::Error<'a>) -> Self {
		Self::InvalidData(err.into_owned())
	}
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
