use async_trait::async_trait;
use futures::SinkExt;
use redust_resp::Data;

use crate::{Connection, Result};

/// [Connection](https://redis.io/commands/?group=connection) commands.
pub mod connection;
/// [PubSub](https://redis.io/commands/?group=pubsub) commands.
pub mod pubsub;

/// Types that can be executed on the Redis server as a command.
#[async_trait]
pub trait Command {
	/// The expected response type of this command.
	type Response;

	/// Run the command using the given connection.
	async fn run(self, connection: &mut Connection) -> Result<Self::Response>;
}

#[async_trait]
impl Command for Data<'_> {
	type Response = Data<'static>;

	async fn run(self, connection: &mut Connection) -> Result<Self::Response> {
		connection.send(self).await?;
		connection.read_cmd().await
	}
}
