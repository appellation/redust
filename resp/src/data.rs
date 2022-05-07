use std::borrow::Cow;

pub mod de;
pub mod ser;

/// RESP data. Read the [Redis documenation](https://redis.io/commands) for details on which type
/// to expect as a response.
///
/// Both [Data::BulkString] and [Data::Array] can represent nulls in RESP, but in this
/// representation they are not optional. They will be represented with [Data::Null] if the bulk
/// string or array is null.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Data<'a> {
	SimpleString(Cow<'a, str>),
	Integer(i64),
	BulkString(Cow<'a, [u8]>),
	Array(Vec<Data<'a>>),
	Null,
}

impl<'a> Data<'a> {
	/// Convenience method to create a [Data::SimpleString].
	pub fn simple_string<T>(str: &'a T) -> Self
	where
		T: AsRef<str> + ?Sized,
	{
		Self::SimpleString(str.as_ref().into())
	}

	/// Convenience method to create a [Data::BulkString].
	pub fn bulk_string<T>(bytes: &'a T) -> Self
	where
		T: AsRef<[u8]> + ?Sized,
	{
		Self::BulkString(bytes.as_ref().into())
	}

	/// Convert this data into owned data.
	pub fn into_owned(self) -> Data<'static> {
		match self {
			Self::SimpleString(str) => Data::SimpleString(str.into_owned().into()),
			Self::Integer(int) => Data::Integer(int),
			Self::BulkString(bytes) => Data::BulkString(bytes.into_owned().into()),
			Self::Array(arr) => Data::Array(arr.into_iter().map(Data::into_owned).collect()),
			Self::Null => Data::Null,
		}
	}
}

/// Macro to simplify making a [Data::Array].
///
/// Changes:
/// ```rust
/// use resp::Data;
///
/// Data::Array(vec![Data::simple_string("foo"), Data::simple_string("bar")]);
/// ```
/// into
/// ```rust
/// use resp::{array, Data};
///
/// array!(Data::simple_string("foo"), Data::simple_string("bar"));
/// ```
#[macro_export]
macro_rules! array {
	($($items:expr),*) => {
		Data::Array(vec![$($items),*])
	};
}
