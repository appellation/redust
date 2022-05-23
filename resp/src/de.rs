use std::{borrow::Cow, fmt::Display};

use serde::{de, Deserialize};

mod accessor;
mod deserializer;

pub use accessor::*;
pub use deserializer::*;

use crate::Error;

/// An error occurred while reading bytes.
#[derive(Debug)]
pub struct ReadError<'a> {
	/// The error which occurred.
	pub data: Error<'a>,
	/// Bytes remaining to be read.
	pub remaining: Cow<'a, [u8]>,
}

impl ReadError<'_> {
	/// Convert this error into an owned error.
	pub fn into_owned(self) -> ReadError<'static> {
		ReadError {
			data: self.data.into_owned(),
			remaining: self.remaining.into_owned().into(),
		}
	}
}

impl Display for ReadError<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.data)
	}
}

impl std::error::Error for ReadError<'_> {
	fn cause(&self) -> Option<&dyn de::StdError> {
		Some(&self.data)
	}
}

/// Deserialize RESP bytes, returning the target and any remaining bytes.
pub fn from_bytes<'de, T: Deserialize<'de>>(
	data: &'de [u8],
) -> Result<(T, &'de [u8]), ReadError<'de>> {
	let mut de = Deserializer { input: data };
	let res = de::Deserialize::deserialize(&mut de).map_err(|e| ReadError {
		data: e,
		remaining: de.input.into(),
	})?;
	Ok((res, de.input))
}

#[cfg(test)]
mod test {
	use std::collections::HashMap;

	use serde_bytes::Bytes;

	use crate::{array, from_bytes, Data, Error};

	#[test]
	fn de_int() {
		let data = b":1\r\n";
		let (res, rem) = from_bytes::<u8>(data).unwrap();

		assert_eq!(res, 1);
		assert_eq!(rem, []);
	}

	#[test]
	fn de_str() {
		let data = b"+foo\r\n";
		let (res, rem) = from_bytes::<&str>(data).unwrap();

		assert_eq!(res, "foo");
		assert_eq!(rem, []);
	}

	#[test]
	fn de_error() {
		let data = b"-foo\r\n";
		let err = from_bytes::<()>(data).unwrap_err();

		match err.data {
			Error::Redis(_) => {}
			_ => panic!("unexpected error type {}", err),
		}
	}

	#[test]
	fn de_bytes() {
		let data = b"$3\r\nfoo\r\n";
		let (res, rem) = from_bytes::<&[u8]>(data).unwrap();

		assert_eq!(res, b"foo");
		assert_eq!(rem, []);
	}

	#[test]
	fn de_null_bytes() {
		let data = b"$-1\r\n";
		let (res, rem) = from_bytes::<Option<&[u8]>>(data).unwrap();

		assert_eq!(res, None);
		assert_eq!(rem, []);
	}

	#[test]
	fn de_arr() {
		let data = b"*1\r\n+foo\r\n";
		let (res, rem) = from_bytes::<Vec<String>>(data).unwrap();

		assert_eq!(res, ["foo"]);
		assert_eq!(rem, []);
	}

	#[test]
	fn de_null_arr() {
		let data = b"*-1\r\n";
		let (res, rem) = from_bytes::<Option<Vec<i64>>>(data).unwrap();

		assert_eq!(res, None);
		assert_eq!(rem, []);
	}

	#[test]
	fn de_nested_arr() {
		let data = b"*2\r\n+foo\r\n*1\r\n$3\r\nbar\r\n";
		let (res, rem) = from_bytes::<(&str, Vec<&[u8]>)>(data).unwrap();

		assert_eq!(res, ("foo", [&b"bar"[..]].to_vec()));
		assert_eq!(rem, []);
	}

	#[test]
	fn de_pubsub_subscribe() {
		let data = b"*3\r\n$9\r\nsubscribe\r\n$3\r\nfoo\r\n:1\r\n";
		let (res, rem) = from_bytes::<(&Bytes, &Bytes, usize)>(data).unwrap();

		assert_eq!(res, (Bytes::new(b"subscribe"), Bytes::new(b"foo"), 1));
		assert_eq!(rem, []);
	}

	#[test]
	fn de_map() {
		let data = b"*2\r\n+foo\r\n*1\r\n$3\r\nbar\r\n";
		let (res, rem) = from_bytes::<HashMap<&str, Vec<&Bytes>>>(data).unwrap();

		let mut exp = HashMap::new();
		exp.insert("foo", vec![Bytes::new(b"bar")]);

		assert_eq!(res, exp);
		assert_eq!(rem, []);
	}

	#[test]
	fn de_data_str() {
		let bytes = b"+OK\r\n";
		let (data, rem) = from_bytes::<Data>(bytes).unwrap();

		assert_eq!(data, Data::SimpleString("OK".into()));
		assert_eq!(rem, []);
	}

	#[test]
	fn de_data_err() {
		let bytes = b"-Error\r\n";
		let err = from_bytes::<Data>(bytes).unwrap_err();

		match err.data {
			Error::Redis(msg) if msg == "Error" => {}
			_ => panic!(),
		}
	}

	#[test]
	fn de_data_int() {
		let bytes = b":123\r\n";
		let (data, rem) = from_bytes::<Data>(bytes).unwrap();

		assert_eq!(data, Data::Integer(123));
		assert_eq!(rem, []);
	}

	#[test]
	fn de_data_bulk_str() {
		let bytes = b"$3\r\nfoo\r\n";
		let (data, rem) = from_bytes::<Data>(bytes).unwrap();

		assert_eq!(data, Data::bulk_string("foo"));
		assert_eq!(rem, []);
	}

	#[test]
	fn de_data_arr() {
		let bytes = b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
		let (data, rem) = from_bytes::<Data>(bytes).unwrap();

		assert_eq!(
			data,
			array!(Data::bulk_string("hello"), Data::bulk_string("world"))
		);
		assert_eq!(rem, []);
	}
}
