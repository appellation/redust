use std::borrow::Cow;

use serde::de::{self, Unexpected};

/// Information about a subscription, returned from `(p)(un)subscribe`).
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

/// A pubsub message from Redis. Once a [Connection](crate::Connection) enters pubsub mode, all
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
		struct Visitor;

		impl Visitor {
			fn exp_len<E>(len: usize) -> impl FnOnce() -> E
			where
				E: de::Error,
			{
				move || de::Error::invalid_length(len, &"subscription element")
			}

			fn next_cow<'de, A>(seq: &mut A, len: usize) -> Result<Cow<'de, [u8]>, A::Error>
			where
				A: de::SeqAccess<'de>,
			{
				Ok(Cow::Borrowed(
					seq.next_element()?.ok_or_else(Visitor::exp_len(len))?,
				))
			}
		}

		impl<'de> de::Visitor<'de> for Visitor {
			type Value = Response<'de>;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(formatter, "a list")
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: de::SeqAccess<'de>,
			{
				match seq.next_element()? {
					Some("subscribe" | "psubscribe") => Ok(Response::Subscribe(Subscription {
						name: Visitor::next_cow(&mut seq, 1)?,
						count: seq.next_element()?.ok_or_else(Visitor::exp_len(2))?,
					})),
					Some("unsubscribe" | "punsubscribe") => {
						Ok(Response::Unsubscribe(Subscription {
							name: Visitor::next_cow(&mut seq, 1)?,
							count: seq.next_element()?.ok_or_else(Visitor::exp_len(2))?,
						}))
					}
					Some("message") => Ok(Response::Message(Message {
						pattern: None,
						channel: Visitor::next_cow(&mut seq, 1)?,
						data: seq.next_element()?.ok_or_else(Visitor::exp_len(2))?,
					})),
					Some("pmessage") => Ok(Response::Message(Message {
						pattern: Some(Visitor::next_cow(&mut seq, 1)?),
						channel: Visitor::next_cow(&mut seq, 2)?,
						data: seq.next_element()?.ok_or_else(Visitor::exp_len(3))?,
					})),
					Some(s) => Err(de::Error::invalid_value(
						Unexpected::Str(s),
						&"one of (p)(un)subscribe",
					)),
					None => Err(de::Error::invalid_length(
						0,
						&"an array with at least one element",
					)),
				}
			}
		}

		deserializer.deserialize_seq(Visitor)
	}
}

#[cfg(test)]
mod test {
	use resp::from_bytes;

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
