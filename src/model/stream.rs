use std::{
	borrow::Cow,
	fmt::{Display, Formatter},
	str::FromStr,
};

use resp::nom::{
	character::complete::{char, u64},
	error::Error,
	sequence::separated_pair,
	Err,
};
use serde::{Deserialize, Serialize};

/// Models for XAUTOCLAIM commands.
pub mod claim;
/// Models for XREAD(GROUP) commands.
pub mod read;

/// A [stream ID](https://redis.io/topics/streams-intro#entry-ids).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "Vec<u8>", try_from = "&[u8]")]
pub struct Id(
	/// The timestamp, in milliseconds
	pub u64,
	/// The sequence number
	pub u64,
);

impl From<Id> for String {
	fn from(id: Id) -> Self {
		id.to_string()
	}
}

impl From<Id> for Vec<u8> {
	fn from(id: Id) -> Self {
		id.to_string().into_bytes()
	}
}

impl<'a> TryFrom<&'a [u8]> for Id {
	type Error = Error<Cow<'a, str>>;

	fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
		let (_, (a, b)) =
			separated_pair(u64, char('-'), u64)(value).map_err(|e: Err<Error<&[u8]>>| match e {
				Err::Error(e) | Err::Failure(e) => Error {
					input: String::from_utf8_lossy(e.input),
					code: e.code,
				},
				_ => unreachable!(),
			})?;
		Ok(Self(a, b))
	}
}

impl<'a> TryFrom<&'a str> for Id {
	type Error = Error<Cow<'a, str>>;

	fn try_from(value: &'a str) -> Result<Self, Self::Error> {
		value.as_bytes().try_into()
	}
}

impl FromStr for Id {
	type Err = Error<String>;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		s.try_into().map_err(|e: Error<Cow<'_, str>>| Error {
			input: e.input.into_owned(),
			code: e.code,
		})
	}
}

impl Display for Id {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}-{}", self.0, self.1)
	}
}
