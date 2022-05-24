use std::{
	convert, io,
	mem::{replace, MaybeUninit},
	ops::{Deref, DerefMut},
	pin::Pin,
	task::{Context, Poll},
};

use futures::{Sink, SinkExt, Stream, TryStreamExt};
use pin_project_lite::pin_project;
use redust_resp::Data;
use tokio::{
	net::{TcpStream, ToSocketAddrs},
	spawn,
	sync::oneshot,
};
use tokio_util::codec::{Decoder, Framed};

use crate::{codec::Codec, Error, Result};

pin_project! {
	/// A TCP connection to a Redis server.
	///
	/// To enter PubSub mode, send the appropriate subscription command using [send_cmd()](Self::send_cmd()) and
	/// then consume the stream.
	#[derive(Debug)]
	pub struct Connection {
		#[pin]
		framed: Framed<TcpStream, Codec>,
	}
}

impl Connection {
	/// Connect to the Redis server using the provided `addr`.
	pub async fn new(addr: impl ToSocketAddrs) -> Result<Self, std::io::Error> {
		let stream = TcpStream::connect(addr).await?;
		let framed = Codec.framed(stream);
		Ok(Self { framed })
	}

	pub fn into_pubsub(self) -> PubSub {
		PubSub::new(self)
	}

	pub async fn pipeline<'a, C, I>(
		&mut self,
		cmds: impl IntoIterator<Item = C>,
	) -> Result<Vec<Data<'static>>>
	where
		C: IntoIterator<Item = &'a I>,
		I: 'a + AsRef<[u8]> + ?Sized,
	{
		let mut len = 0;
		for cmd in cmds {
			self.feed(Self::make_cmd(cmd)).await?;
			len += 1;
		}

		if len > 0 {
			self.flush().await?;

			let mut results = Vec::with_capacity(len);
			for _ in 0..len {
				let data = self.read_cmd().await?;
				results.push(data);
			}

			Ok(results)
		} else {
			Ok(vec![])
		}
	}

	/// Send a command to the server, awaiting a single response.
	pub async fn cmd<'a, C, I>(&mut self, cmd: C) -> Result<Data<'static>>
	where
		C: IntoIterator<Item = &'a I>,
		I: 'a + AsRef<[u8]> + ?Sized,
	{
		self.send_cmd(cmd).await?;
		self.read_cmd().await
	}

	/// Send a command without waiting for a response.
	pub async fn send_cmd<'a, C, I>(&mut self, cmd: C) -> Result<()>
	where
		C: IntoIterator<Item = &'a I>,
		I: 'a + AsRef<[u8]> + ?Sized,
	{
		self.send(Self::make_cmd(cmd)).await
	}

	/// Read a single command response.
	pub async fn read_cmd(&mut self) -> Result<Data<'static>> {
		self.try_next()
			.await?
			.ok_or_else(|| Error::Io(io::Error::new(io::ErrorKind::Other, "stream closed")))
	}

	fn make_cmd<'a, C, I>(cmd: C) -> Data<'a>
	where
		C: IntoIterator<Item = &'a I>,
		I: 'a + AsRef<[u8]> + ?Sized,
	{
		Data::Array(
			cmd.into_iter()
				.map(|bytes| Data::BulkString(bytes.as_ref().into()))
				.collect(),
		)
	}
}

impl Stream for Connection {
	type Item = Result<Data<'static>>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.project()
			.framed
			.poll_next(cx)
			.map(|res| res.map(|item| item.and_then(convert::identity)))
	}
}

impl Sink<Data<'_>> for Connection {
	type Error = Error;

	fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.project().framed.poll_ready(cx)
	}

	fn start_send(self: Pin<&mut Self>, item: Data<'_>) -> Result<(), Self::Error> {
		self.project().framed.start_send(item)
	}

	fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.project().framed.poll_flush(cx)
	}

	fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.project().framed.poll_close(cx)
	}
}

/// A wrapper around a [Connection] to make PubSub easier. This is a convenience RAII pointer which
/// automatically unsubscribes the connection from all subscriptions when dropped.
#[derive(Debug)]
pub struct PubSub {
	connection: Option<Connection>,
	dropped: MaybeUninit<oneshot::Sender<Connection>>,
}

