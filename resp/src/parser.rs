use std::{borrow::Cow, str::from_utf8};

use nom::{
	branch::alt,
	bytes::streaming::take,
	character::streaming::{char, crlf, i64, not_line_ending},
	combinator::{map, map_res},
	error::ErrorKind,
	sequence::{delimited, terminated},
	IResult,
};

/// A parser error.
pub type Error<'a> = nom::Err<nom::error::Error<Cow<'a, [u8]>>>;
pub(crate) type RawError<'a> = nom::Err<nom::error::Error<&'a [u8]>>;

/// Parse a RESP string.
pub fn parse_str(data: &[u8]) -> IResult<&[u8], &str> {
	map_res(delimited(char('+'), not_line_ending, crlf), from_utf8)(data)
}

/// Parse a RESP error.
pub fn parse_err(data: &[u8]) -> IResult<&[u8], &str> {
	map_res(delimited(char('-'), not_line_ending, crlf), from_utf8)(data)
}

/// Parse a RESP integer.
pub fn parse_int(data: &[u8]) -> IResult<&[u8], i64> {
	delimited(char(':'), i64, crlf)(data)
}

/// Parse a RESP bulk string.
pub fn parse_bytes(data: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
	let (data, len) = delimited(char('$'), i64, crlf)(data)?;
	Ok(match len {
		-1 => (data, None),
		0.. => map(terminated(take(len as usize), crlf), Some)(data)?,
		_ => {
			return Err(nom::Err::Failure(nom::error::Error::new(
				data,
				ErrorKind::Digit,
			)))
		}
	})
}

/// Parse the length of a RESP array. Parsing the array elements is handled handled by the other
/// parsers.
pub fn parse_array(data: &[u8]) -> IResult<&[u8], i64> {
	delimited(char('*'), i64, crlf)(data)
}

/// Parse a RESP string, including bulk string if the bytes are valid UTF-8.
pub fn parse_str_loose(data: &[u8]) -> IResult<&[u8], &str> {
	alt((
		parse_str,
		map_res(map(parse_bytes, Option::unwrap_or_default), from_utf8),
	))(data)
}

/// Parse a RESP integer, including strings and bulk strings if they are valid integers.
pub fn parse_int_loose(data: &[u8]) -> IResult<&[u8], i64> {
	alt((parse_int, map_res(parse_str_loose, str::parse)))(data)
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_parse_str() {
		let resp = "+OK\r\n".as_bytes();
		let (rem, res) = parse_str(resp).expect("Parsed string");

		assert_eq!(0, rem.len());
		assert_eq!("OK", res);
	}

	#[test]
	fn test_parse_int() {
		let resp = ":10\r\n".as_bytes();
		let (rem, res) = parse_int(resp).expect("Parsed int");

		assert_eq!(0, rem.len());
		assert_eq!(10, res);
	}

	#[test]
	fn test_parse_bytes() {
		let resp = "$6\r\nfoobar\r\n".as_bytes();
		let (rem, res) = parse_bytes(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(Some("foobar".as_bytes()), res);
	}

	#[test]
	fn test_parse_empty_bytes() {
		let resp = "$0\r\n\r\n".as_bytes();
		let (rem, res) = parse_bytes(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(Some([].as_slice()), res);
	}

	#[test]
	fn test_parse_null_bytes() {
		let resp = "$-1\r\n".as_bytes();
		let (rem, res) = parse_bytes(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(None, res);
	}

	#[test]
	fn test_parse_array() {
		let resp = "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n".as_bytes();
		let (rem, res) = parse_array(resp).expect("Parsed bytes");

		assert_eq!(18, rem.len());
		assert_eq!(2, res);
	}

	#[test]
	fn test_parse_empty_array() {
		let resp = "*0\r\n".as_bytes();
		let (rem, res) = parse_array(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(0, res);
	}

	#[test]
	fn test_parse_null_array() {
		let resp = "*-1\r\n".as_bytes();
		let (rem, res) = parse_array(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(-1, res);
	}
}
