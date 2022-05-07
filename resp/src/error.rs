use std::borrow::Cow;

use serde::{de, ser};
use thiserror::Error;

use crate::parser;

type NomError<T> = nom::error::Error<T>;

fn transform_parse_err<T, I>(
	err: nom::Err<NomError<I>>,
	map_input: impl FnOnce(I) -> T,
) -> nom::Err<NomError<T>> {
	let make_err = |e: NomError<I>| NomError {
		input: map_input(e.input),
		code: e.code,
	};

	match err {
		nom::Err::Error(e) => nom::Err::Error(make_err(e)),
		nom::Err::Failure(e) => nom::Err::Failure(make_err(e)),
		nom::Err::Incomplete(needed) => nom::Err::Incomplete(needed),
	}
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
	Parse(parser::Error<'a>),
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
			Self::Parse(err) => Error::Parse(transform_parse_err(err, |i| i.into_owned().into())),
			Self::Redis(msg) => Error::Redis(msg.into_owned().into()),
		}
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

impl<'a> From<parser::Error<'a>> for Error<'a> {
	fn from(err: parser::Error<'a>) -> Self {
		Self::Parse(err)
	}
}

impl<'a> From<parser::RawError<'a>> for Error<'a> {
	fn from(err: parser::RawError<'a>) -> Self {
		Self::Parse(transform_parse_err(err, |i| i.into()))
	}
}

/// Result with an error type defaulting to [enum@Error].
pub type Result<'a, T, E = Error<'a>> = std::result::Result<T, E>;
