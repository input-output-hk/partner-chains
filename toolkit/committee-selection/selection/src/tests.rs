use quickcheck::Arbitrary;

macro_rules! assert_subset {
	($type: ident, $subset:expr, $superset:expr) => {
		let subset = std::collections::HashSet::<$type>::from_iter($subset.iter().cloned());
		let superset = std::collections::HashSet::<$type>::from_iter($superset.iter().cloned());
		assert!(subset.is_subset(&superset), "{:?} was not a subset of {:?}?", subset, superset);
	};
}
pub(crate) use assert_subset;

#[derive(Clone, Debug)]
pub(crate) struct TestNonce(pub [u8; 32]);

impl Arbitrary for TestNonce {
	fn arbitrary(g: &mut quickcheck::Gen) -> Self {
		let mut array = [1; 32];
		for elem in &mut array {
			*elem = <u8 as Arbitrary>::arbitrary(g);
		}

		TestNonce(array)
	}
}
