use std::{
	mem::{replace, MaybeUninit},
	ops::{Deref, DerefMut},
	pin::Pin,
	task::{Context, Poll},
};

use futures::{Sink, SinkExt, Stream, TryStreamExt};
use pin_project_lite::pin_project;
use resp::Data;
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
		cmds: impl Iterator<Item = C>,
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
		self.try_next().await.transpose().unwrap()
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
		self.project().framed.poll_next(cx)
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
			dropped_rx
				.await
				.unwrap()
				.pipeline([["unsubscribe"], ["punsubscribe"]].into_iter())
				.await
				.unwrap();
		});

		Self {
			connection: Some(connection),
			dropped: MaybeUninit::new(dropped_tx),
		}
	}

	pub fn into_connection(mut self) -> Connection {
		self.connection.take().unwrap()
	}
}

impl Deref for PubSub {
	type Target = Connection;

	fn deref(&self) -> &Self::Target {
		self.connection.as_ref().unwrap()
	}
}

impl DerefMut for PubSub {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.connection.as_mut().unwrap()
	}
}

impl Drop for PubSub {
	fn drop(&mut self) {
		let conn = self.connection.take();

		if let Some(conn) = conn {
			// SAFETY: dropped is initialized until this block
			unsafe { replace(&mut self.dropped, MaybeUninit::uninit()).assume_init() }
				.send(conn)
				.unwrap();
		}
	}
}

#[cfg(test)]
mod test {
	use std::env;

	use futures::TryStreamExt;
	use resp::{array, from_data, Data};

	use crate::{model::pubsub, Result};

	use super::Connection;

	fn redis_url() -> String {
		env::var("REDIS_URL").unwrap_or_else(|_| "localhost:6379".to_string())
	}

	#[tokio::test]
	async fn ping() {
		let mut conn = Connection::new(redis_url()).await.expect("new connection");

		let res = conn.cmd(&["PING"]).await.expect("send command");
		assert_eq!(res, Data::SimpleString("PONG".into()));
	}

	#[tokio::test]
	async fn multi_ping() {
		let mut conn = Connection::new(redis_url()).await.expect("new connection");

		let res = conn.cmd(["PING"]).await.expect("send command");
		assert_eq!(res, Data::SimpleString("PONG".into()));

		let res = conn.cmd(["PING", "foobar"]).await.expect("send command");
		assert_eq!(res, Data::bulk_string("foobar"));
	}

	#[tokio::test]
	async fn stream() {
		let mut conn = Connection::new(redis_url()).await.expect("new connection");

		// return value is ID which is dynamic
		let res_id = conn
			.cmd(["XADD", "foo", "*", "foo", "bar"])
			.await
			.expect("send command");

		let res = conn
			.cmd(["XREAD", "STREAMS", "foo", "0-0"])
			.await
			.expect("send command");

		conn.cmd(["DEL", "foo"]).await.expect("delete stream key");

		let expected = array![array![
			Data::BulkString(b"foo"[..].into()),
			array![array![
				res_id,
				array![
					Data::BulkString(b"foo"[..].into()),
					Data::BulkString(b"bar"[..].into())
				]
			]]
		]];

		assert_eq!(res, expected);
	}

	#[tokio::test]
	async fn ping_stream() {
		let mut conn = Connection::new(redis_url()).await.expect("new connection");

		let cmds = [["ping", "foo"], ["ping", "bar"]];
		let res = conn.pipeline(cmds.iter()).await.unwrap();

		assert_eq!(
			res,
			vec![Data::bulk_string(b"foo"), Data::bulk_string(b"bar")]
		);
	}

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
