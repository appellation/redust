use std::io;

use async_trait::async_trait;
use bb8::ManageConnection;
use tokio::net::ToSocketAddrs;

use crate::{connection::Connection, Error};

pub use bb8;

/// A bb8 [`ManageConnection`] for a Redis [`Connection`].
#[derive(Debug, Clone)]
pub struct Manager<A> {
	addr: A,
}

impl<A> Manager<A> {
	/// Make a new manager.
	pub fn new(addr: A) -> Self {
		Self { addr }
	}
}

#[async_trait]
impl<A> ManageConnection for Manager<A>
where
	A: 'static + ToSocketAddrs + Clone + Send + Sync,
{
	type Connection = Connection;
	type Error = Error;

	async fn connect(&self) -> Result<Self::Connection, Self::Error> {
		Ok(Connection::new(self.addr.clone()).await?)
	}

	async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
		if conn.cmd(["PING"]).await? == "PONG" {
			Ok(())
		} else {
			Err(Error::Io(io::Error::new(
				io::ErrorKind::Other,
				"ping request",
			)))
		}
	}

	fn has_broken(&self, conn: &mut Self::Connection) -> bool {
		conn.is_dead()
	}
}
