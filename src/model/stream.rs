use std::{
	fmt::{Display, Formatter},
	str::FromStr,
};

use resp::nom::{
	character::complete::{char, u64},
	combinator::complete,
	error::Error,
	sequence::separated_pair,
	Err,
};
use serde::{de, Deserialize, Serialize, __private::from_utf8_lossy};

/// Models for XAUTOCLAIM commands.
pub mod claim;
/// Models for XREAD(GROUP) commands.
pub mod read;

/// A [stream ID](https://redis.io/topics/streams-intro#entry-ids).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
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

impl<'a> TryFrom<&'a [u8]> for Id {
	type Error = Error<&'a [u8]>;

	fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
		let (_, (a, b)) =
			complete(separated_pair(u64, char('-'), u64))(value).map_err(|e| match e {
				Err::Error(e) | Err::Failure(e) => e,
				Err::Incomplete(_) => unreachable!(),
			})?;

		Ok(Self(a, b))
	}
}

impl<'a> TryFrom<&'a str> for Id {
	type Error = Error<&'a [u8]>;

	fn try_from(value: &'a str) -> Result<Self, Self::Error> {
		value.as_bytes().try_into()
	}
}

impl FromStr for Id {
	type Err = Error<String>;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		s.try_into().map_err(|e: Error<&[u8]>| Error {
			input: String::from_utf8_lossy(e.input).into_owned(),
			code: e.code,
		})
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

		fn xform_err<E>(err: Error<&[u8]>) -> E
		where
			E: de::Error,
		{
			E::custom(Error {
				input: from_utf8_lossy(err.input),
				code: err.code,
			})
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
