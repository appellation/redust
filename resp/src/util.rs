pub mod tuple_map {
	use std::{collections::HashMap, hash::Hash, marker::PhantomData};

	use serde::{
		de::{SeqAccess, Visitor},
		Deserialize, Deserializer, Serialize, Serializer, ser::{SerializeSeq},
	};

	pub fn serialize<K, V, S>(value: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
	where
		K: Serialize,
		V: Serialize,
		S: Serializer,
	{
		let mut seq = serializer.serialize_seq(Some(value.len()))?;
		for entry in value {
			seq.serialize_element(&entry)?;
		}
		seq.end()
	}

	pub fn deserialize<'de, K, V, D>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
	where
		K: Deserialize<'de> + Eq + Hash,
		V: Deserialize<'de>,
		D: Deserializer<'de>,
	{
		struct MapVisitor<K, V>(PhantomData<K>, PhantomData<V>);

		impl<K, V> Default for MapVisitor<K, V> {
			fn default() -> Self {
				Self(Default::default(), Default::default())
			}
		}

		impl<'de, K, V> Visitor<'de> for MapVisitor<K, V>
		where
			K: Deserialize<'de> + Eq + Hash,
			V: Deserialize<'de>,
		{
			type Value = HashMap<K, V>;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("list of data")
			}

			fn visit_seq<A>(self, mut visitor: A) -> Result<Self::Value, A::Error>
			where
				A: SeqAccess<'de>,
			{
				let mut items = HashMap::with_capacity(visitor.size_hint().unwrap_or(0));

				while let Some((k, v)) = visitor.next_element()? {
					items.insert(k, v);
				}

				Ok(items)
			}
		}

		deserializer.deserialize_seq(MapVisitor::default())
	}
}
