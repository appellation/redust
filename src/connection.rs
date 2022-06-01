use std::{
	convert, io,
	pin::Pin,
	task::{Context, Poll},
};

use futures::{Sink, SinkExt, Stream, TryStreamExt};
use pin_project_lite::pin_project;
use redust_resp::Data;
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio_util::codec::{Decoder, Framed};

use crate::{codec::Codec, Error, Result};

pin_project! {
	/// A TCP connection to a Redis server.
	///
	/// To enter PubSub mode, send the appropriate subscription command using [`send_cmd()`](Self::send_cmd()) and
	/// then consume the stream.
	///
	/// Once connected, clients should say [`hello()`](Self::hello()).
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

	/// Send a [`HELLO`](https://redis.io/commands/hello/) command. If the Redis server doesn't
	/// support `HELLO`, this attempts to authenticate using the
	/// [`AUTH`](https://redis.io/commands/auth/) command.
	pub async fn hello(
		&mut self,
		maybe_username: Option<impl AsRef<[u8]>>,
		maybe_password: Option<impl AsRef<[u8]>>,
	) -> Result<()> {
		let handshake_res = match maybe_password {
			Some(ref password) => {
				self.cmd([
					&b"hello"[..],
					b"2",
					b"auth",
					maybe_username
						.as_ref()
						.map(|u| u.as_ref())
						.unwrap_or(b"default"),
					password.as_ref(),
				])
				.await
			}
			None => self.cmd(["hello", "2"]).await,
		};

		match handshake_res {
			Ok(_) => Ok(()),
			Err(Error::Redis(msg)) if msg == "ERR unknown command 'HELLO'" => {
				if let Some(password) = maybe_password {
					match maybe_username {
						Some(username) => {
							self.cmd([b"auth", username.as_ref(), password.as_ref()])
								.await?
						}
						None => self.cmd([b"auth", password.as_ref()]).await?,
					};
				}

				Ok(())
			}
			Err(e) => Err(e),
		}
	}

	/// Pipeline commands to Redis. This avoids extra syscalls when sending and receiving commands
	/// in bulk.
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
			self.feed(Data::from_bytes_iter(cmd)).await?;
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
		self.send(Data::from_bytes_iter(cmd)).await
	}

	/// Read a single command response.
	pub async fn read_cmd(&mut self) -> Result<Data<'static>> {
		self.try_next()
			.await?
			.ok_or_else(|| Error::Io(io::Error::new(io::ErrorKind::Other, "stream closed")))
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

#[cfg(test)]
pub mod test {
	use std::env;

	use redust_resp::{array, Data};

	use crate::Result;

	use super::Connection;

	pub fn redis_url() -> String {
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

	// This cannot run in CI since debug commands are disabled
	// #[tokio::test]
	// async fn error() -> Result<()> {
	// 	let mut conn = Connection::new(redis_url()).await?;

	// 	let res = conn.cmd(["debug", "error", "uh oh"]).await;
	// 	assert!(matches!(dbg!(res), Err(Error::Redis(msg)) if msg == "uh oh"));

	// 	let res = conn.cmd(["ping"]).await?;
	// 	assert_eq!(res, "PONG");

	// 	Ok(())
	// }

	#[tokio::test]
	async fn hello_no_auth() -> Result<()> {
		let mut conn = Connection::new(redis_url()).await?;
		conn.hello(None::<&str>, None::<&str>).await?;

		Ok(())
	}
}
