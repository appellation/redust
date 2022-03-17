use resp::Data;

use super::{
	super::error::{Error, Result},
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

impl<'a> TryFrom<Data<'a>> for AutoclaimResponse<'a> {
	type Error = Error;

	fn try_from(value: Data<'a>) -> Result<Self, Self::Error> {
		let mut iter = value.into_array()?.into_iter();

		let next = Id::try_from(iter.next().ok_or(Error::MissingElement(0))?)?;

		let claimed = iter
			.next()
			.ok_or(Error::MissingElement(1))?
			.into_array()?
			.into_iter()
			.flat_map(ReadResponse::parse_entries)
			.collect();

		let deleted = iter
			.next()
			.map(|d| Ok::<_, Error>(d.into_array()?.into_iter().flat_map(Id::try_from).collect()))
			.transpose()?
			.unwrap_or_default();

		Ok(Self {
			next,
			claimed,
			deleted,
		})
	}
}
