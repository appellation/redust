use serde::ser;

use crate::Data;

impl<'a> ser::Serialize for Data<'a> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		match self {
			Data::SimpleString(str) => str.serialize(serializer),
			Data::Integer(i) => i.serialize(serializer),
			Data::BulkString(bytes) => serde_bytes::serialize(bytes, serializer),
			Data::Array(arr) => arr.serialize(serializer),
			Data::Null => serializer.serialize_unit(),
		}
	}
}
