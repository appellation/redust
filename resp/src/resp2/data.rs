use std::borrow::Cow;

pub mod de;
pub mod ser;

/// RESP data. Read the [Redis documenation](https://redis.io/commands) for details on which type
/// to expect as a response.
///
/// Both [Data::BulkString] and [Data::Array] can represent nulls in RESP, but in this
/// representation they are not optional. They will be represented with [Data::Null] if the bulk
/// string or array is null.
///
/// Errors are not represented here for two reasons: 1) it's never correct to send an error to the
/// Redis server, and 2) it's more ergonomic to have errors returned in a [Result](crate::Result).
///
/// Since errors are not represented, it's possible to convert a Rust string into `Data` without
/// ambiguity.
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

	pub fn from_bytes_iter<I, B>(iter: I) -> Data<'a>
	where
		I: IntoIterator<Item = &'a B>,
		B: 'a + AsRef<[u8]> + ?Sized,
	{
		Data::Array(iter.into_iter().map(Data::from_bytes).collect())
	}

	pub fn from_bytes<B>(bytes: &'a B) -> Data<'a>
	where
		B: 'a + AsRef<[u8]> + ?Sized,
	{
		Data::BulkString(bytes.as_ref().into())
	}
}

impl<'a> From<&'a str> for Data<'a> {
	fn from(str: &'a str) -> Self {
		Data::SimpleString(str.into())
	}
}

impl From<String> for Data<'_> {
	fn from(str: String) -> Self {
		Data::SimpleString(str.into())
	}
}

impl From<i64> for Data<'_> {
	fn from(i: i64) -> Self {
		Data::Integer(i)
	}
}

impl<'a, const N: usize> From<&'a [u8; N]> for Data<'a> {
	fn from(bytes: &'a [u8; N]) -> Self {
		Self::bulk_string(bytes)
	}
}

impl<'a> From<&'a [u8]> for Data<'a> {
	fn from(bytes: &'a [u8]) -> Self {
		Self::BulkString(bytes.into())
	}
}

impl From<Vec<u8>> for Data<'_> {
	fn from(bytes: Vec<u8>) -> Self {
		Self::BulkString(bytes.into())
	}
}

impl<I> FromIterator<I> for Data<'_>
where
	Self: From<I>,
{
	fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
		iter.into_iter()
			.map(Data::from)
			.collect::<Vec<Data>>()
			.into()
	}
}

impl<'a> From<Vec<Data<'a>>> for Data<'a> {
	fn from(data: Vec<Data<'a>>) -> Self {
		Self::Array(data)
	}
}

impl From<()> for Data<'_> {
	fn from(_: ()) -> Self {
		Data::Null
	}
}

impl PartialEq<str> for Data<'_> {
	fn eq(&self, other: &str) -> bool {
		matches!(self, Data::SimpleString(str) if str == other)
	}
}

impl PartialEq<&str> for Data<'_> {
	fn eq(&self, other: &&str) -> bool {
		matches!(self, Data::SimpleString(str) if str == other)
	}
}

impl PartialEq<[u8]> for Data<'_> {
	fn eq(&self, other: &[u8]) -> bool {
		matches!(self, Data::BulkString(bytes) if bytes.as_ref() == other)
	}
}

impl PartialEq<&[u8]> for Data<'_> {
	fn eq(&self, other: &&[u8]) -> bool {
		matches!(self, Data::BulkString(bytes) if bytes == other)
	}
}

impl<const N: usize> PartialEq<[u8; N]> for Data<'_> {
	fn eq(&self, other: &[u8; N]) -> bool {
		matches!(self, Data::BulkString(bytes) if bytes.as_ref() == other)
	}
}

impl<const N: usize> PartialEq<&[u8; N]> for Data<'_> {
	fn eq(&self, other: &&[u8; N]) -> bool {
		matches!(self, Data::BulkString(bytes) if bytes.as_ref() == *other)
	}
}

impl PartialEq<i64> for Data<'_> {
	fn eq(&self, other: &i64) -> bool {
		matches!(self, Data::Integer(i) if *i == *other)
	}
}

impl PartialEq<()> for Data<'_> {
	fn eq(&self, _: &()) -> bool {
		matches!(self, Data::Null)
	}
}

/// Macro to simplify making a [Data::Array].
///
/// Changes:
/// ```rust
/// # use redust_resp::Data;
/// Data::Array(vec![Data::simple_string("foo"), Data::simple_string("bar")]);
/// ```
/// into
/// ```rust
/// # use redust_resp::array;
/// array!("foo", "bar");
/// ```
#[macro_export]
macro_rules! array {
	($($items:expr),*) => {
		$crate::Data::Array(vec![$($crate::Data::from($items)),*])
	};
}
