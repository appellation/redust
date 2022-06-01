use std::{borrow::Cow, marker::PhantomData, str::from_utf8};

use serde::de::{self, Unexpected};
use serde_bytes::Bytes;

/// Information about a subscription, returned from `(p)(un)subscribe`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscription<'a> {
	/// The name of this channel.
	pub name: Cow<'a, [u8]>,
	/// The number of remaining subscriptions with this connection.
	pub count: i64,
}

impl<'a> Subscription<'a> {
	/// Whether the connection is still in pubsub mode. When this is false, the connection can be
	/// reused as a normal Redis connection.
	pub fn is_in_pubsub_mode(&self) -> bool {
		self.count > 0
	}
}

/// A message received from a PubSub subscription.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message<'a> {
	/// The pattern which was matched (only for `p(un)subscribe`).
	pub pattern: Option<Cow<'a, [u8]>>,
	/// The channel this message was received from.
	pub channel: Cow<'a, [u8]>,
	/// The data that was published.
	pub data: Cow<'a, [u8]>,
}

/// A pubsub message from Redis. Once a [`Connection`](crate::Connection) enters pubsub mode, all
/// messages can be deserialized into this enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response<'a> {
	/// Subscribed to a channel.
	Subscribe(Subscription<'a>),
	/// Unsubscribed from a channel.
	Unsubscribe(Subscription<'a>),
	/// Received a new message from one of the channels currently subscribed to.
	Message(Message<'a>),
}

impl<'a, 'de: 'a> de::Deserialize<'de> for Response<'a> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		#[derive(Default)]
		struct Visitor<'a>(PhantomData<&'a ()>);

		impl<'a> Visitor<'a> {
			fn exp_len<E>(&self, len: usize) -> impl FnOnce() -> E + '_
			where
				E: de::Error,
			{
				move || de::Error::invalid_length(len, self)
			}

			fn next_cow<'de: 'a, A>(
				&self,
				seq: &mut A,
				len: usize,
			) -> Result<Cow<'a, [u8]>, A::Error>
			where
				A: de::SeqAccess<'de>,
			{
				let bytes = seq
					.next_element::<Cow<Bytes>>()?
					.ok_or_else(self.exp_len(len))?;

				Ok(match bytes {
					Cow::Owned(bytes) => Cow::Owned(bytes.into_vec()),
					Cow::Borrowed(bytes) => Cow::Borrowed(bytes),
				})
			}
		}

		impl<'de: 'a, 'a> de::Visitor<'de> for Visitor<'a> {
			type Value = Response<'a>;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(formatter, "a list")
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: de::SeqAccess<'de>,
			{
				let bytes = seq
					.next_element::<Cow<Bytes>>()?
					.ok_or_else(self.exp_len(0))?;

				let bytes_str = from_utf8(&*bytes)
					.map_err(|_| de::Error::invalid_value(Unexpected::Bytes(&*bytes), &self))?;

				match &*bytes_str {
					"subscribe" | "psubscribe" => Ok(Response::Subscribe(Subscription {
						name: self.next_cow(&mut seq, 1)?,
						count: seq.next_element()?.ok_or_else(self.exp_len(2))?,
					})),
					"unsubscribe" | "punsubscribe" => Ok(Response::Unsubscribe(Subscription {
						name: self.next_cow(&mut seq, 1)?,
						count: seq.next_element()?.ok_or_else(self.exp_len(2))?,
					})),
					"message" => Ok(Response::Message(Message {
						pattern: None,
						channel: self.next_cow(&mut seq, 1)?,
						data: self.next_cow(&mut seq, 2)?,
					})),
					"pmessage" => Ok(Response::Message(Message {
						pattern: Some(self.next_cow(&mut seq, 1)?),
						channel: self.next_cow(&mut seq, 2)?,
						data: self.next_cow(&mut seq, 3)?,
					})),
					s => Err(de::Error::invalid_value(
						Unexpected::Str(s),
						&"one of (p)(un)subscribe",
					)),
				}
			}
		}

		deserializer.deserialize_seq(Visitor::default())
	}
}

#[cfg(test)]
mod test {
	use redust_resp::from_bytes;

	use crate::model::pubsub::Subscription;

	use super::Response;

	#[test]
	fn subscribe() {
		let body = b"*3\r\n$9\r\nsubscribe\r\n$3\r\nfoo\r\n:1\r\n";

		let (res, rem) = from_bytes::<Response>(body).unwrap();
		assert_eq!(
			res,
			Response::Subscribe(Subscription {
				count: 1,
				name: b"foo"[..].into(),
			})
		);
		assert_eq!(rem, []);
	}
}
