use std::{
	fmt::{Display, Formatter},
	str::from_utf8,
};

use resp::{error::DataType, Data};

use super::error::{Error, Result};

/// Models for XAUTOCLAIM commands.
pub mod claim;
/// Models for XREAD(GROUP) commands.
pub mod read;

/// A [stream ID](https://redis.io/topics/streams-intro#entry-ids).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Id(
	/// The timestamp, in milliseconds
	pub u64,
	/// The sequence number
	pub u64,
);

impl<'a> TryFrom<Data<'a>> for Id {
	type Error = Error;

	fn try_from(value: Data<'a>) -> Result<Self, Self::Error> {
		match value {
			Data::SimpleString(str) => Self::parse(&str),
			Data::BulkString(Some(str)) => Self::parse(from_utf8(&str)?),
			_ => Err(Error::InvalidData(resp::error::Error {
				expected: DataType::SimpleString,
				found: value.into_owned(),
			})),
		}
	}
}

impl Id {
	pub fn parse(input: &str) -> Result<Self> {
		let (a, b) = input
			.split_once('-')
			.ok_or(Error::InvalidFormat("missing - separator"))?;

		Ok(Self(a.parse()?, b.parse()?))
	}
}

impl Display for Id {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}-{}", self.0, self.1)
	}
}
