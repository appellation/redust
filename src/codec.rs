use bytes::{Buf, BufMut, BytesMut};
use redust_resp::{
	de::ReadError,
	from_bytes,
	nom::{Err, Needed},
	to_bytes, Data,
};
use tokio_util::codec::{Decoder, Encoder};

use crate::Error;

/// Tokio codec with [`Encoder`] and [`Decoder`] for RESP.
///
/// This codec has a Result as its Item in order to represent transient errors.
#[derive(Debug)]
pub struct Codec;

impl Decoder for Codec {
	type Item = Result<Data<'static>, Error>;

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

				Ok(Some(Ok(owned)))
			}
			Err(ReadError { data, remaining }) => {
				let end_len = remaining.len();

				let result = match data {
					redust_resp::Error::Parse(Err::Incomplete(needed)) => {
						if let Needed::Size(size) = needed {
							src.reserve(size.into());
						}

						Ok(None)
					}
					other if other.is_transient() => Ok(Some(Err(other.into_owned()))),
					other => Err(other.into_owned()),
				};

				src.advance(start_len - end_len);
				result
			}
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
