use std::{fmt::Display, io::Write};

use serde::ser;

use crate::Error;

#[derive(Debug, Clone)]
pub enum NullType {
	BulkString,
	Array,
}

impl Default for NullType {
	fn default() -> Self {
		Self::BulkString
	}
}

#[derive(Debug, Clone, Default)]
pub struct Options {
	/// The type to use for serializing missing Optional values.
	null_type: NullType,
}

#[derive(Default)]
pub struct Serializer<W> {
	pub output: W,
	pub options: Options,
}

impl<W> Serializer<W>
where
	W: Write,
{
	fn serialize_int<T>(&mut self, v: T) -> crate::Result<'static, ()>
	where
		T: Display,
	{
		Ok(write!(self.output, ":{}\r\n", v)?)
	}
}

impl<'a, W> ser::Serializer for &'a mut Serializer<W>
where
	W: Write,
{
	type Ok = ();

	type Error = Error<'static>;

	type SerializeSeq = Self;

	type SerializeTuple = Self;

	type SerializeTupleStruct = Self;

	type SerializeTupleVariant = Self;

	type SerializeMap = Self;

	type SerializeStruct = Self;

	type SerializeStructVariant = Self;

	fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
		self.serialize_str(&v.to_string())
	}

	fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
		self.serialize_int(v)
	}

	fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
		self.serialize_int(v)
	}

	fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
		self.serialize_int(v)
	}

	fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
		self.serialize_int(v)
	}

	fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
		self.serialize_int(v)
	}

	fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
		self.serialize_int(v)
	}

	fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
		self.serialize_int(v)
	}

	fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
		self.serialize_int(v)
	}

	fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
		self.serialize_str(&v.to_string())
	}

	fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
		self.serialize_str(&v.to_string())
	}

	fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
		self.serialize_str(&v.to_string())
	}

	fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
		write!(self.output, "+{}\r\n", v)?;
		Ok(())
	}

	fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
		write!(self.output, "${}\r\n", v.len())?;
		self.output.write_all(v)?;
		self.output.write_all(b"\r\n")?;

		Ok(())
	}

	fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
		self.serialize_unit()
	}

	fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
	where
		T: serde::Serialize,
	{
		value.serialize(self)
	}

	fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
		match self.options.null_type {
			NullType::Array => self.output.write_all(b"*-1\r\n\r\n")?,
			NullType::BulkString => self.output.write_all(b"$-1\r\n\r\n")?,
		}

		Ok(())
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
		T: serde::Serialize,
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
		T: serde::Serialize,
	{
		write!(self.output, "*2\r\n{}\r\n", variant)?;
		value.serialize(self)?;

		Ok(())
	}

	fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
		let len = len.ok_or::<Self::Error>(ser::Error::custom("sequence length required"))?;
		write!(self.output, "*{}\r\n", len)?;

		Ok(self)
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
		write!(self.output, "*2\r\n{}\r\n*{}\r\n", variant, len)?;

		Ok(self)
	}

	fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
		self.serialize_seq(len.map(|l| l * 2))
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
		write!(self.output, "*2\r\n{}\r\n*{}\r\n", variant, len * 2)?;

		Ok(self)
	}
}

impl<'a, W> ser::SerializeSeq for &'a mut Serializer<W>
where
	W: Write,
{
	type Ok = ();

	type Error = Error<'static>;

	fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: serde::Serialize,
	{
		value.serialize(&mut **self)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}

impl<'a, W> ser::SerializeTuple for &'a mut Serializer<W>
where
	W: Write,
{
	type Ok = ();

	type Error = Error<'static>;

	fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: serde::Serialize,
	{
		value.serialize(&mut **self)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}

impl<'a, W> ser::SerializeTupleStruct for &'a mut Serializer<W>
where
	W: Write,
{
	type Ok = ();

	type Error = Error<'static>;

	fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: serde::Serialize,
	{
		value.serialize(&mut **self)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}

impl<'a, W> ser::SerializeTupleVariant for &'a mut Serializer<W>
where
	W: Write,
{
	type Ok = ();

	type Error = Error<'static>;

	fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: serde::Serialize,
	{
		value.serialize(&mut **self)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}

impl<'a, W> ser::SerializeMap for &'a mut Serializer<W>
where
	W: Write,
{
	type Ok = ();

	type Error = Error<'static>;

	fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
	where
		T: serde::Serialize,
	{
		key.serialize(&mut **self)
	}

	fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
	where
		T: serde::Serialize,
	{
		value.serialize(&mut **self)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}

impl<'a, W> ser::SerializeStruct for &'a mut Serializer<W>
where
	W: Write,
{
	type Ok = ();

	type Error = Error<'static>;

	fn serialize_field<T: ?Sized>(
		&mut self,
		key: &'static str,
		value: &T,
	) -> Result<(), Self::Error>
	where
		T: serde::Serialize,
	{
		ser::Serialize::serialize(key, &mut **self)?;
		ser::Serialize::serialize(value, &mut **self)?;

		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}

impl<'a, W> ser::SerializeStructVariant for &'a mut Serializer<W>
where
	W: Write,
{
	type Ok = ();

	type Error = Error<'static>;

	fn serialize_field<T: ?Sized>(
		&mut self,
		key: &'static str,
		value: &T,
	) -> Result<(), Self::Error>
	where
		T: serde::Serialize,
	{
		ser::Serialize::serialize(key, &mut **self)?;
		ser::Serialize::serialize(value, &mut **self)?;

		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}
