//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2024-05-28 (Y/M/D)
//! HOSTNAME: `michal-pc`, CPU: `AMD Ryzen 9 5950X 16-Core Processor`
//!
//! DATABASE: `ParityDb`, RUNTIME: `Local Testnet`
//! BLOCK-NUM: `BlockId::Number(404)`
//! SKIP-WRITE: `false`, SKIP-READ: `false`, WARMUPS: `1`
//! STATE-VERSION: `V1`, STATE-CACHE-SIZE: ``
//! WEIGHT-PATH: `./runtime/src/weights/`
//! METRIC: `Average`, WEIGHT-MUL: `1.0`, WEIGHT-ADD: `0`

// Executed Command:
//   ./target/production/partner-chains-node
//   benchmark
//   storage
//   --base-path=/tmp/alice
//   --state-version=1
//   --warmups=1
//   --weight-path=./runtime/src/weights/

/// Storage DB weights for the `Local Testnet` runtime and `ParityDb`.
pub mod constants {
	use frame_support::weights::constants;
	use sp_core::parameter_types;
	use sp_weights::RuntimeDbWeight;

	parameter_types! {
		/// `ParityDB` can be enabled with a feature flag, but is still experimental. These weights
		/// are available for brave runtime engineers who may want to try this out as default.
		pub const ParityDbWeight: RuntimeDbWeight = RuntimeDbWeight {
			/// Time to read one storage item.
			/// Calculated by multiplying the *Average* of all values with `1.0` and adding `0`.
			///
			/// Stats nanoseconds:
			///   Min, Max: 1_150, 574_580
			///   Average:  3_790
			///   Median:   2_660
			///   Std-Dev:  26304.89
			///
			/// Percentiles nanoseconds:
			///   99th: 3_370
			///   95th: 3_070
			///   75th: 2_760
			read: 3_790 * constants::WEIGHT_REF_TIME_PER_NANOS,

			/// Time to write one storage item.
			/// Calculated by multiplying the *Average* of all values with `1.0` and adding `0`.
			///
			/// Stats nanoseconds:
			///   Min, Max: 4_950, 1_373_861
			///   Average:  22_524
			///   Median:   20_420
			///   Std-Dev:  62471.08
			///
			/// Percentiles nanoseconds:
			///   99th: 32_000
			///   95th: 26_900
			///   75th: 22_380
			write: 22_524 * constants::WEIGHT_REF_TIME_PER_NANOS,
		};
	}

	#[cfg(test)]
	mod test_db_weights {
		use super::constants::ParityDbWeight as W;
		use sp_weights::constants;

		/// Checks that all weights exist and have sane values.
		// NOTE: If this test fails but you are sure that the generated values are fine,
		// you can delete it.
		#[test]
		fn bound() {
			// At least 1 µs.
			assert!(
				W::get().reads(1).ref_time() >= constants::WEIGHT_REF_TIME_PER_MICROS,
				"Read weight should be at least 1 µs."
			);
			assert!(
				W::get().writes(1).ref_time() >= constants::WEIGHT_REF_TIME_PER_MICROS,
				"Write weight should be at least 1 µs."
			);
			// At most 1 ms.
			assert!(
				W::get().reads(1).ref_time() <= constants::WEIGHT_REF_TIME_PER_MILLIS,
				"Read weight should be at most 1 ms."
			);
			assert!(
				W::get().writes(1).ref_time() <= constants::WEIGHT_REF_TIME_PER_MILLIS,
				"Write weight should be at most 1 ms."
			);
		}
	}
}
