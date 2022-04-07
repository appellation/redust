use serde::{de, Deserialize};

mod accessor;
mod deserializer;
mod error;

pub use accessor::*;
pub use deserializer::*;
pub use error::*;

pub fn from_bytes<'de, T: Deserialize<'de>>(data: &'de [u8]) -> Result<(T, &'de [u8]), Error<'de>> {
	let mut de = Deserializer { input: data };
	let res = de::Deserialize::deserialize(&mut de)?;
	Ok((res, de.input))
}

#[cfg(test)]
mod test {
	use std::{borrow::Cow, collections::HashMap};

	use crate::from_bytes;

	use super::Error;

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

		match err {
			Error::RedisError(_) => {}
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
		let (res, rem) = from_bytes::<(&str, Cow<'_, [u8]>, usize)>(data).unwrap();

		assert_eq!(res, ("subscribe", b"foo"[..].into(), 1));
		assert_eq!(rem, []);
	}

	#[test]
	fn de_map() {
		let data = b"*2\r\n+foo\r\n*1\r\n$3\r\nbar\r\n";
		let (res, rem) = from_bytes::<HashMap<&str, Vec<&[u8]>>>(data).unwrap();

		let mut exp = HashMap::new();
		exp.insert("foo", vec![b"bar"[..].into()]);

		assert_eq!(res, exp);
		assert_eq!(rem, []);
	}
}
