//! Fixed-width unsigned integer arithmetic.
//!
//! Provides [`U256`] and [`U512`] for big-integer arithmetic needed by
//! MSU table generation and the golden model.  All operations are
//! immutable and use iterator combinators (no loops, no mutable state).
//! Multiplication of two [`U256`] values produces a [`U512`].

pub mod u256;
pub mod u512;

pub use u256::U256;
pub use u512::U512;

/// Extract the low 64 bits of a `u128`.
///
/// Safe truncation: the mask guarantees the result fits in `u64`.
#[must_use]
pub(crate) fn u128_low(x: u128) -> u64 {
    u64::try_from(x & u128::from(u64::MAX)).unwrap_or(0)
}

/// Extract the high 64 bits of a `u128`.
///
/// Safe truncation: right-shift by 64 guarantees the result fits in `u64`.
#[must_use]
pub(crate) fn u128_high(x: u128) -> u64 {
    u64::try_from(x >> 64).unwrap_or(0)
}

/// Parse a single hex digit character.
pub(crate) fn hex_digit(c: u8) -> Result<u8, crate::error::Error> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(crate::error::Error::HexParse("invalid hex digit")),
    }
}
