use std::{borrow::Cow, collections::HashMap, ops::Index};

use resp::Data;

use crate::model::error::{Error, Result};

use super::Id;

/// A stream key in the Redis keyspace.
pub type Key<'a> = Cow<'a, [u8]>;
/// A field from a stream, associated to a [Value].
pub type Field<'a> = Cow<'a, [u8]>;
/// A value from a stream, keyed by a [Field].
pub type Value<'a> = Cow<'a, [u8]>;
/// All entries in a stream, belonging to a [Key].
pub type Entries<'a> = HashMap<Id, Entry<'a>>;

type InnerEntry<'a> = HashMap<Field<'a>, Value<'a>>;

/// An entry in a stream, keyed by [Id].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry<'a>(pub InnerEntry<'a>);

impl<'a> FromIterator<(Field<'a>, Value<'a>)> for Entry<'a> {
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = (Field<'a>, Value<'a>)>,
	{
		Self(iter.into_iter().collect())
	}
}

impl<'a, I> Index<I> for Entry<'a>
where
	I: AsRef<[u8]>,
{
	type Output = Value<'a>;

	fn index(&self, index: I) -> &Self::Output {
		&self.0[index.as_ref()]
	}
}

type InnerReadResponse<'a> = HashMap<Key<'a>, Entries<'a>>;

/// Response from XREAD(GROUP) command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadResponse<'a>(pub InnerReadResponse<'a>);

impl<'a, I> Index<I> for ReadResponse<'a>
where
	I: AsRef<[u8]>,
{
	type Output = Entries<'a>;

	fn index(&self, index: I) -> &Self::Output {
		&self.0[index.as_ref()]
	}
}

impl<'a> TryFrom<Data<'a>> for ReadResponse<'a> {
	type Error = Error;

	fn try_from(value: Data<'a>) -> Result<Self, Self::Error> {
		let inner = value
			.into_array()?
			.into_iter()
			.map(ReadResponse::parse_key)
			.collect::<Result<InnerReadResponse, _>>()?;

		Ok(Self(inner))
	}
}

impl<'a> ReadResponse<'a> {
	fn parse_key(data: Data<'_>) -> Result<(Key, Entries)> {
		// KEY => [ID, [F, V, ...]]
		let mut iter = data.into_array()?.into_iter();

		let key = iter.next().ok_or(Error::MissingElement(0))?;
		let value = iter.next().ok_or(Error::MissingElement(1))?;

		Ok((
			key.into_bulk_str()?,
			value
				.into_array()?
				.into_iter()
				.map(ReadResponse::parse_entries)
				.collect::<Result<Entries, _>>()?,
		))
	}

	pub(crate) fn parse_entries(data: Data<'_>) -> Result<(Id, Entry)> {
		// ID => [F, V, ...]
		let mut iter = data.into_array()?.into_iter();

		let key = iter.next().ok_or(Error::MissingElement(0))?;
		let value = iter.next().ok_or(Error::MissingElement(1))?;

		Ok((
			Id::try_from(key)?,
			value
				.into_array()?
				.chunks_exact(2)
				.map(ReadResponse::parse_entry)
				.collect::<Result<Entry, _>>()?,
		))
	}

	fn parse_entry(chunk: &[Data<'a>]) -> Result<(Field<'a>, Value<'a>)> {
		// [F, V, ...]
		let mut chunk = chunk
			.into_iter()
			.cloned()
			.map(|d| Ok::<_, Error>(d.into_bulk_str()?));

		Ok((
			chunk.next().ok_or(Error::MissingElement(0))??,
			chunk.next().ok_or(Error::MissingElement(1))??,
		))
	}
}

#[cfg(test)]
mod test {
	use std::borrow::Cow;

	use resp::{array, Data};

	use crate::model::stream::Id;

	use super::ReadResponse;

	#[test]
	fn stream_read() {
		let data = array![array![
			Data::BulkString(Some(b"foo"[..].into())),
			array![array![
				Data::BulkString(Some(b"1-0"[..].into())),
				array![
					Data::BulkString(Some(b"abc"[..].into())),
					Data::BulkString(Some(b"def"[..].into()))
				]
			]]
		]];

		let resp = ReadResponse::try_from(data).expect("read data");
		assert_eq!(resp["foo"][&Id(1, 0)]["abc"], Cow::from(&b"def"[..]));
	}
}
