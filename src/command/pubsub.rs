use async_trait::async_trait;
use futures::{future::ready, TryStreamExt};
use redust_resp::{from_data, Data};
use tracing::instrument;

use crate::{model::pubsub::Response, Connection, Result};

use super::Command;

/// Unsubscribe from all channels and return this connection to normal mode.
#[derive(Debug)]
pub struct Unsubscribe;

#[async_trait]
impl Command for Unsubscribe {
	type Response = Vec<Data<'static>>;

	#[instrument(ret, level = "info")]
	async fn run(self, connection: &mut Connection) -> Result<Self::Response> {
		connection
			.pipeline([["unsubscribe"], ["punsubscribe"]])
			.await?;

		connection
			.try_take_while(|data| {
				ready(from_data::<Response>(data.clone()).map(
					|response| matches!(response, Response::Unsubscribe(sub) if sub.is_in_pubsub_mode()),
				))
			})
			.try_collect()
			.await
	}
}
