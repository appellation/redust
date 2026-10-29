use crate::{
	resp2::parser::{parse_value, Value},
	to_bytes, Data, Error, Needed, ParseError,
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

/// Frame for values which can't contain bulk strings, so are never sliced.
static NO_FRAME: Bytes = Bytes::new();

/// Tokio codec with [`Encoder`] and [`Decoder`] for RESP.
///
/// This codec has a Result as its Item in order to represent transient errors.
///
/// Decoding is incremental: when a frame arrives over several reads, only the newly received bytes
/// are scanned on each call.
#[derive(Debug, Default)]
pub struct Codec {
	/// Length of the complete values at the start of the buffer which have already been scanned.
	scanned: usize,
	/// Number of elements still expected by each open array, innermost last.
	remaining: Vec<usize>,
}

impl Codec {
	/// Scan forward from where the previous call left off until a complete frame has been seen.
	fn scan(&mut self, src: &[u8]) -> Result<(), ParseError> {
		loop {
			let mut input = &src[self.scanned..];
			let value = parse_value(&mut input)?;
			self.scanned = src.len() - input.len();

			if let Value::Array(Some(len @ 1..)) = value {
				self.remaining.push(len);
				continue;
			}

			// A value is complete: count it against its array, closing any arrays it completes.
			loop {
				match self.remaining.last_mut() {
					None => return Ok(()),
					Some(1) => {
						self.remaining.pop();
					}
					Some(n) => {
						*n -= 1;
						break;
					}
				}
			}
		}
	}

	/// Decode an array, scanning forward from where the previous call left off so that only the
	/// newly received bytes are scanned.
	fn decode_array(&mut self, src: &mut BytesMut) -> Result<Data<'static>, Error<'static>> {
		self.scan(src)?;

		let frame = src.split_to(std::mem::take(&mut self.scanned)).freeze();
		parse_owned(&frame, &mut &frame[..])
	}

	/// Decode a new frame. Values other than arrays are complete as soon as they parse, so only
	/// arrays need to be scanned.
	fn decode_value(&mut self, src: &mut BytesMut) -> Result<Data<'static>, Error<'static>> {
		let mut input = &src[..];
		let value = parse_value(&mut input)?;
		let consumed = src.len() - input.len();

		match value {
			Value::Array(Some(len @ 1..)) => {
				self.scanned = consumed;
				self.remaining.push(len);
				self.decode_array(src)
			}
			// Split off only the bytes, which are between the header and the trailing CRLF.
			Value::BulkString(Some(bytes)) => {
				let len = bytes.len();
				src.advance(consumed - len - 2);
				let bytes = src.split_to(len).freeze();
				src.advance(2);
				Ok(Data::BulkString(bytes.into()))
			}
			value => {
				// Redis errors are values too, so they are consumed along with other data.
				let result = into_data(value, &NO_FRAME, &mut input);
				src.advance(consumed);
				result
			}
		}
	}
}

/// Parse a value from `input`, a part of `frame`. Bulk strings share `frame` instead of being
/// copied. An error anywhere in the value fails the whole value.
fn parse_owned(frame: &Bytes, input: &mut &[u8]) -> Result<Data<'static>, Error<'static>> {
	let value = parse_value(input)?;
	into_data(value, frame, input)
}

/// Convert `value` into owned data, parsing the elements of arrays from `input`.
fn into_data(
	value: Value<'_>,
	frame: &Bytes,
	input: &mut &[u8],
) -> Result<Data<'static>, Error<'static>> {
	Ok(match value {
		Value::SimpleString(str) => Data::SimpleString(str.into()),
		Value::Error(msg) => return Err(Error::Redis(msg.to_owned().into())),
		Value::Integer(int) => Data::Integer(int),
		Value::BulkString(Some(bytes)) => Data::BulkString(frame.slice_ref(bytes).into()),
		Value::BulkString(None) | Value::Array(None) => Data::Null,
		Value::Array(Some(len)) => {
			let mut arr = Vec::with_capacity(len);
			for _ in 0..len {
				arr.push(parse_owned(frame, input)?);
			}
			Data::Array(arr)
		}
	})
}

impl Decoder for Codec {
	type Item = Result<Data<'static>, Error<'static>>;

	type Error = Error<'static>;

	fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
		// Arrays may be partially scanned already, so `remaining` is only empty between frames.
		let result = if self.remaining.is_empty() {
			self.decode_value(src)
		} else {
			self.decode_array(src)
		};

		match result {
			Ok(data) => Ok(Some(Ok(data))),
			Err(Error::Parse(ParseError::Incomplete(needed))) => {
				if let Needed::Size(size) = needed {
					src.reserve(size);
				}

				Ok(None)
			}
			Err(err) if err.is_transient() => Ok(Some(Err(err))),
			Err(err) => {
				*self = Self::default();
				Err(err)
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

	use bytes::{BufMut, BytesMut};
	use tokio_util::codec::Decoder;

	use crate::{array, Bulk, Data, Error, ParseError};

	use super::Codec;

	#[tokio::test]
	async fn test_decoder() {
		let bytes = b"+OK\r\n-ERR\r\n";
		let mut stream = FramedRead::new(bytes.as_slice(), Codec::default());

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
		let mut stream = FramedRead::new(rd, Codec::default());

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

	#[tokio::test]
	async fn test_chunked_array_decoder() -> Result<(), crate::Error<'static>> {
		let (tx, rx) = mpsc::unbounded_channel::<Result<&'static [u8], io::Error>>();
		let rd = StreamReader::new(UnboundedReceiverStream::new(rx));
		let mut stream = FramedRead::new(rd, Codec::default());

		spawn(async move {
			let send = |b: &'static [u8]| async {
				tx.send(Ok(b)).unwrap();
				sleep(Duration::from_millis(10)).await;
			};

			send(b"*3\r\n").await;
			send(b"+OK").await;
			send(b"\r\n").await;
			send(b":-999\r\n$2\r\nO").await;
			send(b"K\r\n").await;
			send(b":42\r\n").await;
		});

		let data = dbg!(stream.try_next().await?.unwrap()?);
		match data {
			Data::Array(arr) if arr.len() == 3 => {
				assert!(matches!(&arr[0], Data::SimpleString(v) if v == "OK"));
				assert!(matches!(&arr[1], Data::Integer(-999)));
				assert!(matches!(&arr[2], Data::BulkString(v) if &*v == &&b"OK"[..]));
			}
			_ => panic!("{:?}", data),
		}

		let data = stream.try_next().await?.unwrap()?;
		assert_eq!(data, Data::Integer(42));

		assert!(stream.try_next().await?.is_none());
		Ok(())
	}

	#[test]
	fn test_bytewise_decoder() {
		let input = b"*3\r\n*2\r\n:1\r\n*0\r\n$-1\r\n*1\r\n$3\r\nfoo\r\n+NEXT\r\n";
		let mut codec = Codec::default();
		let mut buf = BytesMut::new();
		let mut decoded = vec![];

		for &b in input {
			buf.put_u8(b);
			if let Some(data) = codec.decode(&mut buf).unwrap() {
				decoded.push(data.unwrap());
			}
		}

		assert_eq!(
			decoded,
			[
				array!(array!(1, array!()), (), array!(b"foo")),
				Data::simple_string("NEXT")
			]
		);
		assert!(buf.is_empty());
	}

	#[test]
	fn test_bytewise_values() {
		let input = b"$3\r\nfoo\r\n$-1\r\n-ERR x\r\n:5\r\n+OK\r\n$0\r\n\r\n";
		let mut codec = Codec::default();
		let mut buf = BytesMut::new();
		let mut decoded = vec![];

		for &b in input {
			buf.put_u8(b);
			if let Some(data) = codec.decode(&mut buf).unwrap() {
				decoded.push(data.map_err(|err| err.to_string()));
			}
		}

		assert_eq!(
			decoded,
			[
				Ok(Data::bulk_string("foo")),
				Ok(Data::Null),
				Err("Redis error: ERR x".to_string()),
				Ok(Data::Integer(5)),
				Ok(Data::simple_string("OK")),
				Ok(Data::bulk_string("")),
			]
		);
		assert!(buf.is_empty());
	}

	#[test]
	fn test_bulk_strings_share_frame() {
		let mut buf = BytesMut::from(&b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"[..]);
		let start = buf.as_ptr();
		let data = Codec::default().decode(&mut buf).unwrap().unwrap().unwrap();

		match data {
			Data::Array(arr) => match &arr[..] {
				[Data::BulkString(foo @ Bulk::Shared(_)), Data::BulkString(bar @ Bulk::Shared(_))] =>
				{
					assert_eq!(foo, b"foo");
					assert_eq!(bar, b"bar");
					assert_eq!(foo.as_ptr(), start.wrapping_add(8));
					assert_eq!(bar.as_ptr(), start.wrapping_add(17));
				}
				other => panic!("{:?}", other),
			},
			other => panic!("{:?}", other),
		}
	}

	#[test]
	fn test_nested_error_consumes_frame() {
		let mut buf = BytesMut::from(&b"*2\r\n-ERR x\r\n:1\r\n+NEXT\r\n"[..]);
		let mut codec = Codec::default();

		let first = codec.decode(&mut buf).unwrap().unwrap();
		assert!(matches!(first, Err(Error::Redis(msg)) if msg == "ERR x"));

		let second = codec.decode(&mut buf).unwrap().unwrap().unwrap();
		assert_eq!(second, Data::simple_string("NEXT"));
		assert!(buf.is_empty());
	}

	#[test]
	fn test_invalid_type() {
		let mut buf = BytesMut::from(&b"*1\r\n!3\r\n"[..]);
		let err = Codec::default().decode(&mut buf).unwrap_err();

		assert!(matches!(err, Error::Parse(ParseError::Type(b'!'))));
	}
}
