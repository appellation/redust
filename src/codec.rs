use bytes::{Buf, BytesMut};
use resp::{nom::Err, parser::parse, Data};
use tokio_util::codec::{Decoder, Encoder};

use crate::error::Error;

/// Codec with [Encoder] and [Decoder] for RESP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Codec;

impl Decoder for Codec {
	type Item = Data<'static>;

	type Error = Error;

	fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
		let start_len = src.len();
		match parse(&src.clone()) {
			Ok((rem, data)) => {
				let end_len = rem.len();
				src.advance(start_len - end_len);

				Ok(Some(data.into_owned()))
			}
			Err(Err::Incomplete(_)) => Ok(None),
			_ => Err(Error::Parse),
		}
	}
}

impl<'a> Encoder<Data<'a>> for Codec {
	type Error = Error;

	fn encode(&mut self, item: Data<'a>, dst: &mut BytesMut) -> Result<(), Self::Error> {
		Ok(item.to_bytes(dst))
	}
}
