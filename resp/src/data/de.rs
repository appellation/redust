use std::borrow::Cow;

use itertools::Itertools;
use serde::{
	de::{
		self,
		value::{MapDeserializer, SeqDeserializer},
	},
	forward_to_deserialize_any, Deserialize,
};

use crate::{Data, Error};

pub fn from_data<'de, T>(data: Data<'de>) -> Result<T, Error<'de>>
where
	T: Deserialize<'de>,
{
	T::deserialize(data)
}

impl<'de> de::Deserialize<'de> for Data<'de> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		struct Visitor;

		impl<'de> de::Visitor<'de> for Visitor {
			type Value = Data<'de>;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(formatter, "valid RESP data")
			}

			fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(Data::Integer(v))
			}

			fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(Data::SimpleString(Cow::Owned(v)))
			}

			fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				self.visit_string(v.to_owned())
			}

			fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(Data::SimpleString(Cow::Borrowed(v)))
			}

			fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(Data::BulkString(Cow::Owned(v)))
			}

			fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				self.visit_byte_buf(v.to_owned())
			}

			fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(Data::BulkString(Cow::Borrowed(v)))
			}

			fn visit_none<E>(self) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(Data::Null)
			}

			fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
			where
				D: serde::Deserializer<'de>,
			{
				de::Deserialize::deserialize(deserializer)
			}

			fn visit_unit<E>(self) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				self.visit_none()
			}

			fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
			where
				D: serde::Deserializer<'de>,
			{
				de::Deserialize::deserialize(deserializer)
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: de::SeqAccess<'de>,
			{
				let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0));
				while let Some(v) = seq.next_element()? {
					out.push(v);
				}

				Ok(Data::Array(out))
			}
		}

		deserializer.deserialize_any(Visitor)
	}
}

fn visit_array<'de, V>(
	array: impl Iterator<Item = Data<'de>>,
	visitor: V,
) -> Result<V::Value, Error<'de>>
where
	V: de::Visitor<'de>,
{
	let mut deserializer = SeqDeserializer::new(array);
	let seq = visitor.visit_seq(&mut deserializer)?;
	deserializer.end()?;
	Ok(seq)
}

fn visit_map<'de, V>(
	array: impl Iterator<Item = Data<'de>>,
	visitor: V,
) -> Result<V::Value, Error<'de>>
where
	V: de::Visitor<'de>,
{
	let mut deserializer = MapDeserializer::new(array.tuples::<(_, _)>());
	let seq = visitor.visit_map(&mut deserializer)?;
	deserializer.end()?;
	Ok(seq)
}

impl<'de> de::Deserializer<'de> for Data<'de> {
	type Error = Error<'de>;

	fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: de::Visitor<'de>,
	{
		match self {
			Data::Array(data) => visit_array(data.into_iter(), visitor),
			Data::BulkString(bytes) => match bytes {
				Cow::Owned(b) => visitor.visit_byte_buf(b),
				Cow::Borrowed(b) => visitor.visit_borrowed_bytes(b),
			},
			Data::Integer(i) => visitor.visit_i64(i),
			Data::Null => visitor.visit_none(),
			Data::SimpleString(str) => match str {
				Cow::Owned(s) => visitor.visit_string(s),
				Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
			},
		}
	}

	fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: de::Visitor<'de>,
	{
		fn make_err<T, E>(unex: de::Unexpected) -> Result<T, E>
		where
			E: de::Error,
		{
			Err(de::Error::invalid_type(unex, &"sequence"))
		}

		match self {
			Data::Array(data) => visit_map(data.into_iter(), visitor),
			Data::BulkString(b) => make_err(de::Unexpected::Bytes(&b)),
			Data::Integer(i) => make_err(de::Unexpected::Signed(i)),
			Data::Null => make_err(de::Unexpected::Unit),
			Data::SimpleString(str) => make_err(de::Unexpected::Str(&str)),
		}
	}

	fn deserialize_struct<V>(
		self,
		_name: &'static str,
		_fields: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: de::Visitor<'de>,
	{
		self.deserialize_map(visitor)
	}

	forward_to_deserialize_any! {
		bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
		bytes byte_buf option unit unit_struct newtype_struct seq tuple
		tuple_struct enum identifier ignored_any
	}
}

impl<'de> de::IntoDeserializer<'de, Error<'de>> for Data<'de> {
	type Deserializer = Self;

	fn into_deserializer(self) -> Self::Deserializer {
		self
	}
}

#[cfg(test)]
mod test {
	use crate::{array, Data};

	use super::from_data;

	#[test]
	fn to_str() {
		let res = from_data::<&str>(Data::simple_string("foo")).unwrap();
		assert_eq!(res, "foo");
	}

	#[test]
	fn to_bytes() {
		let res = from_data::<&[u8]>(Data::bulk_string(b"foo")).unwrap();
		assert_eq!(res, b"foo");
	}

	#[test]
	fn to_arr() {
		let res = from_data::<Vec<&str>>(array!(Data::simple_string("foo"))).unwrap();
		assert_eq!(res, vec!["foo"]);
	}

	#[test]
	fn to_int() {
		let res = from_data::<isize>(Data::Integer(42)).unwrap();
		assert_eq!(res, 42);
	}
}
