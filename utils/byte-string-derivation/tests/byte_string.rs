extern crate alloc;
extern crate byte_string_derive;

use byte_string_derive::byte_string;
use sp_core::{bounded_vec::BoundedVec, ConstU32};

use serde_test::Token as SerdeToken;

#[derive(Debug, PartialEq)]
#[byte_string(hex_deserialize, hex_serialize)]
pub struct TestStruct(pub BoundedVec<u8, ConstU32<32>>);

#[test]
fn deserialize_works() {
	serde_test::assert_de_tokens(
		&TestStruct(BoundedVec::truncate_from(vec![1, 31, 112, 255])),
		&[SerdeToken::Str("0x011f70ff")],
	)
}

#[test]
fn serialize_works() {
	serde_test::assert_ser_tokens(
		&TestStruct(BoundedVec::truncate_from(vec![1, 31, 112, 255])),
		&[SerdeToken::Str("0x011f70ff")],
	)
}
