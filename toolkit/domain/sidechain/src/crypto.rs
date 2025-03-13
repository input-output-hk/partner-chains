pub fn blake2b<const N: usize>(data: &[u8]) -> [u8; N] {
	blake2b_simd::Params::new()
		.hash_length(N)
		.hash(data)
		.as_bytes()
		.try_into()
		.unwrap_or_else(|_| panic!("hash output always has expected length of {N}"))
}
