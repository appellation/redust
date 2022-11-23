use serde::de;

use super::Deserializer;

use super::Error;

pub struct WithLen<'a, 'de: 'a> {
	pub de: &'a mut Deserializer<'de>,
	pub cur: i64,
	pub len: i64,
}

impl<'a, 'de> de::SeqAccess<'de> for WithLen<'a, 'de> {
	type Error = Error<'de>;

	fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
	where
		T: de::DeserializeSeed<'de>,
	{
		if self.cur == self.len {
			Ok(None)
		} else {
			self.cur += 1;
			seed.deserialize(&mut *self.de).map(Some)
		}
	}

	fn size_hint(&self) -> Option<usize> {
		(self.len - self.cur).try_into().ok()
	}
}

impl<'a, 'de> de::MapAccess<'de> for WithLen<'a, 'de> {
	type Error = Error<'de>;

	fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
	where
		K: de::DeserializeSeed<'de>,
	{
		if self.cur == self.len {
			Ok(None)
		} else {
			self.cur += 1;
			seed.deserialize(&mut *self.de).map(Some)
		}
	}

	fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
	where
		V: de::DeserializeSeed<'de>,
	{
		seed.deserialize(&mut *self.de)
	}

	fn size_hint(&self) -> Option<usize> {
		(self.len - self.cur).try_into().ok()
	}
}

pub struct Enum<'a, 'de: 'a> {
	pub de: &'a mut Deserializer<'de>,
}

impl<'de, 'a> de::EnumAccess<'de> for Enum<'a, 'de> {
	type Error = Error<'de>;
	type Variant = Self;

	fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
	where
		V: de::DeserializeSeed<'de>,
	{
		let val = seed.deserialize(&mut *self.de)?;
		Ok((val, self))
	}
}

impl<'de, 'a> de::VariantAccess<'de> for Enum<'a, 'de> {
	type Error = Error<'de>;

	fn unit_variant(self) -> Result<(), Self::Error> {
		Ok(())
	}

	fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
	where
		T: de::DeserializeSeed<'de>,
	{
		seed.deserialize(self.de)
	}

	fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: de::Visitor<'de>,
	{
		de::Deserializer::deserialize_tuple(self.de, len, visitor)
	}

	fn struct_variant<V>(
		self,
		_fields: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: de::Visitor<'de>,
	{
		de::Deserializer::deserialize_map(self.de, visitor)
	}
}
