/// Serde utility for values represented as strings, such as numbers stored in bulk strings.
///
/// The RESP deserializer only reads a type from its matching RESP type: strings from simple
/// strings, integers from integers, and bytes from bulk strings. This module instead parses a
/// value from its string representation using [`FromStr`](std::str::FromStr), accepting simple
/// strings, UTF-8 bulk strings, and integers. Values are serialized as bulk strings using
/// [`Display`](std::fmt::Display).
///
/// ```
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Foo {
///     #[serde(with = "redust_resp::util::parse")]
///     count: u64,
///     #[serde(with = "redust_resp::util::parse")]
///     name: String,
/// }
///
/// let data = b"*4\r\n$5\r\ncount\r\n$2\r\n42\r\n$4\r\nname\r\n$3\r\nfoo\r\n";
/// let (foo, _) = redust_resp::from_bytes::<Foo>(data).unwrap();
///
/// assert_eq!(foo.count, 42);
/// assert_eq!(foo.name, "foo");
/// ```
pub mod parse {
	use std::{
		fmt::{self, Display},
		marker::PhantomData,
		str::{from_utf8, FromStr},
	};

	use serde::{
		de::{self, Unexpected, Visitor},
		Deserializer, Serializer,
	};

	pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
	where
		T: Display,
		S: Serializer,
	{
		serializer.serialize_bytes(value.to_string().as_bytes())
	}

	pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
	where
		T: FromStr,
		T::Err: Display,
		D: Deserializer<'de>,
	{
		struct ParseVisitor<T>(PhantomData<T>);

		impl<'de, T> Visitor<'de> for ParseVisitor<T>
		where
			T: FromStr,
			T::Err: Display,
		{
			type Value = T;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("a string or integer")
			}

			fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				v.parse().map_err(E::custom)
			}

			fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				match from_utf8(v) {
					Ok(str) => self.visit_str(str),
					Err(_) => Err(E::invalid_value(Unexpected::Bytes(v), &self)),
				}
			}

			fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				self.visit_str(&v.to_string())
			}

			fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				self.visit_str(&v.to_string())
			}
		}

		deserializer.deserialize_any(ParseVisitor(PhantomData))
	}
}

/// Serde utility for borrowing a string from a simple string or a UTF-8 bulk string.
///
/// This is the zero-copy counterpart to [`parse`] for `&str`, which can't be
/// parsed with [`FromStr`](std::str::FromStr). The input must outlive the deserialized value, so
/// deserializing from owned data (such as [`Data`](crate::Data) after
/// [`into_owned`](crate::Data::into_owned)) fails. Values are serialized as bulk strings.
///
/// ```
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Foo<'a> {
///     #[serde(borrow, with = "redust_resp::util::str")]
///     name: &'a str,
/// }
///
/// let data = b"*2\r\n$4\r\nname\r\n$3\r\nfoo\r\n";
/// let (foo, _) = redust_resp::from_bytes::<Foo>(data).unwrap();
///
/// assert_eq!(foo.name, "foo");
/// ```
pub mod str {
	use std::{fmt, str::from_utf8};

	use serde::{
		de::{self, Unexpected, Visitor},
		Deserializer, Serializer,
	};

	pub fn serialize<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_bytes(value.as_bytes())
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<&'de str, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct StrVisitor;

		impl<'de> Visitor<'de> for StrVisitor {
			type Value = &'de str;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("a borrowed string")
			}

			fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(v)
			}

			fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				from_utf8(v).map_err(|_| E::invalid_value(Unexpected::Bytes(v), &self))
			}
		}

		deserializer.deserialize_any(StrVisitor)
	}
}

/// Serde utility for indicating a RESP array of tuples is represented as a map.
///
/// By default, the RESP (de)serializer will assume the only map representation is a flat array,
/// such as the response expected from [HGETALL](https://redis.io/commands/hgetall/).
///
/// ```
/// use std::collections::HashMap;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Foo {
///     #[serde(with = "redust_resp::util::tuple_map")]
///     bar: HashMap<String, isize>,
/// }
/// ```
pub mod tuple_map {
	use std::{collections::HashMap, hash::Hash, marker::PhantomData};

	use serde::{
		de::{SeqAccess, Visitor},
		ser::SerializeSeq,
		Deserialize, Deserializer, Serialize, Serializer,
	};

