//! Coefficient newtypes for the redundant polynomial representation.
//!
//! - [`WordCoeff`] holds a 16-bit non-redundant value.
//! - [`RedundantBit`] holds a single redundant bit.
//! - [`FullWordCoeff`] holds a full 17-bit coefficient.

use crate::error::Error;
use crate::params::{FULL_WORD_BITS_MASK, WORD_BITS, WORD_BITS_MASK};

/// A 16-bit non-redundant coefficient.  Value is guaranteed `< 2^16`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[must_use]
pub struct WordCoeff(u32);

impl WordCoeff {
    /// Construct from a `u32`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CoefficientOutOfRange`] if the value is `>= 2^16`.
    pub fn new(value: u32) -> Result<Self, Error> {
        if value <= WORD_BITS_MASK {
            Ok(Self(value))
        } else {
            Err(Error::CoefficientOutOfRange {
                value: u64::from(value),
                max: u64::from(WORD_BITS_MASK) + 1,
            })
        }
    }

    /// The zero coefficient.
    pub const fn zero() -> Self {
        Self(0)
    }

    /// The underlying value.
    #[must_use]
    pub fn value(self) -> u32 {
        self.0
    }
}

/// A single redundant bit.  Value is guaranteed `0` or `1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[must_use]
pub struct RedundantBit(u32);

impl RedundantBit {
    /// Construct from a `u32`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CoefficientOutOfRange`] if the value is `> 1`.
    pub fn new(value: u32) -> Result<Self, Error> {
        match value {
            0 | 1 => Ok(Self(value)),
            v => Err(Error::CoefficientOutOfRange {
                value: u64::from(v),
                max: 2,
            }),
        }
    }

    /// The zero bit.
    pub const fn zero() -> Self {
        Self(0)
    }

    /// The underlying value.
    #[must_use]
    pub fn value(self) -> u32 {
        self.0
    }
}

/// A 17-bit full coefficient.  Value is guaranteed `< 2^17`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[must_use]
pub struct FullWordCoeff(u32);

impl FullWordCoeff {
    /// Construct from a `u32`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CoefficientOutOfRange`] if the value is `>= 2^17`.
    pub fn new(value: u32) -> Result<Self, Error> {
        if value <= FULL_WORD_BITS_MASK {
            Ok(Self(value))
        } else {
            Err(Error::CoefficientOutOfRange {
                value: u64::from(value),
                max: u64::from(FULL_WORD_BITS_MASK) + 1,
            })
        }
    }

    /// The zero coefficient.
    pub const fn zero() -> Self {
        Self(0)
    }

    /// The underlying value.
    #[must_use]
    pub fn value(self) -> u32 {
        self.0
    }

    /// Combine a non-redundant word with a redundant bit.
    pub fn combine(nr: WordCoeff, r: RedundantBit) -> Self {
        Self(nr.0 | (r.0 << WORD_BITS))
    }

    /// Split into non-redundant word and redundant bit.
    pub fn split(self) -> (WordCoeff, RedundantBit) {
        (
            WordCoeff(self.0 & WORD_BITS_MASK),
            RedundantBit((self.0 >> WORD_BITS) & 1),
        )
    }
}

impl From<WordCoeff> for FullWordCoeff {
    fn from(w: WordCoeff) -> Self {
        Self(w.0)
    }
}
