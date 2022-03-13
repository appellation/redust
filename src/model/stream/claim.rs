use resp::Data;

use super::{
	read::{Entries, ReadResponse},
	Id,
};

/// Response from `XAUTOCLAIM`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoclaimResponse<'a> {
	/// The ID to use in the next `XAUTOCLAIM` call.
	pub next: Id,
	/// The entries which were claimed in this call.
	pub claimed: Entries<'a>,
	/// Entries removed from the PEL. Redis >= 7.0.0
	pub deleted: Vec<Id>,
}

impl<'a> AutoclaimResponse<'a> {
	pub fn try_from_data(data: Data<'a>) -> Option<Self> {
		match data {
			Data::Array(Some(values)) => {
				let mut iter = values.into_iter();

				Some(Self {
					next: Id::try_from_data(iter.next()?)?,
					claimed: iter
						.next()?
						.into_array()?
						.into_iter()
						.flat_map(ReadResponse::parse_entries)
						.collect(),
					deleted: iter
						.next()
						.and_then(|d| {
							Some(
								d.into_array()?
									.into_iter()
									.flat_map(Id::try_from_data)
									.collect(),
							)
						})
						.unwrap_or_default(),
				})
			}
			_ => None,
		}
	}
}
