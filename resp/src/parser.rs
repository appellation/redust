use nom::{
	branch::alt,
	bytes::streaming::take,
	character::{
		complete::not_line_ending,
		streaming::{char, crlf, i64},
	},
	combinator::{map, map_res},
	multi::many_m_n,
	sequence::{preceded, terminated},
	IResult,
};

use crate::Data;

pub type Error<'a> = nom::Err<nom::error::Error<&'a [u8]>>;

pub(crate) fn parse_str(data: &[u8]) -> IResult<&[u8], &str> {
	map_res(terminated(not_line_ending, crlf), std::str::from_utf8)(data)
}

pub(crate) fn parse_int(data: &[u8]) -> IResult<&[u8], i64> {
	terminated(i64, crlf)(data)
}

pub(crate) fn parse_bytes(data: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
	let (data, len) = parse_int(data)?;
	Ok(match len {
		-1 => (data, None),
		0.. => map(terminated(take(len as usize), crlf), Some)(data)?,
		_ => panic!("Unexpected data length from Redis: {}", len),
	})
}

pub(crate) fn parse_array(data: &[u8]) -> IResult<&[u8], Option<Vec<Data>>> {
	let (data, len) = parse_int(data)?;
	Ok(match len {
		-1 => (data, None),
		0.. => map(many_m_n(len as usize, len as usize, parse), Some)(data)?,
		_ => panic!("Unexpected data length from Redis: {}", len),
	})
}

pub(crate) fn parse_data_simple_string(data: &[u8]) -> IResult<&[u8], Data> {
	map(parse_str, Data::SimpleString)(data)
}

pub(crate) fn parse_data_error(data: &[u8]) -> IResult<&[u8], Data> {
	map(parse_str, Data::Error)(data)
}

pub(crate) fn parse_data_integer(data: &[u8]) -> IResult<&[u8], Data> {
	map(parse_int, Data::Integer)(data)
}

pub(crate) fn parse_data_bulk_string(data: &[u8]) -> IResult<&[u8], Data> {
	map(parse_bytes, Data::BulkString)(data)
}

pub(crate) fn parse_data_array(data: &[u8]) -> IResult<&[u8], Data> {
	map(parse_array, Data::Array)(data)
}

pub fn parse(data: &[u8]) -> IResult<&[u8], Data> {
	alt((
		preceded(char('+'), parse_data_simple_string),
		preceded(char('-'), parse_data_error),
		preceded(char(':'), parse_data_integer),
		preceded(char('$'), parse_data_bulk_string),
		preceded(char('*'), parse_data_array),
	))(data)
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_parse_str() {
		let resp = "OK\r\n".as_bytes();
		let (rem, res) = parse_str(resp).expect("Parsed string");

		assert_eq!(0, rem.len());
		assert_eq!("OK", res);
	}

	#[test]
	fn test_parse_int() {
		let resp = "10\r\n".as_bytes();
		let (rem, res) = parse_int(resp).expect("Parsed int");

		assert_eq!(0, rem.len());
		assert_eq!(10, res);
	}

	#[test]
	fn test_parse_bytes() {
		let resp = "6\r\nfoobar\r\n".as_bytes();
		let (rem, res) = parse_bytes(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(Some("foobar".as_bytes()), res);
	}

	#[test]
	fn test_parse_empty_bytes() {
		let resp = "0\r\n\r\n".as_bytes();
		let (rem, res) = parse_bytes(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(Some([].as_slice()), res);
	}

	#[test]
	fn test_parse_null_bytes() {
		let resp = "-1\r\n".as_bytes();
		let (rem, res) = parse_bytes(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(None, res);
	}

	#[test]
	fn test_parse_array() {
		let resp = "2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n".as_bytes();
		let (rem, res) = parse_array(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(
			Some(vec![
				Data::BulkString(Some("foo".as_bytes())),
				Data::BulkString(Some("bar".as_bytes()))
			]),
			res
		);
	}

	#[test]
	fn test_parse_empty_array() {
		let resp = "0\r\n".as_bytes();
		let (rem, res) = parse_array(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(Some(vec![]), res);
	}

	#[test]
	fn test_parse_null_array() {
		let resp = "-1\r\n".as_bytes();
		let (rem, res) = parse_array(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(None, res);
	}

	#[test]
	fn test_parse_resp_string() {
		let resp = "+OK\r\n".as_bytes();
		let (rem, res) = parse(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(Data::SimpleString("OK"), res);
	}

	#[test]
	fn test_parse_resp_error() {
		let resp = "-ERR unknown command 'foobar'\r\n".as_bytes();
		let (rem, res) = parse(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(Data::Error("ERR unknown command 'foobar'"), res);
	}

	#[test]
	fn test_parse_resp_int() {
		let resp = ":1000\r\n".as_bytes();
		let (rem, res) = parse(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(Data::Integer(1000), res);
	}

	#[test]
	fn test_parse_resp_bulk_string() {
		let resp = "$6\r\nfoobar\r\n".as_bytes();
		let (rem, res) = parse(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(Data::BulkString(Some("foobar".as_bytes())), res);
	}

	#[test]
	fn test_parse_resp_array() {
		let resp = "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n".as_bytes();
		let (rem, res) = parse(resp).expect("Parsed bytes");

		assert_eq!(0, rem.len());
		assert_eq!(
			Data::Array(Some(vec![
				Data::BulkString(Some("foo".as_bytes())),
				Data::BulkString(Some("bar".as_bytes()))
			])),
			res
		);
	}
}
