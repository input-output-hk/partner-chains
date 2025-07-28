//! Time source abstraction for obtaining the current time in milliseconds.
//!
//! This module defines a `TimeSource` trait and two implementations:
//! - [`SystemTimeSource`]: system-clock based implementation
//! - [`MockedTimeSource`]: mock implementation for testing purposes
//!
//! # Example
//!
//! ```
//! use time_source::*;
//! let system_time = SystemTimeSource;
//! let now = system_time.get_current_time_millis();
//! println!("Current time: {now} ms since epoch");
//! ```

use std::time::SystemTime;

/// A trait representing a source of time that can provide the current time in milliseconds.
pub trait TimeSource {
	/// Returns the current time in milliseconds since Unix epoch.
	fn get_current_time_millis(&self) -> u64;
}

/// A system clock based time source
pub struct SystemTimeSource;

impl TimeSource for SystemTimeSource {
	fn get_current_time_millis(&self) -> u64 {
		u64::try_from(
			SystemTime::now()
				.duration_since(SystemTime::UNIX_EPOCH)
				.expect("Current time is always after unix epoch; qed")
				.as_millis(),
		)
		.expect("Current time in millis should fit in 64 bits")
	}
}

#[cfg(any(feature = "mock", doc))]
/// A mock implementation of `TimeSource` for testing purposes.
pub struct MockedTimeSource {
	/// The mocked current time in milliseconds since Unix epoch
	///
	/// This value will be returned on all `get_current_time_millis` calls
	pub current_time_millis: u64,
}
#[cfg(feature = "mock")]
impl TimeSource for MockedTimeSource {
	fn get_current_time_millis(&self) -> u64 {
		self.current_time_millis
	}
}
