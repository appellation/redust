use bytes::{Buf, BufMut, BytesMut};
use resp::{from_bytes, nom::Err, to_bytes, Data};
use tokio_util::codec::{Decoder, Encoder};

use crate::Error;

/// Codec with [Encoder] and [Decoder] for RESP.
#[derive(Debug)]
pub struct Codec;

impl Decoder for Codec {
	type Item = Data<'static>;

	type Error = Error;

	fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
		let start_len = src.len();
		if start_len == 0 {
			return Ok(None);
		}

		match from_bytes::<Data>(src) {
			Ok((data, rem)) => {
				let owned = data.into_owned();

				let end_len = rem.len();
				src.advance(start_len - end_len);

				Ok(Some(owned))
			}
			Err(resp::Error::Parse(Err::Incomplete(_))) => Ok(None),
			Err(e) => Err(e.into_owned()),
		}
	}
}

impl<'a> Encoder<Data<'a>> for Codec {
	type Error = Error;

	fn encode(&mut self, item: Data<'a>, dst: &mut BytesMut) -> Result<(), Self::Error> {
		to_bytes(&item, dst.writer()).map_err(|e| e.into_owned())?;
		Ok(())
	}
}
