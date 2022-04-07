use std::{
	borrow::Cow,
	collections::HashMap,
};

use serde::{Deserialize, Serialize};

use super::Id;

/// A stream key in the Redis keyspace.
pub type Key<'a> = Cow<'a, [u8]>;
/// A field from a stream, associated to a [Value].
pub type Field<'a> = Cow<'a, [u8]>;
/// A value from a stream, keyed by a [Field].
pub type Value<'a> = Cow<'a, [u8]>;

/// All entries in a stream, belonging to a [Key].
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Entries<'a>(#[serde(with = "resp::util::tuple_map")] pub HashMap<Id, Entry<'a>>);

pub type Entry<'a> = HashMap<Field<'a>, Value<'a>>;

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ReadResponse<'a>(
	#[serde(with = "resp::util::tuple_map")] pub HashMap<Key<'a>, Entries<'a>>,
);

#[cfg(test)]
mod test {
	// use std::borrow::Cow;

	// use crate::model::stream::Id;

	// use super::ReadResponse;

	// #[test]
	// fn stream_read() {
	// 	let data = array![array![
	// 		Data::BulkString(Some(b"foo"[..].into())),
	// 		array![array![
	// 			Data::BulkString(Some(b"1-0"[..].into())),
	// 			array![
	// 				Data::BulkString(Some(b"abc"[..].into())),
	// 				Data::BulkString(Some(b"def"[..].into()))
	// 			]
	// 		]]
	// 	]];

	// 	let resp = ReadResponse::try_from(data).expect("read data");
	// 	assert_eq!(resp["foo"][&Id(1, 0)]["abc"], Cow::from(&b"def"[..]));
	// }
}
