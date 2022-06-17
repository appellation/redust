use std::fmt::Debug;

use async_trait::async_trait;
use tracing::instrument;

use crate::{Connection, Error, Result};

use super::Command;

/// A [`HELLO`](https://redis.io/commands/hello/) command. If the Redis server doesn't support
/// `HELLO`, this attempts to authenticate using the [`AUTH`](https://redis.io/commands/auth/)
/// command.
#[derive(Debug, Clone)]
pub struct Hello<U, P> {
	pub username: Option<U>,
	pub password: Option<P>,
}

#[async_trait]
impl<U, P> Command for Hello<U, P>
where
	U: AsRef<[u8]> + Send + Sync + Debug,
	P: AsRef<[u8]> + Send + Sync + Debug,
{
	type Response = ();

	#[instrument]
	async fn run(self, connection: &mut Connection) -> Result<Self::Response> {
		let handshake_res = match self.password {
			Some(ref password) => {
				connection
					.cmd([
						&b"hello"[..],
						b"2",
						b"auth",
						self.username
							.as_ref()
							.map(|u| u.as_ref())
							.unwrap_or(b"default"),
						password.as_ref(),
					])
					.await
			}
			None => connection.cmd(["hello", "2"]).await,
		};

		match handshake_res {
			Ok(_) => Ok(()),
			Err(Error::Redis(msg)) if msg == "ERR unknown command 'HELLO'" => {
				if let Some(password) = self.password {
					match self.username {
						Some(username) => {
							connection
								.cmd([b"auth", username.as_ref(), password.as_ref()])
								.await?
						}
						None => connection.cmd([b"auth", password.as_ref()]).await?,
					};
				}

				Ok(())
			}
			Err(e) => Err(e),
		}
	}
}
