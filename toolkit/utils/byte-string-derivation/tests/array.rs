extern crate alloc;
extern crate byte_string_derive;

use byte_string_derive::byte_string;

#[byte_string(debug, from_num, from_bytes, decode_hex)]
pub struct Test(pub [u8; 8]);

#[test]
fn debug() {
	let t = Test([1, 2, 3, 4, 5, 6, 7, 8]);
	assert_eq!(format!("{:?}", t), "Test(0x0102030405060708)")
}

#[test]
fn from_num() {
	let t = Test::from(0xA01020304);
	assert_eq!(t.0, [0, 0, 0, 10, 1, 2, 3, 4])
}

#[test]
fn from_bytes_success() {
	let slice: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7];
	let t = Test::try_from(slice).unwrap();
	assert_eq!(t.0, [0, 1, 2, 3, 4, 5, 6, 7])
}

#[test]
fn from_bytes_failure() {
	let slice: &[u8] = &[0, 1, 2, 3];
	let t = Test::try_from(slice);
	assert!(t.is_err())
}

#[test]
fn decode_hex_success() {
	let t = Test::decode_hex("010203040a0b0c0d").unwrap();
	assert_eq!(t.0, [0x1, 0x2, 0x3, 0x4, 0xa, 0xb, 0xc, 0xd]);
}

#[test]
fn decode_hex_success_0x_prefix() {
	let t = Test::decode_hex("0x010203040a0b0c0d").unwrap();
	assert_eq!(t.0, [0x1, 0x2, 0x3, 0x4, 0xa, 0xb, 0xc, 0xd]);
}

#[test]
fn decode_hex_fail() {
	let t = Test::decode_hex("sfdds");
	assert!(t.is_err());

	let t = Test::decode_hex("abcd");
	assert!(t.is_err());
}
