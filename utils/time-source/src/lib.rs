use std::time::SystemTime;

pub trait TimeSource {
	fn get_current_time_millis(&self) -> u64;
}

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

#[cfg(feature = "mock")]
pub struct MockedTimeSource {
	pub current_time_millis: u64,
}
#[cfg(feature = "mock")]
impl TimeSource for MockedTimeSource {
	fn get_current_time_millis(&self) -> u64 {
		self.current_time_millis
	}
}
