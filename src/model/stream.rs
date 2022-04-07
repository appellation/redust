use std::{
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
#[serde(into = "String", try_from = "&str")]
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

impl<'a> TryFrom<&'a str> for Id {
	type Error = <Id as FromStr>::Err;

	fn try_from(value: &'a str) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl FromStr for Id {
	type Err = Error<String>;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let (_, (a, b)) =
			separated_pair(u64, char('-'), u64)(s).map_err(|e: Err<Error<&str>>| match e {
				Err::Error(e) | Err::Failure(e) => Error {
					input: e.input.to_string(),
					code: e.code,
				},
				_ => unreachable!(),
			})?;
		Ok(Self(a, b))
	}
}

impl Display for Id {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}-{}", self.0, self.1)
	}
}
