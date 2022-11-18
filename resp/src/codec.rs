use crate::{
	de::ReadError,
	from_bytes,
	nom::{Err, Needed},
	to_bytes, Data, Error,
};
use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

/// Tokio codec with [`Encoder`] and [`Decoder`] for RESP.
///
/// This codec has a Result as its Item in order to represent transient errors.
#[derive(Debug)]
pub struct Codec;

impl Decoder for Codec {
	type Item = Result<Data<'static>, Error<'static>>;

	type Error = Error<'static>;

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
					Error::Parse(Err::Incomplete(needed)) => {
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
	type Error = Error<'static>;

	fn encode(&mut self, item: Data<'a>, dst: &mut BytesMut) -> Result<(), Self::Error> {
		to_bytes(&item, dst.writer()).map_err(|e| e.into_owned())?;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use std::{io, time::Duration};

	use futures::{StreamExt, TryStreamExt};
	use tokio::{spawn, sync::mpsc, time::sleep};
	use tokio_stream::wrappers::UnboundedReceiverStream;
	use tokio_util::{codec::FramedRead, io::StreamReader};

	use crate::{Data, Error};

	use super::Codec;

	#[tokio::test]
	async fn test_decoder() {
		let bytes = b"+OK\r\n-ERR\r\n";
		let mut stream = FramedRead::new(bytes.as_slice(), Codec);

		let first = dbg!(stream.next().await);
		assert!(matches!(
			first,
			Some(Ok(Ok(Data::SimpleString(v)))) if v == "OK"
		));

		let second = dbg!(stream.next().await);
		assert!(matches!(second, Some(Ok(Err(Error::Redis(v)))) if v == "ERR"));

		let third = dbg!(stream.next().await);
		assert!(third.is_none());
	}

	#[tokio::test]
	async fn test_chunked_decoder() -> Result<(), crate::Error<'static>> {
		let (tx, rx) = mpsc::unbounded_channel::<Result<&'static [u8], io::Error>>();
		let rd = StreamReader::new(UnboundedReceiverStream::new(rx));
		let mut stream = FramedRead::new(rd, Codec);

		spawn(async move {
			let send = |b: &'static [u8]| async {
				tx.send(Ok(b)).unwrap();
				sleep(Duration::from_millis(10)).await;
			};

			send(b"+OK\r\n").await;
			send(b"+OK").await;
			send(b"\r\n").await;
			send(b"+OK\r\n+O").await;
			send(b"K\r\n").await;
		});

		for _ in 0..4 {
			let data = stream.try_next().await?.unwrap()?;
			assert!(matches!(data, Data::SimpleString(v) if v == "OK"));
		}

		assert!(stream.try_next().await?.is_none());
		Ok(())
	}
}
