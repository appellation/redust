use thiserror::Error;

use crate::Data;

/// A RESP data type.
#[derive(Debug, Clone)]
pub enum DataType {
	SimpleString,
	BulkString,
	Array,
	Integer,
}

/// An error that can occur when transforming [Data].
#[derive(Debug, Error, Clone)]
#[error("Expected {expected:?} but got {found:?}")]
pub struct Error<'a> {
	/// The data type which was expected.
	pub expected: DataType,
	/// The data which was found.
	pub found: Data<'a>,
}

impl<'a> Error<'a> {
	pub fn into_owned(self) -> Error<'static> {
		Error {
			expected: self.expected,
			found: self.found.into_owned(),
		}
	}
}

pub type Result<'a, T, E = Error<'a>> = std::result::Result<T, E>;
