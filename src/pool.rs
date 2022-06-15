use std::sync::{
	atomic::{AtomicUsize, Ordering},
	Arc,
};

use async_trait::async_trait;
use deadpool::managed::{self, RecycleError, RecycleResult};
use redust_resp::Data;
use tokio::net::ToSocketAddrs;

use crate::{connection::Connection, Error};

pub use deadpool;

/// A deadpool [`Manager`](managed::Manager) for a Redis [`Connection`].
#[derive(Debug, Clone)]
pub struct Manager<A> {
	addr: A,
	ping_number: Arc<AtomicUsize>,
}

impl<A> Manager<A> {
	/// Make a new manager.
	pub fn new(addr: A) -> Self {
		Self {
			addr,
			ping_number: Arc::default(),
		}
	}
}

#[async_trait]
impl<A> managed::Manager for Manager<A>
where
	A: ToSocketAddrs + Clone + Send + Sync,
{
	type Type = Connection;
	type Error = Error;

	async fn create(&self) -> Result<Self::Type, Self::Error> {
		Ok(Connection::new(self.addr.clone()).await?)
	}

	async fn recycle(&self, conn: &mut Self::Type) -> RecycleResult<Self::Error> {
		if conn.is_dead() {
			return Err(RecycleError::StaticMessage("connection is dead"));
		}

		let ping_number = self.ping_number.fetch_add(1, Ordering::Relaxed).to_string();
		if conn.cmd(["PING", &ping_number]).await? == Data::bulk_string(ping_number.as_bytes()) {
			Ok(())
		} else {
			Err(RecycleError::StaticMessage("invalid PING response"))
		}
	}
}

pub type Pool<A> = managed::Pool<Manager<A>>;
pub type PoolBuilder<A> = managed::PoolBuilder<Manager<A>>;
pub type BuildError = managed::BuildError<Error>;
pub type PoolError = managed::PoolError<Error>;
pub type Object<A> = managed::Object<Manager<A>>;
pub type Hook<A> = managed::Hook<Manager<A>>;
pub type HookError = managed::HookError<Error>;
pub type HookErrorCause = managed::HookErrorCause<Error>;

#[cfg(test)]
mod test {
	use futures::Future;

	use crate::Result;

	use super::{Manager, Pool};

	fn assert_static<F>(_block: F)
	where
		F: Future + Send + 'static,
	{
	}

	#[tokio::test]
	async fn static_pool() -> Result<()> {
		let manager = Manager::new("localhost:6379");
		let pool = Pool::builder(manager).build().unwrap();

		assert_static(async move {
			let _ = pool.get().await;
		});

		Ok(())
	}
}
