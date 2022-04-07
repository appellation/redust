use serde::{Deserialize, Serialize};

use super::{read::Entries, Id};

/// Response from `XAUTOCLAIM`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoclaimResponse<'a>(
	/// The ID to use in the next `XAUTOCLAIM` call.
	pub Id,
	/// The entries which were claimed in this call.
	pub Entries<'a>,
	/// Entries removed from the PEL. Redis >= 7.0.0
	#[serde(default)]
	pub Vec<Id>,
);

#[cfg(test)]
mod test {
	use resp::from_bytes;

	use crate::model::stream::{read::Entries, Id};

	use super::AutoclaimResponse;

	#[test]
	fn de() {
		let data = b"*3\r\n+0-0\r\n*1\r\n*2\r\n+1234-5678\r\n*2\r\n+field\r\n+value\r\n*0\r\n";

		let (res, rem) = from_bytes::<AutoclaimResponse>(data).unwrap();

		assert_eq!(res, AutoclaimResponse(Id(0, 0), Entries::default(), Vec::new()));
		assert_eq!(rem, []);
	}
}
