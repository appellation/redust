use serde::{
	de::{self, value::SeqDeserializer, DeserializeOwned},
	forward_to_deserialize_any,
};

use crate::{de::Error, Data};

pub fn from_data<'de, 'a, T>(data: &'a Data<'de>) -> Result<T, Error<'a>>
where
	T: DeserializeOwned,
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

			fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(Data::SimpleString(v.into()))
			}

			fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(Data::BulkString(v.into()))
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
				let mut out = Vec::with_capacity(seq.size_hint().unwrap_or_default());
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
	array: impl Iterator<Item = &'de Data<'de>>,
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

impl<'de, 'a: 'de> de::Deserializer<'de> for &'a Data<'de> {
	type Error = Error<'de>;

	fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: de::Visitor<'de>,
	{
		match self {
			Data::Array(data) => visit_array(data.iter(), visitor),
			Data::BulkString(ref bytes) => visitor.visit_borrowed_bytes(bytes),
			Data::Integer(i) => visitor.visit_i64(*i),
			Data::Null => visitor.visit_none(),
			Data::SimpleString(ref str) => visitor.visit_borrowed_str(str),
		}
	}

	forward_to_deserialize_any! {
		bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
		bytes byte_buf option unit unit_struct newtype_struct seq tuple
		tuple_struct map struct enum identifier ignored_any
	}
}

impl<'de, 'a: 'de> de::IntoDeserializer<'de, Error<'de>> for &'a Data<'de> {
	type Deserializer = Self;

	fn into_deserializer(self) -> Self::Deserializer {
		self
	}
}

#[cfg(test)]
mod test {
	use crate::{array, de::Error, from_bytes, Data};

	#[test]
	fn de_str() {
		let bytes = b"+OK\r\n";
		let (data, rem) = from_bytes::<Data>(bytes).unwrap();

		assert_eq!(data, Data::SimpleString("OK".into()));
		assert_eq!(rem, []);
	}

	#[test]
	fn de_err() {
		let bytes = b"-Error\r\n";
		let err = from_bytes::<Data>(bytes).unwrap_err();

		assert_eq!(err, Error::RedisError("Error"));
	}

	#[test]
	fn de_int() {
		let bytes = b":123\r\n";
		let (data, rem) = from_bytes::<Data>(bytes).unwrap();

		assert_eq!(data, Data::Integer(123));
		assert_eq!(rem, []);
	}

	#[test]
	fn de_bulk_str() {
		let bytes = b"$3\r\nfoo\r\n";
		let (data, rem) = from_bytes::<Data>(bytes).unwrap();

		assert_eq!(data, Data::bulk_string("foo"));
		assert_eq!(rem, []);
	}

	#[test]
	fn de_arr() {
		let bytes = b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
		let (data, rem) = from_bytes::<Data>(bytes).unwrap();

		assert_eq!(
			data,
			array!(Data::bulk_string("hello"), Data::bulk_string("world"))
		);
		assert_eq!(rem, []);
	}
}
