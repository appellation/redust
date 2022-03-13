use std::{
	fmt::{Display, Formatter},
	str::from_utf8,
};

use resp::Data;

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

impl Id {
	/// Try to create an ID from Redis data. Returns [None] if the data does not represent an ID.
	pub fn try_from_data<'a>(data: Data<'a>) -> Option<Self> {
		match data {
			Data::SimpleString(str) => Self::parse(&str),
			Data::BulkString(Some(str)) => Self::parse(from_utf8(&str).ok()?),
			_ => None,
		}
	}

	pub fn parse(input: &str) -> Option<Self> {
		let (a, b) = input.split_once('-')?;
		Some(Self(a.parse().ok()?, b.parse().ok()?))
	}
}

impl Display for Id {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}-{}", self.0, self.1)
	}
}
