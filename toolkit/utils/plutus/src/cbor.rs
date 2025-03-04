use num_bigint::{BigInt, Sign};

extern crate alloc;

use crate::{Datum, Datum::*, MapDatumEntry};

use minicbor::{
	data::Tag,
	encode,
	encode::{Encoder, Write},
};

impl<C> encode::Encode<C> for Datum {
	fn encode<W: Write>(
		&self,
		e: &mut Encoder<W>,
		ctx: &mut C,
	) -> Result<(), encode::Error<W::Error>> {
		encode_datum(self, e, ctx)
	}
}

fn encode_datum<C, W: Write>(
	datum: &Datum,
	e: &mut Encoder<W>,
	ctx: &mut C,
) -> Result<(), encode::Error<W::Error>> {
	match datum {
		IntegerDatum(bi) => encode_integer(bi, e, ctx),
		ByteStringDatum(bytes) => encode_bytestring(bytes, e, ctx),
		ConstructorDatum { constructor, fields } => {
			encode_constructor(*constructor, fields, e, ctx)
		},
		ListDatum(items) => encode_linear_seq(items, e, ctx),
		MapDatum(entries) => encode_entries(entries, e, ctx),
	}
}

fn encode_integer<W: Write, C>(
	i: &BigInt,
	e: &mut Encoder<W>,
	ctx: &mut C,
) -> Result<(), encode::Error<W::Error>> {
	match i64::try_from(i) {
		Ok(num) => {
			e.i64(num)?;
			Ok(())
		},
		_ => {
			let (sign, bytes) = i.to_bytes_be();
			match sign {
				Sign::Plus => e.tag(Tag::PosBignum)?,
				Sign::Minus => e.tag(Tag::NegBignum)?,
				Sign::NoSign => unreachable!("NoSign is not possible here, since 0 is a valid i64"),
			};
			encode_bytestring(&bytes, e, ctx)
		},
	}
}

const BYTES_CHUNK_SIZE_LIMIT: usize = 64;

fn encode_bytestring<W: Write, C>(
	bytes: &[u8],
	e: &mut Encoder<W>,
	_ctx: &mut C,
) -> Result<(), encode::Error<W::Error>> {
	if bytes.len() <= BYTES_CHUNK_SIZE_LIMIT {
		e.bytes(bytes)?;
		Ok(())
	} else {
		e.begin_bytes()?;
		for chunk in bytes.chunks(BYTES_CHUNK_SIZE_LIMIT) {
			e.bytes(chunk)?;
		}
		e.end()?;
		Ok(())
	}
}

/// The only specification is the plutus-core source code
/// Please refer to: https://github.com/input-output-hk/plutus/blob/ab4d2cc43b642d8ddc43180d1ca1c40937b1e629/plutus-core/plutus-core/src/PlutusCore/Data.hs#L71
fn encode_constructor<W: Write, C>(
	constructor: u64,
	fields: &[Datum],
	e: &mut Encoder<W>,
	ctx: &mut C,
) -> Result<(), encode::Error<W::Error>> {
	if constructor < 7 {
		e.tag(constructor_small_tag(constructor))?;
		encode_linear_seq(fields, e, ctx)
	} else if (7..128).contains(&constructor) {
		e.tag(constructor_big_tag(constructor))?;
		encode_linear_seq(fields, e, ctx)
	} else {
		e.tag(CONSTRUCTOR_OVER_LIMIT_TAG)?;
		encode_constructor_over_limit_fields(constructor, fields, e, ctx)
	}
}

fn encode_linear_seq<W: Write, C>(
	ds: &[Datum],
	e: &mut Encoder<W>,
	ctx: &mut C,
) -> Result<(), encode::Error<W::Error>> {
	//Do it like Plutus
	// "spec": https://github.com/input-output-hk/plutus/blob/ab4d2cc43b642d8ddc43180d1ca1c40937b1e629/plutus-core/plutus-core/src/PlutusCore/Data.hs#L71
	if ds.is_empty() {
		e.array(0)?;
	} else {
		e.begin_array()?;
		for d in ds {
			encode_datum(d, e, ctx)?;
		}
		e.end()?;
	}
	Ok(())
}

fn encode_constructor_over_limit_fields<W: Write, C>(
	constructor: u64,
	ds: &[Datum],
	e: &mut Encoder<W>,
	ctx: &mut C,
) -> Result<(), encode::Error<W::Error>> {
	//This is two elements array where the first item is constructor value and the second is encoded sequence of datums
	e.array(2)?;
	e.u64(constructor)?;
	encode_linear_seq(ds, e, ctx)
}

fn encode_entries<W: Write, C>(
	entries: &[MapDatumEntry],
	e: &mut Encoder<W>,
	ctx: &mut C,
) -> Result<(), encode::Error<W::Error>> {
	//Do it like Plutus
	let len = entries.len();
	e.map(len as u64)?;
	for entry in entries {
		encode_datum(&entry.key, e, ctx)?;
		encode_datum(&entry.value, e, ctx)?;
	}
	Ok(())
}

