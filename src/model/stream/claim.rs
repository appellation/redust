use serde::{Deserialize, Serialize};

use super::{read::Entries, Id};

/// Response from `XAUTOCLAIM`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoclaimResponse<'a>(
	/// The ID to use in the next `XAUTOCLAIM` call.
	pub Id,
	/// The entries which were claimed in this call.
	#[serde(borrow)]
	pub Entries<'a>,
	/// Entries removed from the PEL. Redis >= 7.0.0
	#[serde(default)]
	pub Vec<Id>,
);

#[cfg(test)]
mod test {
	use resp::from_bytes;

	use crate::model::stream::{
		read::{Entries, Entry, Field, Value},
		Id,
	};

	use super::AutoclaimResponse;

	#[test]
	fn de() {
		let data =
			b"*3\r\n+0-0\r\n*1\r\n*2\r\n+1234-5678\r\n*2\r\n$5\r\nfield\r\n$5\r\nvalue\r\n*0\r\n";

		let (res, rem) = from_bytes::<AutoclaimResponse>(data).unwrap();
		let mut entries = Entries::default();
		let mut entry = Entry::new();

		let field = Field(b"field"[..].into());
		let value = Value(b"value"[..].into());
		entry.insert(field, value);

		entries.0.insert(Id(1234, 5678), entry);

		assert_eq!(res, AutoclaimResponse(Id(0, 0), entries, Vec::new()));
		assert_eq!(rem, []);
	}
}
