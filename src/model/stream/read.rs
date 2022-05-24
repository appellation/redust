use std::{borrow::Cow, collections::HashMap};

use serde::{Deserialize, Serialize};

use super::Id;

/// A stream key in the Redis keyspace.
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Key<'a>(#[serde(borrow, with = "serde_bytes")] pub Cow<'a, [u8]>);

/// A field from a stream, associated to a [Value].
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Field<'a>(#[serde(borrow, with = "serde_bytes")] pub Cow<'a, [u8]>);

/// A value from a stream, keyed by a [Field].
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Value<'a>(#[serde(borrow, with = "serde_bytes")] pub Cow<'a, [u8]>);

/// All entries in a stream, belonging to a [Key].
#[derive(Debug, Default, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Entries<'a>(
	#[serde(borrow, with = "redust_resp::util::tuple_map")] pub HashMap<Id, Entry<'a>>,
);

pub type Entry<'a> = HashMap<Field<'a>, Value<'a>>;

#[derive(Debug, Default, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReadResponse<'a>(
	#[serde(borrow, with = "redust_resp::util::tuple_map")] pub HashMap<Key<'a>, Entries<'a>>,
);

#[cfg(test)]
mod test {
	use std::borrow::Cow;

	use redust_resp::{array, from_data, Data};

	use crate::model::stream::{
		read::{Field, Key, Value},
		Id,
	};

	use super::ReadResponse;

	#[test]
	fn stream_read() {
		let data = array![array![
			Data::BulkString(b"foo"[..].into()),
			array![array![
				Data::BulkString(b"1-0"[..].into()),
				array![
					Data::BulkString(b"abc"[..].into()),
					Data::BulkString(b"def"[..].into())
				]
			]]
		]];

		let resp: ReadResponse = from_data(data).expect("read data");
		assert_eq!(
			resp.0[&Key(b"foo"[..].into())].0[&Id(1, 0)][&Field(b"abc"[..].into())],
			Value(Cow::from(&b"def"[..]))
		);
	}
}