impl PubSub {
	fn new(connection: Connection) -> Self {
		let (dropped_tx, dropped_rx) = oneshot::channel::<Connection>();

		spawn(async move {
			if let Ok(mut conn) = dropped_rx.await {
				conn.pipeline([["unsubscribe"], ["punsubscribe"]])
					.await
					.unwrap();
			}
		});

		Self {
			connection: Some(connection),
			dropped: MaybeUninit::new(dropped_tx),
		}
	}

	pub fn into_connection(mut self) -> Connection {
		// SAFETY: connection is initialized until this OR dropped
		unsafe { self.connection.take().unwrap_unchecked() }
	}
}

impl Deref for PubSub {
	type Target = Connection;

	fn deref(&self) -> &Self::Target {
		// SAFETY: connection is initialized until dropped
		unsafe { self.connection.as_ref().unwrap_unchecked() }
	}
}

impl DerefMut for PubSub {
	fn deref_mut(&mut self) -> &mut Self::Target {
		// SAFETY: connection is initialized until dropped
		unsafe { self.connection.as_mut().unwrap_unchecked() }
	}
}

impl Drop for PubSub {
	fn drop(&mut self) {
		let conn = self.connection.take();
		// SAFETY: dropped is initialized until this block
		let sender = unsafe { replace(&mut self.dropped, MaybeUninit::uninit()).assume_init() };

		if let Some(conn) = conn {
			sender.send(conn).unwrap();
		}
	}
}

#[cfg(test)]
mod test {
	use std::env;

	#[cfg(feature = "model")]
	use futures::TryStreamExt;
	use redust_resp::{array, from_data, Data};

	#[cfg(feature = "model")]
	use crate::model::pubsub;
	use crate::Result;

	use super::Connection;

	fn redis_url() -> String {
		env::var("REDIS_URL").unwrap_or_else(|_| "localhost:6379".to_string())
	}

	#[tokio::test]
	async fn ping() -> Result<()> {
		let mut conn = Connection::new(redis_url()).await?;

		let res = conn.cmd(["PING"]).await?;
		assert_eq!(res, "PONG");

		Ok(())
	}

	#[tokio::test]
	async fn multi_ping() -> Result<()> {
		let mut conn = Connection::new(redis_url()).await?;

		let res = conn.cmd(["PING"]).await?;
		assert_eq!(res, "PONG");

		let res = conn.cmd(["PING", "foobar"]).await?;
		assert_eq!(res, b"foobar");

		Ok(())
	}

	#[tokio::test]
	async fn stream() -> Result<()> {
		let mut conn = Connection::new(redis_url()).await?;

		// return value is ID which is dynamic
		let res_id = conn.cmd(["XADD", "foo1", "*", "foo", "bar"]).await?;

		let res = conn.cmd(["XREAD", "STREAMS", "foo1", "0-0"]).await?;

		conn.cmd(["DEL", "foo1"]).await?;

		let expected = array![array![
			b"foo1",
			array![array![res_id, array![b"foo", b"bar"]]]
		]];

		assert_eq!(res, expected);
		Ok(())
	}

	#[tokio::test]
	async fn ping_stream() -> Result<()> {
		let mut conn = Connection::new(redis_url()).await?;

		let cmds = [["ping", "foo"], ["ping", "bar"]];
		let res = conn.pipeline(cmds.iter()).await?;

		assert_eq!(
			res,
			vec![Data::bulk_string(b"foo"), Data::bulk_string(b"bar")]
		);

		Ok(())
	}

	#[cfg(feature = "model")]
	#[tokio::test]
	async fn pubsub() -> Result<()> {
		let mut conn = Connection::new(redis_url()).await?.into_pubsub();

		let cmds = ["subscribe", "foo"];
		let res = from_data::<pubsub::Response>(conn.cmd(cmds).await?)?;

		assert_eq!(
			res,
			pubsub::Response::Subscribe(pubsub::Subscription {
				count: 1,
				name: b"foo".as_slice().into()
			})
		);

		let mut conn2 = Connection::new(redis_url()).await?;
		conn2.cmd(["publish", "foo", "bar"]).await?;

		let res = from_data::<pubsub::Response>(conn.try_next().await?.expect("response"))?;

		assert_eq!(
			res,
			pubsub::Response::Message(pubsub::Message {
				pattern: None,
				channel: b"foo"[..].into(),
				data: b"bar"[..].into(),
			})
		);
		Ok(())
	}
}
