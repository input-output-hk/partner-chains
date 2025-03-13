use std::cmp::Ordering;
use std::ops::{Add, Div, Sub};

pub fn binary_search_by<T, F, E>(range: std::ops::Range<T>, mut f: F) -> Option<T>
where
	F: FnMut(&T) -> Result<Ordering, E>,
	T: Add<Output = T> + Div<Output = T> + Sub<Output = T> + PartialOrd,
	T: From<u8>,
	T: Copy,
{
	let (mut left, mut right) = (range.start, range.end);

	while left <= right {
		let middle = (left + right) / 2.into();

		match f(&middle).ok()? {
			Ordering::Equal => return Some(middle),
			Ordering::Less => left = middle + 1.into(),
			Ordering::Greater => right = middle - 1.into(),
		}
	}

	None
}
