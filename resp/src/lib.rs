use std::borrow::Cow;

use bytes::{BufMut, BytesMut};
pub use nom;
use parser::parse;
pub use parser::Error;

pub mod borrow;
pub mod parser;

const CRLF: [u8; 2] = [b'\r', b'\n'];

/// RESP data. Read the [Redis documenation](https://redis.io/commands) for details on which type
/// to expect as a response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Data<'a> {
	SimpleString(Cow<'a, str>),
	Error(Cow<'a, str>),
	Integer(i64),
	BulkString(Option<Cow<'a, [u8]>>),
	Array(Option<Vec<Data<'a>>>),
}

impl<'a> Data<'a> {
	/// Convenience method to create a [Data::SimpleString].
	pub fn simple_string(str: &'a (impl AsRef<str> + ?Sized)) -> Self {
		Self::SimpleString(str.as_ref().into())
	}

	/// Convenience method to create a [Data::BulkString].
	pub fn bulk_string(bytes: &'a (impl AsRef<[u8]> + ?Sized)) -> Self {
		Self::BulkString(Some(bytes.as_ref().into()))
	}

	/// Convert this data into owned data.
	pub fn into_owned(self) -> Data<'static> {
		match self {
			Self::SimpleString(str) => Data::SimpleString(str.into_owned().into()),
			Self::Error(str) => Data::Error(str.into_owned().into()),
			Self::Integer(int) => Data::Integer(int),
			Self::BulkString(bytes) => Data::BulkString(bytes.map(|s| s.into_owned().into())),
			Self::Array(str) => {
				Data::Array(str.map(|s| s.into_iter().map(Data::into_owned).collect()))
			}
		}
	}

	/// Convert this data into an array.
	///
	/// Returns [None] if this is not an array or there is no array data.
	pub fn into_array(self) -> Option<Vec<Data<'a>>> {
		match self {
			Data::Array(arr) => arr,
			_ => None,
		}
	}

	/// Convert this data into a string.
	///
	/// Returns [None] if this is not a string.
	pub fn into_str(self) -> Option<Cow<'a, str>> {
		match self {
			Self::SimpleString(str) => Some(str),
			_ => None,
		}
	}

	/// Convert this data into a bulk string (bytes).
	///
	/// Returns [None] if this is not a bulk string or there is no bulk string data.
	pub fn into_bulk_str(self) -> Option<Cow<'a, [u8]>> {
		match self {
			Self::BulkString(str) => str,
			_ => None,
		}
	}

	/// Get this data as an integer.
	///
	/// Returns [None] if this is not an integer.
	pub fn as_int(&self) -> Option<i64> {
		match self {
			Self::Integer(int) => Some(*int),
			_ => None,
		}
	}

	pub fn to_bytes(&self, dst: &mut BytesMut) {
		match self {
			Data::SimpleString(str) => {
				dst.reserve(1 + str.len() + CRLF.len());

				dst.put_u8(b'+');
				dst.extend_from_slice(str.as_bytes());
				dst.extend_from_slice(&CRLF);
			}
			Data::Error(str) => {
				dst.reserve(1 + str.len() + CRLF.len());

				dst.put_u8(b'-');
				dst.extend_from_slice(str.as_bytes());
				dst.extend_from_slice(&CRLF);
			}
			Data::Integer(int) => {
				let str = int.to_string();
				dst.reserve(1 + str.len() + CRLF.len());

				dst.put_u8(b':');
				dst.extend_from_slice(str.as_bytes());
				dst.extend_from_slice(&CRLF);
			}
			Data::BulkString(Some(bytes)) => {
				let len = bytes.len().to_string();
				dst.reserve(1 + len.len() + CRLF.len() + bytes.len() + CRLF.len());

				dst.put_u8(b'$');
				dst.extend_from_slice(len.as_bytes());
				dst.extend_from_slice(&CRLF);
				dst.extend_from_slice(&bytes);
				dst.extend_from_slice(&CRLF);
			}
			Data::BulkString(None) => dst.extend_from_slice(&[b'$', b'-', b'1', b'\r', b'\n']),
			Data::Array(Some(data)) => {
				let len = data.len().to_string();
				dst.reserve(1 + len.len() + CRLF.len());

				dst.put_u8(b'*');
				dst.extend_from_slice(len.as_bytes());
				dst.extend_from_slice(&CRLF);

				for inner in data {
					inner.to_bytes(dst);
				}

				dst.extend_from_slice(&CRLF);
			}
			Data::Array(None) => dst.extend_from_slice(&[b'*', b'-', b'1', b'\r', b'\n']),
		}
	}
}

impl<'a> TryFrom<&'a [u8]> for Data<'a> {
	type Error = Error<'a>;

	fn try_from(value: &'a [u8]) -> Result<Self, Error<'a>> {
		let (_, data) = parse(value)?;
		Ok(data)
	}
}

/// Macro to simplify making a [Data::Array].
///
/// Changes:
/// ```rust
/// use resp::Data;
///
/// Data::Array(Some(vec![Data::SimpleString("foo".into()), Data::SimpleString("bar".into())]));
/// ```
/// into
/// ```rust
/// use resp::{array, Data};
///
/// array!(Data::SimpleString("foo".into()), Data::SimpleString("bar".into()));
/// ```
#[macro_export]
macro_rules! array {
	($($items:expr),*) => {
		Data::Array(Some(vec![$($items),*]))
	};
}

#[cfg(test)]
mod test {
	use std::borrow::Cow;

	use crate::Data;

	#[test]
	fn from_simple_string() {
		Data::simple_string("foo");
		Data::simple_string(&Cow::from("foo"));
	}

	#[test]
	fn from_bytes() {
		Data::bulk_string("foo");
		Data::bulk_string(b"foo");
	}

	#[test]
	fn array_macro() {
		let arr = array!(
			Data::SimpleString("foo".into()),
			Data::SimpleString("bar".into())
		);

		assert_eq!(
			arr,
			Data::Array(Some(vec![
				Data::SimpleString("foo".into()),
				Data::SimpleString("bar".into())
			]))
		);
	}

	#[test]
	fn empty_array_macro() {
		array!();
	}
}
