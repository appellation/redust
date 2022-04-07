use std::{
	char::ParseCharError,
	num::{ParseFloatError, TryFromIntError},
	str::ParseBoolError,
};

use serde::de;
use thiserror::Error;

use crate::parser;

#[derive(Debug, Error, PartialEq)]
pub enum Error<'a> {
	#[error("{0}")]
	Message(String),
	/// An error occurred while parsing RESP.
	#[error("parse error")]
	ParseError(parser::Error<'a>),
	/// An error was indicated by the data.
	#[error("Redis error")]
	RedisError(&'a str),
	/// Expected a boolean but got something else.
	#[error("invalid bool format")]
	InvalidBool(#[from] ParseBoolError),
	/// Expected an integer but got something else.
	#[error("invalid integer type")]
	InvalidInt(#[from] TryFromIntError),
	/// Expected a float (in string format) but got something else.
	#[error("invalid float format")]
	InvalidFloat(#[from] ParseFloatError),
	/// Expected a character but got something else.
	#[error("invalid char format")]
	InvalidChar(#[from] ParseCharError),
}

impl<'a> From<parser::Error<'a>> for Error<'a> {
	fn from(err: parser::Error<'a>) -> Self {
		Self::ParseError(err)
	}
}

impl de::Error for Error<'_> {
	fn custom<T>(msg: T) -> Self
	where
		T: std::fmt::Display,
	{
		Self::Message(msg.to_string())
	}
}

pub type Result<'a, T, E = Error<'a>> = std::result::Result<T, E>;
