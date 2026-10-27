use std::{
	error::Error,
	fmt::{Display, Formatter},
	str::FromStr,
};

use redust_resp::resp2::parser::parse_digits;
use serde::{de, Deserialize, Serialize};

/// Models for XAUTOCLAIM commands.
pub mod claim;
/// Models for XREAD(GROUP) commands.
pub mod read;

/// A [stream ID](https://redis.io/topics/streams-intro#entry-ids).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(into = "Vec<u8>")]
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

/// An error parsing a stream [`Id`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseIdError;

impl Display for ParseIdError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("invalid stream ID")
	}
}

impl Error for ParseIdError {}

impl TryFrom<&[u8]> for Id {
	type Error = ParseIdError;

	fn try_from(mut value: &[u8]) -> Result<Self, Self::Error> {
		let timestamp = parse_digits(&mut value).map_err(|_| ParseIdError)?;
		value = value.strip_prefix(b"-").ok_or(ParseIdError)?;
		let sequence = parse_digits(&mut value).map_err(|_| ParseIdError)?;

		Ok(Self(timestamp, sequence))
	}
}

impl TryFrom<&str> for Id {
	type Error = ParseIdError;

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		value.as_bytes().try_into()
	}
}

impl FromStr for Id {
	type Err = ParseIdError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		s.try_into()
	}
}

impl Display for Id {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}-{}", self.0, self.1)
	}
}

impl<'de> Deserialize<'de> for Id {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		struct Visitor;

		fn xform_err<E>(err: ParseIdError) -> E
		where
			E: de::Error,
		{
			E::custom(err)
		}

		impl<'de> de::Visitor<'de> for Visitor {
			type Value = Id;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("bytes or string")
			}

			fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				v.as_slice().try_into().map_err(xform_err)
			}

			fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				v.try_into().map_err(xform_err)
			}

			fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				v.try_into().map_err(xform_err)
			}

			fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				v.try_into().map_err(xform_err)
			}

			fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				v.try_into().map_err(xform_err)
			}

			fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				v.as_str().try_into().map_err(xform_err)
			}
		}

		deserializer.deserialize_any(Visitor)
	}
}

#[cfg(test)]
mod test {
	use super::{Id, ParseIdError};

	#[test]
	fn parse_id() {
		assert_eq!("1234-5678".parse(), Ok(Id(1234, 5678)));
		assert_eq!("0-0".parse(), Ok(Id(0, 0)));
		assert_eq!("18446744073709551615-1".parse(), Ok(Id(u64::MAX, 1)));
	}

	#[test]
	fn parse_invalid_id() {
		for id in [
			"",
			"1234",
			"1234-",
			"-5678",
			"a-1",
			"1-a",
			"18446744073709551616-0",
		] {
			assert_eq!(id.parse::<Id>(), Err(ParseIdError), "{id:?}");
		}
	}
}
