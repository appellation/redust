/// Models related to Redis streams.
pub mod stream {
	use std::{collections::HashMap, ops::Deref};

	use bytes::Bytes;
	use resp::Data;

	/// A stream key in the Redis keyspace.
	pub type Key = Bytes;
	/// A field from a stream, associated to a [Value].
	pub type Field = Bytes;
	/// A value from a stream, keyed by a [Field].
	pub type Value = Bytes;
	/// An entry in a stream, keyed by [Id].
	pub type Entry = HashMap<Field, Value>;
	/// All entries in a stream, belonging to a [Key].
	pub type Entries = HashMap<Id, Entry>;
	type InnerReadResponse = HashMap<Key, Entries>;

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
				Data::SimpleString(str) => {
					let (a, b) = str.split_once('-')?;
					Some(Self(a.parse().ok()?, b.parse().ok()?))
				}
				Data::BulkString(Some(str)) => {
					let (a, b) = std::str::from_utf8(str).ok()?.split_once('-')?;
					Some(Self(a.parse().ok()?, b.parse().ok()?))
				}
				_ => None,
			}
		}
	}

	/// Response from XREAD(GROUP) command.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct ReadResponse(InnerReadResponse);

	impl Deref for ReadResponse {
		type Target = InnerReadResponse;

		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}

	impl ReadResponse {
		/// Try to create a ReadResponse from Redis data. Returns [None] if the data does not represent
		/// a read response.
		pub fn try_from_data<'a>(data: Data<'a>) -> Option<Self> {
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
				Bytes::copy_from_slice(key.into_bulk_str()?),
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

		fn parse_entry(chunk: &[Data<'_>]) -> Option<(Field, Value)> {
			// [F, V, ...]
			let mut chunk = chunk
				.into_iter()
				.cloned()
				.filter_map(|d| Some(Bytes::copy_from_slice(d.into_bulk_str()?)));
			Some((chunk.next()?, chunk.next()?))
		}
	}

	#[cfg(test)]
	mod test {
		use bytes::Bytes;
		use resp::Data;

		use crate::model::stream::Id;

		use super::ReadResponse;

		#[test]
		fn stream_read() {
			let data = Data::Array(Some(vec![Data::Array(Some(vec![
				Data::BulkString(Some(b"foo")),
				Data::Array(Some(vec![Data::Array(Some(vec![
					Data::BulkString(Some(b"1-0")),
					Data::Array(Some(vec![
						Data::BulkString(Some(b"abc")),
						Data::BulkString(Some(b"def")),
					])),
				]))])),
			]))]));

			let resp = ReadResponse::try_from_data(data).expect("read data");
			assert_eq!(
				resp[&Bytes::from("foo")][&Id(1, 0)][&Bytes::from("abc")],
				"def"
			);
		}
	}
}
