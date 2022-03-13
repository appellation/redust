use std::{io::Error, net::SocketAddr};

use async_trait::async_trait;
use deadpool::managed;
pub use deadpool::managed::reexports::*;

use crate::connection::Connection;

/// A Deadpool [managed::Manager] for a Redis [Connection].
#[derive(Debug, Clone)]
pub struct Manager {
	addr: SocketAddr,
}

impl Manager {
	pub fn new(addr: SocketAddr) -> Self {
		Self { addr }
	}
}

#[async_trait]
impl managed::Manager for Manager {
	type Type = Connection;
	type Error = Error;

	async fn create(&self) -> Result<Connection, Error> {
		Connection::new(self.addr.clone()).await
	}

	async fn recycle(&self, _: &mut Connection) -> managed::RecycleResult<Error> {
		Ok(())
	}
}

deadpool::managed_reexports!(
	"redis",
	Manager,
	Object,
	Error,
	Error
);
