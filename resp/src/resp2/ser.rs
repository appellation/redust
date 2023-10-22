use std::io::Write;

use serde::Serialize;

use crate::Result;

mod serializer;

pub use serializer::*;

/// Serialize to a writer using RESP.
#[tracing::instrument(level = "trace", err, skip_all)]
pub fn to_bytes<T, W>(value: &T, output: W) -> Result<()>
where
	T: Serialize,
	W: Write,
{
	let mut serializer = Serializer {
		output,
		options: Options::default(),
	};
	value.serialize(&mut serializer)?;
	Ok(())
}

#[cfg(test)]
mod test {
	use bytes::{BufMut, BytesMut};

	use crate::Data;

	use super::to_bytes;

	#[test]
	fn ser_str() {
		let data = Data::simple_string("OK");
		let mut writer = BytesMut::new().writer();
		to_bytes(&data, &mut writer).unwrap();

		assert_eq!(writer.get_ref(), &b"+OK\r\n"[..]);
	}
}
