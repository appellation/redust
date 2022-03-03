use std::{
	io,
	pin::Pin,
	task::{Context, Poll},
};

use bytes::{BufMut, Bytes, BytesMut};
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

use crate::error::Result;

#[derive(Debug)]
#[pin_project]
pub struct Connection {
	#[pin]
	read: ReaderStream<OwnedReadHalf>,
	write: OwnedWriteHalf,
	buf: BytesMut,
}

impl Connection {
	pub async fn new(addr: impl ToSocketAddrs) -> Result<Self> {
		let (read, write) = TcpStream::connect(addr).await?.into_split();
		let buf = BytesMut::new();
		Ok(Self {
			read: ReaderStream::new(read),
			write,
			buf,
		})
	}

	pub async fn cmd<'a, C, I>(&mut self, cmd: C) -> Result<OwnedData>
	where
		C: IntoIterator<Item = I>,
		I: Into<&'a [u8]>,
	{
		self.send_cmd(cmd).await?;
		self.try_next().await.transpose().unwrap()
	}

	pub async fn send_cmd<'a, C, I>(&mut self, cmd: C) -> io::Result<()>
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

	pub async fn send_bytes(&mut self, body: &[u8]) -> io::Result<()> {
		self.write.write_all(body).await
	}

	pub async fn read_bytes(&mut self) -> io::Result<Bytes> {
		self.read.try_next().await.transpose().unwrap()
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
	use resp::OwnedData;

	use super::Connection;

	#[tokio::test]
	async fn ping() {
		let mut conn = Connection::new("localhost:6379")
			.await
			.expect("new connection");

		let res = conn.cmd([&b"PING"[..]]).await.expect("send command");
		assert_eq!(res, OwnedData::SimpleString("PONG".to_owned()));
	}

	#[tokio::test]
	async fn multi_ping() {
		let mut conn = Connection::new("localhost:6379")
			.await
			.expect("new connection");

		let res = conn.cmd([&b"PING"[..]]).await.expect("send command");
		assert_eq!(res, OwnedData::SimpleString("PONG".to_owned()));

		let res = conn
			.cmd([&b"PING"[..], &b"foobar"[..]])
			.await
			.expect("send command");
		assert_eq!(res, OwnedData::BulkString(Some(b"foobar".to_vec())));
	}
}
