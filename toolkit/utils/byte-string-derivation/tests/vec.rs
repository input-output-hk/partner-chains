extern crate alloc;
extern crate byte_string_derive;
use byte_string_derive::byte_string;

#[byte_string(debug, from_num, from_bytes, decode_hex)]
pub struct Test(pub Vec<u8>);

#[test]
fn debug() {
	let t = Test(vec![1, 2, 3, 4, 5, 6, 7, 8]);
	assert_eq!(format!("{:?}", t), "Test(0x102030405060708)")
}

#[test]
fn from_num() {
	let t = Test::from(0xA01020304);
	assert_eq!(t.0, vec![0, 0, 0, 10, 1, 2, 3, 4])
}

#[test]
fn from_bytes() {
	let slice: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7];
	let t = Test::from(slice);
	assert_eq!(t.0, vec![0, 1, 2, 3, 4, 5, 6, 7])
}

#[test]
fn decode_hex_success() {
	let t = Test::decode_hex("abcd").unwrap();
	assert_eq!(t.0, vec![0xab, 0xcd]);
}

#[test]
fn decode_hex_success_with_0x_prefix() {
	let t = Test::decode_hex("0xabcd").unwrap();
	assert_eq!(t.0, vec![0xab, 0xcd]);
}

#[test]
fn decode_hex_fail() {
	let t = Test::decode_hex("sfdds");

	assert!(t.is_err())
}
