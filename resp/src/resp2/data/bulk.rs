use std::{
	fmt::{self, Debug},
	hash::{Hash, Hasher},
	ops::Deref,
};

use bytes::Bytes;

/// The contents of a [bulk string](crate::Data::BulkString).
///
/// Like a [`Cow`](std::borrow::Cow), this either borrows its bytes or owns them. Owned bytes are a
/// reference-counted [`Bytes`], so data decoded by the [codec](crate::Codec) shares the buffer it
/// was read into instead of being copied out of it.
#[derive(Clone)]
pub enum Bulk<'a> {
	/// Borrowed bytes.
	Borrowed(&'a [u8]),
	/// Owned, reference-counted bytes.
	Shared(Bytes),
}

impl Bulk<'_> {
	/// Convert these bytes into owned bytes, copying them if they are borrowed.
	pub fn into_owned(self) -> Bulk<'static> {
		Bulk::Shared(self.into_bytes())
	}

	/// Convert these bytes into [`Bytes`], copying them if they are borrowed.
	pub fn into_bytes(self) -> Bytes {
		match self {
			Self::Borrowed(bytes) => Bytes::copy_from_slice(bytes),
			Self::Shared(bytes) => bytes,
		}
	}
}

impl Deref for Bulk<'_> {
	type Target = [u8];

	fn deref(&self) -> &[u8] {
		match self {
			Self::Borrowed(bytes) => bytes,
			Self::Shared(bytes) => bytes,
		}
	}
}

impl AsRef<[u8]> for Bulk<'_> {
	fn as_ref(&self) -> &[u8] {
		self
	}
}

impl Debug for Bulk<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		Debug::fmt(&**self, f)
	}
}

impl Hash for Bulk<'_> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		(**self).hash(state)
	}
}

impl PartialEq for Bulk<'_> {
	fn eq(&self, other: &Self) -> bool {
		**self == **other
	}
}

impl Eq for Bulk<'_> {}

impl PartialEq<[u8]> for Bulk<'_> {
	fn eq(&self, other: &[u8]) -> bool {
		**self == *other
	}
}

impl PartialEq<&[u8]> for Bulk<'_> {
	fn eq(&self, other: &&[u8]) -> bool {
		**self == **other
	}
}

impl<const N: usize> PartialEq<[u8; N]> for Bulk<'_> {
	fn eq(&self, other: &[u8; N]) -> bool {
		**self == *other
	}
}

impl<const N: usize> PartialEq<&[u8; N]> for Bulk<'_> {
	fn eq(&self, other: &&[u8; N]) -> bool {
		**self == **other
	}
}

impl<'a> From<&'a [u8]> for Bulk<'a> {
	fn from(bytes: &'a [u8]) -> Self {
		Self::Borrowed(bytes)
	}
}

impl<'a, const N: usize> From<&'a [u8; N]> for Bulk<'a> {
	fn from(bytes: &'a [u8; N]) -> Self {
		Self::Borrowed(bytes)
	}
}

impl From<Vec<u8>> for Bulk<'_> {
	fn from(bytes: Vec<u8>) -> Self {
		Self::Shared(bytes.into())
	}
}

impl From<Bytes> for Bulk<'_> {
	fn from(bytes: Bytes) -> Self {
		Self::Shared(bytes)
	}
}
