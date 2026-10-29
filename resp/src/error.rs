use std::{borrow::Cow, str::Utf8Error};

use serde::{de, ser};
use thiserror::Error;

/// The amount of additional data needed to finish parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Needed {
	/// The amount of data needed is unknown.
	Unknown,
	/// At least this many more bytes are needed.
	Size(usize),
}

/// An error encountered while parsing RESP.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
	/// The data ended before a complete value was parsed.
	#[error("incomplete data")]
	Incomplete(Needed),
	/// A byte other than the expected type marker was found.
	#[error("expected {:?}, found {:?}", char::from(*.expected), char::from(*.found))]
	Unexpected {
		/// The expected byte.
		expected: u8,
		/// The byte which was found.
		found: u8,
	},
	/// A value started with an unknown type marker.
	#[error("unknown type {:?}", char::from(*.0))]
	Type(u8),
	/// A line was not terminated by CRLF.
	#[error("expected CRLF")]
	Crlf,
	/// An integer was missing, malformed, or out of range.
	#[error("invalid integer")]
	Integer,
	/// A bulk string had an invalid length.
	#[error("invalid length {0}")]
	Length(i64),
	/// A string was not valid UTF-8.
	#[error("invalid UTF-8: {0}")]
	Utf8(#[from] Utf8Error),
}

/// Errors that can be encountered when interacting with RESP.
#[derive(Debug, Error)]
pub enum Error<'a> {
	/// Serialization error.
	#[error("{0}")]
	Message(Cow<'a, str>),
	/// An IO error occured when writing to the buffer.
	#[error("io error: {0}")]
	Io(#[from] std::io::Error),
	/// Invalid RESP syntax.
	#[error("parse error: {0}")]
	Parse(#[from] ParseError),
	/// An error was indicated by the data.
	#[error("Redis error: {0}")]
	Redis(Cow<'a, str>),
}

impl Error<'_> {
	/// Convert this error into an owned error.
	pub fn into_owned(self) -> Error<'static> {
		match self {
			Self::Message(msg) => Error::Message(msg.into_owned().into()),
			Self::Io(err) => Error::Io(err),
			Self::Parse(err) => Error::Parse(err),
			Self::Redis(msg) => Error::Redis(msg.into_owned().into()),
		}
	}

	/// Whether this error is transient (i.e. the source is still valid)
	pub fn is_transient(&self) -> bool {
		matches!(self, Self::Redis(_))
	}
}

impl ser::Error for Error<'_> {
	fn custom<T>(msg: T) -> Self
	where
		T: std::fmt::Display,
	{
		Self::Message(msg.to_string().into())
	}
}

impl de::Error for Error<'_> {
	fn custom<T>(msg: T) -> Self
	where
		T: std::fmt::Display,
	{
		Self::Message(msg.to_string().into())
	}
}

/// Result with an error type defaulting to [enum@Error].
pub type Result<'a, T, E = Error<'a>> = std::result::Result<T, E>;
