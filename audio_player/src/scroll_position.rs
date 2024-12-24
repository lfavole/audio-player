//! Implementation of the [`Scrollable`] trait.
use std::ops::{Add, Sub};

/// Allows a numeric type to add or subtract 1 while staying within some bounds.
///
/// This is useful for scrolling, hence the name.
pub trait Scrollable
where
    Self: Sized + Add<Output = Self> + Copy + Default + Eq + Sub<Output = Self>,
{
    /// The value of 1 in the given type.
    const ONE: Self;

    /// Returns the index of the previous element according to the total number of elements.
    fn previous(&self, length: Self) -> Self {
        if *self == Self::default() {
            length - Self::ONE
        } else {
            *self - Self::ONE
        }
    }

    /// Returns the index of the next element according to the total number of elements.
    fn next(&self, length: Self) -> Self {
        if *self == length - Self::ONE {
            Self::default()
        } else {
            *self + Self::ONE
        }
    }
}

macro_rules! scrollable_impl {
    ($($t:ty)+) => ($(
        impl Scrollable for $t {
            const ONE: Self = 1;
        }
    )+)
}

scrollable_impl! { usize u8 u16 u32 u64 u128 isize i8 i16 i32 i64 i128 }