	pub fn serialize<K, V, S>(value: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
	where
		K: Serialize,
		V: Serialize,
		S: Serializer,
	{
		let mut seq = serializer.serialize_seq(Some(value.len()))?;
		for entry in value {
			seq.serialize_element(&entry)?;
		}
		seq.end()
	}

	pub fn deserialize<'de, K, V, D>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
	where
		K: Deserialize<'de> + Eq + Hash,
		V: Deserialize<'de>,
		D: Deserializer<'de>,
	{
		struct MapVisitor<K, V>(PhantomData<K>, PhantomData<V>);

		impl<K, V> Default for MapVisitor<K, V> {
			fn default() -> Self {
				Self(Default::default(), Default::default())
			}
		}

		impl<'de, K, V> Visitor<'de> for MapVisitor<K, V>
		where
			K: Deserialize<'de> + Eq + Hash,
			V: Deserialize<'de>,
		{
			type Value = HashMap<K, V>;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("list of data")
			}

			fn visit_seq<A>(self, mut visitor: A) -> Result<Self::Value, A::Error>
			where
				A: SeqAccess<'de>,
			{
				let mut items = HashMap::with_capacity(visitor.size_hint().unwrap_or(0));

				while let Some((k, v)) = visitor.next_element()? {
					items.insert(k, v);
				}

				Ok(items)
			}
		}

		deserializer.deserialize_seq(MapVisitor::default())
	}
}

#[cfg(test)]
mod test {
	use serde::{Deserialize, Serialize};

	use crate::{from_bytes, from_data, to_bytes, Data, Error};

	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	#[serde(transparent)]
	struct Parsed<T: std::str::FromStr + std::fmt::Display>(#[serde(with = "super::parse")] T)
	where
		T::Err: std::fmt::Display;

	#[test]
	fn parse_from_any_string() {
		for data in [&b"+-12\r\n"[..], b"$3\r\n-12\r\n", b":-12\r\n"] {
			let (res, rem) = from_bytes::<Parsed<i64>>(data).unwrap();

			assert_eq!(res, Parsed(-12));
			assert_eq!(rem, []);
		}

		let (res, _) = from_bytes::<Parsed<f64>>(b"$4\r\n1.25\r\n").unwrap();
		assert_eq!(res, Parsed(1.25));

		let (res, _) = from_bytes::<Parsed<String>>(b"$3\r\nfoo\r\n").unwrap();
		assert_eq!(res, Parsed("foo".to_owned()));

		let (res, _) = from_bytes::<Parsed<String>>(b":42\r\n").unwrap();
		assert_eq!(res, Parsed("42".to_owned()));
	}

	#[test]
	fn parse_from_data() {
		let res = from_data::<Parsed<u64>>(Data::BulkString(b"42".to_vec().into())).unwrap();
		assert_eq!(res, Parsed(42));
	}

	#[test]
	fn parse_invalid() {
		for data in [
			&b"$3\r\nfoo\r\n"[..],
			b"$2\r\n\xff\xfe\r\n",
			b"$-1\r\n",
			b"*0\r\n",
		] {
			let err = from_bytes::<Parsed<i64>>(data).unwrap_err();

			assert!(matches!(err.data, Error::Message(_)), "{:?}", err);
		}
	}

	#[test]
	fn parse_serialize() {
		let mut buf = Vec::new();
		to_bytes(&Parsed(-12), &mut buf).unwrap();

		assert_eq!(buf, b"$3\r\n-12\r\n");
	}

	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	#[serde(transparent)]
	struct Borrowed<'a>(#[serde(borrow, with = "super::str")] &'a str);

	#[test]
	fn str_borrowed() {
		for data in [&b"+foo\r\n"[..], b"$3\r\nfoo\r\n"] {
			let (res, rem) = from_bytes::<Borrowed>(data).unwrap();

			assert_eq!(res, Borrowed("foo"));
			assert_eq!(rem, []);
		}

		let res = from_data::<Borrowed>(Data::BulkString(b"foo"[..].into())).unwrap();
		assert_eq!(res, Borrowed("foo"));
	}

	#[test]
	fn str_invalid() {
		for data in [&b"$2\r\n\xff\xfe\r\n"[..], b":1\r\n", b"$-1\r\n", b"*0\r\n"] {
			let err = from_bytes::<Borrowed>(data).unwrap_err();

			assert!(matches!(err.data, Error::Message(_)), "{:?}", err);
		}
	}

	#[test]
	fn str_owned() {
		let data = Data::BulkString(b"foo".to_vec().into());
		let err = from_data::<Borrowed>(data).unwrap_err();

		assert!(matches!(err, Error::Message(_)), "{:?}", err);
	}

	#[test]
	fn str_serialize() {
		let mut buf = Vec::new();
		to_bytes(&Borrowed("foo"), &mut buf).unwrap();

		assert_eq!(buf, b"$3\r\nfoo\r\n");
	}
}
