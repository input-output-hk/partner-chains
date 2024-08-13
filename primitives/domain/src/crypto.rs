use alloc::format;

pub fn blake2b<const N: usize>(data: &[u8]) -> [u8; N] {
	blake2b_simd::Params::new()
		.hash_length(N)
		.hash(data)
		.as_bytes()
		.try_into()
		.expect(&format!("hash output always has expected length of {N}"))
}
