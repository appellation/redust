use std::{
	io::Result,
	pin::Pin,
	task::{Context, Poll},
};

use bytes::{BufMut, BytesMut};
use futures::{Stream, TryStreamExt};
use pin_project::pin_project;
use resp::{parser::parse, Data, OwnedData};
use tokio::{
	io::AsyncWriteExt,
	net::{
		tcp::{OwnedReadHalf, OwnedWriteHalf},
		TcpStream, ToSocketAddrs,
	},
};
use tokio_util::io::ReaderStream;

/// A TCP connection to a Redis server.
///
/// Responses must be polled, as this type implements [Stream].
///
/// To enter PubSub mode, send the appropriate subscription command using [Self::send_cmd()] and
/// then consume the stream.
#[derive(Debug)]
#[pin_project]
pub struct Connection {
	#[pin]
	read: ReaderStream<OwnedReadHalf>,
	write: OwnedWriteHalf,
	buf: BytesMut,
}

impl Connection {
	/// Connect to the Redis server using the provided `addr`.
	pub async fn new(addr: impl ToSocketAddrs) -> Result<Self> {
		let (read, write) = TcpStream::connect(addr).await?.into_split();
		let buf = BytesMut::new();
		Ok(Self {
			read: ReaderStream::new(read),
			write,
			buf,
		})
	}

	/// Send a command to the server, awaiting a single response.
	pub async fn cmd<'a, C, I>(&mut self, cmd: C) -> Result<OwnedData>
	where
		C: IntoIterator<Item = I>,
		I: Into<&'a [u8]>,
	{
		self.send_cmd(cmd).await?;
		self.try_next().await.transpose().unwrap()
	}

	/// Send a command without waiting for a response.
	pub async fn send_cmd<'a, C, I>(&mut self, cmd: C) -> Result<()>
	where
		C: IntoIterator<Item = I>,
		I: Into<&'a [u8]>,
	{
		let bytes = Vec::from(Data::Array(Some(
			cmd.into_iter()
				.map(|bytes| Data::BulkString(Some(bytes.into())))
				.collect(),
		)));

		self.send_bytes(&*bytes).await
	}

	/// Send raw bytes.
	pub async fn send_bytes(&mut self, body: &[u8]) -> Result<()> {
		self.write.write_all(body).await
	}
}

impl Stream for Connection {
	type Item = Result<OwnedData>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.project();
		let poll = this.read.poll_next(cx);

		match poll {
			Poll::Ready(Some(Ok(bytes))) => {
				this.buf.put(bytes);

				if let Ok((rem, data)) = parse(&this.buf.clone()) {
					this.buf.clear();
					this.buf.put_slice(rem);
					Poll::Ready(Some(Ok(dbg!(data.into()))))
				} else {
					cx.waker().wake_by_ref();
					Poll::Pending
				}
			}
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e.into()))),
			Poll::Pending => Poll::Pending,
		}
	}
}

#[cfg(test)]
mod test {
	use std::env;

	use resp::OwnedData;

	use super::Connection;

	fn redis_url() -> String {
		env::var("REDIS_URL").unwrap_or_else(|_| "localhost:6379".to_string())
	}

	#[tokio::test]
	async fn ping() {
		let mut conn = Connection::new(redis_url()).await.expect("new connection");

		let res = conn.cmd([&b"PING"[..]]).await.expect("send command");
		assert_eq!(res, OwnedData::SimpleString("PONG".to_owned()));
	}

	#[tokio::test]
	async fn multi_ping() {
		let mut conn = Connection::new(redis_url()).await.expect("new connection");

		let res = conn.cmd([&b"PING"[..]]).await.expect("send command");
		assert_eq!(res, OwnedData::SimpleString("PONG".to_owned()));

		let res = conn
			.cmd([&b"PING"[..], &b"foobar"[..]])
			.await
			.expect("send command");
		assert_eq!(res, OwnedData::BulkString(Some(b"foobar".to_vec())));
	}

	#[tokio::test]
	async fn stream() {
		let mut conn = Connection::new(redis_url()).await.expect("new connection");

		// return value is ID which is dynamic
		let res_id = conn
			.cmd([
				"XADD".as_bytes(),
				"foo".as_bytes(),
				"*".as_bytes(),
				"foo".as_bytes(),
				"bar".as_bytes(),
			])
			.await
			.expect("send command");

		let res = conn
			.cmd([
				"XREAD".as_bytes(),
				"STREAMS".as_bytes(),
				"foo".as_bytes(),
				"0-0".as_bytes(),
			])
			.await
			.expect("send command");

		conn.cmd(["DEL".as_bytes(), "foo".as_bytes()])
			.await
			.expect("delete stream key");

		let expected = OwnedData::Array(Some(vec![OwnedData::Array(Some(vec![
			OwnedData::BulkString(Some(b"foo".to_vec())),
			OwnedData::Array(Some(vec![OwnedData::Array(Some(vec![
				res_id,
				OwnedData::Array(Some(vec![
					OwnedData::BulkString(Some(b"foo".to_vec())),
					OwnedData::BulkString(Some(b"bar".to_vec())),
				])),
			]))])),
		]))]));

		assert_eq!(res, expected);
	}
}
