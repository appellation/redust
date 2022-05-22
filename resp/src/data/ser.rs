use std::{borrow::Cow, num::TryFromIntError};

use serde::{ser, Serialize};

use crate::{array, Data, Error};

/// Serialize `T` into [Data].
pub fn to_data<T>(value: &T) -> Result<Data<'static>, Error<'static>>
where
	T: Serialize,
{
	value.serialize(Serializer)
}

impl<'a> ser::Serialize for Data<'a> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		match self {
			Data::SimpleString(str) => str.serialize(serializer),
			Data::Integer(i) => i.serialize(serializer),
			Data::BulkString(bytes) => serde_bytes::serialize(bytes, serializer),
			Data::Array(arr) => arr.serialize(serializer),
			Data::Null => serializer.serialize_unit(),
		}
	}
}

struct Serializer;

impl ser::Serializer for Serializer {
	type Ok = Data<'static>;

	type Error = Error<'static>;

	type SerializeSeq = SerializeVec;

	type SerializeTuple = SerializeVec;

	type SerializeTupleStruct = SerializeVec;

	type SerializeTupleVariant = SerializeVariantVec;

	type SerializeMap = SerializeVec;

	type SerializeStruct = SerializeVec;

	type SerializeStructVariant = SerializeVariantVec;

	fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
		Ok(Data::SimpleString(Cow::Owned(v.to_string())))
	}

	fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Integer(v.into()))
	}

	fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Integer(v.into()))
	}

	fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Integer(v.into()))
	}

	fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Integer(v))
	}

	fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Integer(v.into()))
	}

	fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Integer(v.into()))
	}

	fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Integer(v.into()))
	}

	fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Integer(v.try_into().map_err::<Self::Error, _>(
			|e: TryFromIntError| ser::Error::custom(e.to_string()),
		)?))
	}

	fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
		Ok(Data::SimpleString(Cow::Owned(v.to_string())))
	}

	fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
		Ok(Data::SimpleString(Cow::Owned(v.to_string())))
	}

	fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
		Ok(Data::SimpleString(Cow::Owned(v.to_string())))
	}

	fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
		Ok(Data::SimpleString(Cow::Owned(v.to_owned())))
	}

	fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
		Ok(Data::BulkString(Cow::Owned(v.to_vec())))
	}

	fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Null)
	}

	fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
	where
		T: Serialize,
	{
		value.serialize(self)
	}

	fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Null)
	}

	fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
		self.serialize_unit()
	}

	fn serialize_unit_variant(
		self,
		_name: &'static str,
		_variant_index: u32,
		variant: &'static str,
	) -> Result<Self::Ok, Self::Error> {
		self.serialize_str(variant)
	}

	fn serialize_newtype_struct<T: ?Sized>(
		self,
		_name: &'static str,
		value: &T,
	) -> Result<Self::Ok, Self::Error>
	where
		T: Serialize,
	{
		value.serialize(self)
	}

	fn serialize_newtype_variant<T: ?Sized>(
		self,
		_name: &'static str,
		_variant_index: u32,
		variant: &'static str,
		value: &T,
	) -> Result<Self::Ok, Self::Error>
	where
		T: Serialize,
	{
		Ok(array!(Data::simple_string(variant), value.serialize(self)?))
	}

	fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
		Ok(SerializeVec {
			vec: Vec::with_capacity(len.unwrap_or(0)),
		})
	}

	fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
		self.serialize_seq(Some(len))
	}

	fn serialize_tuple_struct(
		self,
		_name: &'static str,
		len: usize,
	) -> Result<Self::SerializeTupleStruct, Self::Error> {
		self.serialize_seq(Some(len))
	}

	fn serialize_tuple_variant(
		self,
		_name: &'static str,
		_variant_index: u32,
		variant: &'static str,
		len: usize,
	) -> Result<Self::SerializeTupleVariant, Self::Error> {
		Ok(SerializeVariantVec {
			name: variant,
			vec: Vec::with_capacity(len),
		})
	}

	fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
		self.serialize_seq(len)
	}

	fn serialize_struct(
		self,
		_name: &'static str,
		len: usize,
	) -> Result<Self::SerializeStruct, Self::Error> {
		self.serialize_map(Some(len))
	}

	fn serialize_struct_variant(
		self,
		_name: &'static str,
		_variant_index: u32,
		variant: &'static str,
		len: usize,
	) -> Result<Self::SerializeStructVariant, Self::Error> {
		Ok(SerializeVariantVec {
			name: variant,
			vec: Vec::with_capacity(len * 2),
		})
	}
}

struct SerializeVec {
	vec: Vec<Data<'static>>,
}

impl ser::SerializeSeq for SerializeVec {
	type Ok = Data<'static>;

	type Error = Error<'static>;

	fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: Serialize,
	{
		self.vec.push(value.serialize(Serializer)?);
		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Array(self.vec))
	}
}

impl ser::SerializeTuple for SerializeVec {
	type Ok = Data<'static>;

	type Error = Error<'static>;

	fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: Serialize,
	{
		self.vec.push(value.serialize(Serializer)?);
		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Array(self.vec))
	}
}

impl ser::SerializeTupleStruct for SerializeVec {
	type Ok = Data<'static>;

	type Error = Error<'static>;

	fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: Serialize,
	{
		self.vec.push(value.serialize(Serializer)?);
		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Array(self.vec))
	}
}

impl ser::SerializeMap for SerializeVec {
	type Ok = Data<'static>;

	type Error = Error<'static>;

	fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
	where
		T: Serialize,
	{
		self.vec.push(key.serialize(Serializer)?);
		Ok(())
	}

	fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: Serialize,
	{
		self.vec.push(value.serialize(Serializer)?);
		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Array(self.vec))
	}
}

impl ser::SerializeStruct for SerializeVec {
	type Ok = Data<'static>;

	type Error = Error<'static>;

	fn serialize_field<T: ?Sized>(
		&mut self,
		key: &'static str,
		value: &T,
	) -> Result<(), Self::Error>
	where
		T: Serialize,
	{
		self.vec.push(Data::simple_string(key));
		self.vec.push(value.serialize(Serializer)?);
		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(Data::Array(self.vec))
	}
}

struct SerializeVariantVec {
	name: &'static str,
	vec: Vec<Data<'static>>,
}

impl ser::SerializeTupleVariant for SerializeVariantVec {
	type Ok = Data<'static>;

	type Error = Error<'static>;

	fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: Serialize,
	{
		self.vec.push(value.serialize(Serializer)?);
		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		let outer = vec![Data::simple_string(self.name), Data::Array(self.vec)];

		Ok(Data::Array(outer))
	}
}

impl ser::SerializeStructVariant for SerializeVariantVec {
	type Ok = Data<'static>;

	type Error = Error<'static>;

	fn serialize_field<T: ?Sized>(
		&mut self,
		key: &'static str,
		value: &T,
	) -> Result<(), Self::Error>
	where
		T: Serialize,
	{
		self.vec.push(Data::simple_string(key));
		self.vec.push(value.serialize(Serializer)?);
		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		let outer = vec![Data::simple_string(self.name), Data::Array(self.vec)];

		Ok(Data::Array(outer))
	}
}