fn constructor_small_tag(value: u64) -> Tag {
	Tag::Unassigned(value + 121)
}

fn constructor_big_tag(value: u64) -> Tag {
	Tag::Unassigned(1280 + value - 7)
}

const CONSTRUCTOR_OVER_LIMIT_TAG: Tag = Tag::Unassigned(102);

#[cfg(test)]
mod tests {
	use crate::{Datum, Datum::*, MapDatumEntry};
	pub use alloc::{vec, vec::Vec};
	use num_bigint::BigInt;
	use num_traits::Num;

	#[test]
	fn integer_datums_to_cbor() {
		let small_int_narrow = IntegerDatum(BigInt::from(7u8));
		let small_int_wide = IntegerDatum(BigInt::from(7i64));
		let small_int_negative = IntegerDatum(BigInt::from(-9i64));
		let big_int_positive =
			IntegerDatum(BigInt::from_str_radix("123456789123456789", 10).unwrap());
		let big_int_negative =
			IntegerDatum(BigInt::from_str_radix("-123456789123456789", 10).unwrap());

		test(small_int_narrow, "07");
		test(small_int_wide, "07");
		test(small_int_negative, "28");
		test(big_int_positive, "1b01b69b4bacd05f15");
		test(big_int_negative, "3b01b69b4bacd05f14");
	}

	#[test]
	fn bytestring_datums_to_cbor() {
		let empty = ByteStringDatum(Vec::new());
		let shorter_than_64 = ByteStringDatum(Vec::from([65u8; 10]));
		let just_64 = ByteStringDatum(Vec::from([66u8; 64]));
		let longer_than_64 = ByteStringDatum(Vec::from([67u8; 65]));

		test(empty, "40");
		test(shorter_than_64, "4a41414141414141414141");
		test(
			just_64,
			"584042424242424242424242424242424242424242424242424242424242424242424242424242424242424242424242424242424242424242424242424242424242",
		);
		test(
			longer_than_64,
			"5f5840434343434343434343434343434343434343434343434343434343434343434343434343434343434343434343434343434343434343434343434343434343434143ff",
		);
	}

	#[test]
	fn constructor_datums_to_cbor() {
		fn make_fields(num: u64) -> Vec<Datum> {
			let mut v = Vec::new();
			for i in 1..(num + 1) {
				v.push(IntegerDatum(BigInt::from(i)))
			}
			v
		}

		let small_empty = ConstructorDatum { constructor: 2, fields: Vec::new() };
		let small_non_empty = ConstructorDatum { constructor: 2, fields: make_fields(3) };
		let big_empty = ConstructorDatum { constructor: 34, fields: Vec::new() };
		let big_non_empty = ConstructorDatum { constructor: 34, fields: make_fields(2) };
		let over_limit_empty = ConstructorDatum { constructor: 130, fields: Vec::new() };
		let over_limit_non_empty = ConstructorDatum { constructor: 130, fields: make_fields(4) };

		test(small_empty, "d87b80");
		test(small_non_empty, "d87b9f010203ff");
		test(big_empty, "d9051b80");
		test(big_non_empty, "d9051b9f0102ff");
		test(over_limit_empty, "d86682188280");
		test(over_limit_non_empty, "d8668218829f01020304ff");
	}

	#[test]
	fn list_datums_to_cbor() {
		test(ListDatum(vec![]), "80");
		test(ListDatum(vec![ByteStringDatum(hex::decode("deadbeef").unwrap())]), "9f44deadbeefff");
		test(
			ListDatum(vec![
				ByteStringDatum(hex::decode("deadbeef").unwrap()),
				Datum::integer(13),
				ListDatum(vec![
					Datum::integer(42),
					ByteStringDatum(hex::decode("0102030405").unwrap()),
				]),
			]),
			"9f44deadbeef0d9f182a450102030405ffff",
		);
	}

	#[test]
	fn map_datums_to_cbor() {
		test(MapDatum(vec![]), "a0");
		test(
			MapDatum(vec![
				MapDatumEntry { key: Datum::integer(99), value: Datum::integer(101) },
				MapDatumEntry {
					key: ByteStringDatum(hex::decode("deadbeef").unwrap()),
					value: Datum::integer(3),
				},
			]),
			"a21863186544deadbeef03",
		);
		test(
			MapDatum(
				(1..14)
					.map(|i| MapDatumEntry { key: Datum::integer(i), value: Datum::integer(1) })
					.collect(),
			),
			"ad0101020103010401050106010701080109010a010b010c010d01",
		);
	}

	fn test(d: Datum, expected_hex: &str) {
		assert_eq!(hex::encode(minicbor::to_vec(d).unwrap()), expected_hex);
	}
}
