use bytes::{Buf, BytesMut};
use resp::{nom::Err, parser::parse, Data, OwnedData};
use tokio_util::codec::{Decoder, Encoder};

use crate::error::Error;

/// Codec to encode & decode RESP.
#[derive(Debug, Clone)]
pub struct Codec;

impl Decoder for Codec {
	type Item = OwnedData;

	type Error = Error;

	fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
		let start_len = src.len();
		match parse(&src.clone()) {
			Ok((rem, data)) => {
				let end_len = rem.len();
				src.advance(start_len - end_len);

				Ok(Some(data.into()))
			}
			Err(Err::Incomplete(_)) => Ok(None),
			_ => Err(Error::Parse),
		}
	}
}

impl<'a> Encoder<Data<'a>> for Codec {
	type Error = Error;

	fn encode(&mut self, item: Data<'a>, dst: &mut BytesMut) -> Result<(), Self::Error> {
		Ok(dst.extend_from_slice(&Vec::from(item)))
	}
}
