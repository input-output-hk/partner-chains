//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2024-05-30 (Y/M/D)
//! HOSTNAME: `0138a9af6c6b`, CPU: `AMD EPYC 7763 64-Core Processor`
//!
//! DATABASE: `RocksDb`, RUNTIME: `Local Testnet`
//! BLOCK-NUM: `BlockId::Number(0)`
//! SKIP-WRITE: `false`, SKIP-READ: `false`, WARMUPS: `1`
//! STATE-VERSION: `V1`, STATE-CACHE-SIZE: ``
//! WEIGHT-PATH: `./runtime/src/weights/`
//! METRIC: `Average`, WEIGHT-MUL: `1.0`, WEIGHT-ADD: `0`

// Executed Command:
//   ./target/production/partner-chains-node
//   benchmark
//   storage
//   --state-version=1
//   --warmups=1
//   --weight-path=./runtime/src/weights/

/// Storage DB weights for the `Local Testnet` runtime and `RocksDb`.
pub mod constants {
	use frame_support::weights::constants;
	use sp_core::parameter_types;
	use sp_weights::RuntimeDbWeight;

	parameter_types! {
		/// By default, Substrate uses `RocksDB`, so this will be the weight used throughout
		/// the runtime.
		pub const RocksDbWeight: RuntimeDbWeight = RuntimeDbWeight {
			/// Time to read one storage item.
			/// Calculated by multiplying the *Average* of all values with `1.0` and adding `0`.
			///
			/// Stats nanoseconds:
			///   Min, Max: 2_054, 877_284
			///   Average:  20_301
			///   Median:   3_927
			///   Std-Dev:  118845.44
			///
			/// Percentiles nanoseconds:
			///   99th: 877_284
			///   95th: 5_179
			///   75th: 4_699
			read: 20_301 * constants::WEIGHT_REF_TIME_PER_NANOS,

			/// Time to write one storage item.
			/// Calculated by multiplying the *Average* of all values with `1.0` and adding `0`.
			///
			/// Stats nanoseconds:
			///   Min, Max: 12_313, 20_813_489
			///   Average:  416_545
			///   Median:   23_234
			///   Std-Dev:  2828556.47
			///
			/// Percentiles nanoseconds:
			///   99th: 20_813_489
			///   95th: 38_583
			///   75th: 28_745
			write: 416_545 * constants::WEIGHT_REF_TIME_PER_NANOS,
		};
	}

	#[cfg(test)]
	mod test_db_weights {
		use super::constants::RocksDbWeight as W;
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
