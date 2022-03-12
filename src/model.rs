/// Models related to Redis streams.
pub mod stream {
	use std::{borrow::Cow, collections::HashMap, ops::Index, str::from_utf8};

	use resp::Data;

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
	pub struct Entry<'a>(InnerEntry<'a>);

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

		#[inline]
		fn index(&self, index: I) -> &Self::Output {
			&self.0[index.as_ref()]
		}
	}

	/// A [stream ID](https://redis.io/topics/streams-intro#entry-ids).
	#[derive(Debug, Clone, PartialEq, Eq, Hash)]
	pub struct Id(
		/// The timestamp, in milliseconds
		pub u64,
		/// The sequence number
		pub u64,
	);

	impl Id {
		/// Try to create an ID from Redis data. Returns [None] if the data does not represent an ID.
		pub fn try_from_data<'a>(data: Data<'a>) -> Option<Self> {
			match data {
				Data::SimpleString(str) => Self::parse(&str),
				Data::BulkString(Some(str)) => Self::parse(from_utf8(&str).ok()?),
				_ => None,
			}
		}

		pub fn parse(input: &str) -> Option<Self> {
			let (a, b) = input.split_once('-')?;
			Some(Self(a.parse().ok()?, b.parse().ok()?))
		}
	}

	type InnerReadResponse<'a> = HashMap<Key<'a>, Entries<'a>>;

	/// Response from XREAD(GROUP) command.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct ReadResponse<'a>(InnerReadResponse<'a>);

	impl<'a, I> Index<I> for ReadResponse<'a>
	where
		I: AsRef<[u8]>,
	{
		type Output = Entries<'a>;

		fn index(&self, index: I) -> &Self::Output {
			&self.0[index.as_ref()]
		}
	}

	impl<'a> ReadResponse<'a> {
		/// Try to create a ReadResponse from Redis data. Returns [None] if the data does not represent
		/// a read response.
		pub fn try_from_data(data: Data<'a>) -> Option<Self> {
			let inner = data
				.into_array()?
				.into_iter()
				.filter_map(ReadResponse::parse_key)
				.collect();

			Some(Self(inner))
		}

		fn parse_key(data: Data<'_>) -> Option<(Key, Entries)> {
			// KEY => [ID, [F, V, ...]]
			let [key, value]: [Data; 2] = dbg!(data).into_array()?.try_into().ok()?;
			Some((
				key.into_bulk_str()?,
				value
					.into_array()?
					.into_iter()
					.filter_map(ReadResponse::parse_entries)
					.collect(),
			))
		}

		fn parse_entries(data: Data<'_>) -> Option<(Id, Entry)> {
			// ID => [F, V, ...]
			let [key, value]: [Data; 2] = data.into_array()?.try_into().ok()?;
			Some((
				Id::try_from_data(key)?,
				value
					.into_array()?
					.chunks_exact(2)
					.filter_map(ReadResponse::parse_entry)
					.collect(),
			))
		}

		fn parse_entry(chunk: &[Data<'a>]) -> Option<(Field<'a>, Value<'a>)> {
			// [F, V, ...]
			let mut chunk = chunk
				.into_iter()
				.cloned()
				.filter_map(|d| Some(d.into_bulk_str()?));
			Some((chunk.next()?, chunk.next()?))
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

			let resp = ReadResponse::try_from_data(data).expect("read data");
			assert_eq!(resp["foo"][&Id(1, 0)]["abc"], Cow::from(&b"def"[..]));
		}
	}
}
