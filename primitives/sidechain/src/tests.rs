mod on_new_epoch {
	use frame_support::weights::Weight;
	use sidechain_domain::ScEpochNumber;

	use crate::OnNewEpoch;

	#[test]
	fn tuple_impl_works() {
		pub struct A {}
		pub struct B {}
		impl OnNewEpoch for A {
			fn on_new_epoch(
				_old_epoch: ScEpochNumber,
				_new_epoch: ScEpochNumber,
			) -> frame_support::weights::Weight {
				Weight::from_all(1000)
			}
		}
		impl OnNewEpoch for B {
			fn on_new_epoch(
				_old_epoch: ScEpochNumber,
				_new_epoch: ScEpochNumber,
			) -> frame_support::weights::Weight {
				Weight::from_all(2000)
			}
		}

		let a_b_weight = <(A, B) as OnNewEpoch>::on_new_epoch(ScEpochNumber(1), ScEpochNumber(2));
		assert_eq!(a_b_weight, Weight::from_all(3000))
	}
}
