use std::io::Error;

use async_trait::async_trait;
use deadpool::managed;
use tokio::net::ToSocketAddrs;

use crate::connection::Connection;

pub type Pool<T> = managed::Pool<Manager<T>, Connection>;

/// A Deadpool [managed::Manager] for a Redis [Connection].
#[derive(Debug, Clone)]
pub struct Manager<T> {
	addr: T,
}

#[async_trait]
impl<T> managed::Manager for Manager<T>
where
	T: ToSocketAddrs + Clone + Send + Sync,
{
	type Type = Connection;
	type Error = Error;

	async fn create(&self) -> Result<Connection, Error> {
		Connection::new(self.addr.clone()).await
	}

	async fn recycle(&self, _: &mut Connection) -> managed::RecycleResult<Error> {
		Ok(())
	}
}
