use std::{
	borrow::Cow,
	fmt::{Debug, Display},
};

use serde::de;

use crate::Error;

/// An error occurred while reading bytes.
pub struct ReadError<'a> {
	/// The error which occurred.
	pub data: Error<'a>,
	/// Bytes remaining to be read.
	pub remaining: Cow<'a, [u8]>,
}

impl ReadError<'_> {
	/// Convert this error into an owned error.
	pub fn into_owned(self) -> ReadError<'static> {
		ReadError {
			data: self.data.into_owned(),
			remaining: self.remaining.into_owned().into(),
		}
	}
}

impl Debug for ReadError<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ReadError")
			.field("data", &self.data)
			.field("remaining", &String::from_utf8_lossy(&self.remaining))
			.finish()
	}
}

impl Display for ReadError<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.data)
	}
}

impl std::error::Error for ReadError<'_> {
	fn cause(&self) -> Option<&dyn de::StdError> {
		Some(&self.data)
	}
}
