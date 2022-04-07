pub mod tuple_map {
	use std::{collections::HashMap, hash::Hash, marker::PhantomData};

	use serde::{
		de::{SeqAccess, Visitor},
		Deserialize, Deserializer, Serialize, Serializer,
	};

	pub fn serialize<T: ?Sized, S>(bytes: &T, serializer: S) -> Result<S::Ok, S::Error>
	where
		T: Serialize,
		S: Serializer,
	{
		todo!()
	}

	pub fn deserialize<'de, K, V, D>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
	where
		K: Deserialize<'de> + Eq + Hash,
		V: Deserialize<'de>,
		D: Deserializer<'de>,
	{
		struct SeqVisitor<K, V>(PhantomData<K>, PhantomData<V>);

		impl<K, V> Default for SeqVisitor<K, V> {
			fn default() -> Self {
				Self(Default::default(), Default::default())
			}
		}

		impl<'de, K, V> Visitor<'de> for SeqVisitor<K, V>
		where
			K: Deserialize<'de>,
			V: Deserialize<'de>,
		{
			type Value = Vec<(K, V)>;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("list of data")
			}

			fn visit_seq<A>(self, mut visitor: A) -> Result<Self::Value, A::Error>
			where
				A: SeqAccess<'de>,
			{
				let mut items = Vec::with_capacity(visitor.size_hint().unwrap_or(0));

				while let Some(b) = visitor.next_element()? {
					items.push(b);
				}

				Ok(items)
			}
		}

		Ok(deserializer
			.deserialize_seq(SeqVisitor::default())?
			.into_iter()
			.collect())
	}
}
