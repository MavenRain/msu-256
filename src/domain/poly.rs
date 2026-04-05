//! Redundant polynomial: the MSU's core signal representation.

use crate::bigint::U256;
use crate::domain::coeff::{FullWordCoeff, RedundantBit, WordCoeff};
use crate::error::Error;
use crate::params::{NUM_ELEMENTS, WORD_BITS};

/// A value stored as a redundant polynomial: [`NUM_ELEMENTS`] coefficients,
/// each with a non-redundant word and a redundant bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub struct RedundantPoly {
    nr: [WordCoeff; NUM_ELEMENTS],
    r: [RedundantBit; NUM_ELEMENTS],
}

impl RedundantPoly {
    /// Construct from arrays of coefficients.
    pub fn new(nr: [WordCoeff; NUM_ELEMENTS], r: [RedundantBit; NUM_ELEMENTS]) -> Self {
        Self { nr, r }
    }

    /// The zero polynomial.
    pub fn zero() -> Self {
        Self {
            nr: [WordCoeff::zero(); NUM_ELEMENTS],
            r: [RedundantBit::zero(); NUM_ELEMENTS],
        }
    }

    /// Non-redundant coefficient at an index.
    ///
    /// # Errors
    ///
    /// Returns [`Error::IndexOutOfBounds`] if `index >= NUM_ELEMENTS`.
    pub fn nr(&self, index: usize) -> Result<WordCoeff, Error> {
        self.nr.get(index).copied().ok_or(Error::IndexOutOfBounds {
            index,
            length: NUM_ELEMENTS,
        })
    }

    /// Redundant bit at an index.
    ///
    /// # Errors
    ///
    /// Returns [`Error::IndexOutOfBounds`] if `index >= NUM_ELEMENTS`.
    pub fn r(&self, index: usize) -> Result<RedundantBit, Error> {
        self.r.get(index).copied().ok_or(Error::IndexOutOfBounds {
            index,
            length: NUM_ELEMENTS,
        })
    }

    /// Full coefficient at an index: `nr[i] + r[i] << WORD_BITS`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::IndexOutOfBounds`] if `index >= NUM_ELEMENTS`.
    pub fn full_coeff(&self, index: usize) -> Result<FullWordCoeff, Error> {
        Ok(FullWordCoeff::combine(self.nr(index)?, self.r(index)?))
    }

    /// The non-redundant coefficients as an array.
    pub fn nr_array(&self) -> [WordCoeff; NUM_ELEMENTS] {
        self.nr
    }

    /// The redundant bits as an array.
    pub fn r_array(&self) -> [RedundantBit; NUM_ELEMENTS] {
        self.r
    }

    /// Evaluate the polynomial to a [`U256`] value.
    ///
    /// Each coefficient `i` contributes `(nr[i] + r[i] << 16) << (i * 16)`.
    /// The result may overflow 256 bits for non-canonical inputs; those
    /// bits are discarded.
    pub fn to_u256(&self) -> U256 {
        (0..NUM_ELEMENTS).fold(U256::zero(), |acc, i| {
            let nr_val = self.nr[i].value();
            let r_val = self.r[i].value();
            let shift = i * WORD_BITS;
            let nr_shifted = U256::from_u64(u64::from(nr_val)) << shift;
            let r_shifted = U256::from_u64(u64::from(r_val)) << (shift + WORD_BITS);
            acc + nr_shifted + r_shifted
        })
    }

    /// Construct from a [`U256`] in canonical form (all redundant bits zero).
    ///
    /// # Errors
    ///
    /// Returns [`Error::CoefficientOutOfRange`] if coefficient extraction
    /// exceeds the allowed range (should not happen for a valid `U256`).
    pub fn from_u256(v: &U256) -> Result<Self, Error> {
        let mask = u64::from(crate::params::WORD_BITS_MASK);
        let nr = (0..NUM_ELEMENTS)
            .map(|i| {
                let shift = i * WORD_BITS;
                let shifted = *v >> shift;
                let low = shifted.limbs()[0] & mask;
                WordCoeff::new(u32::try_from(low).unwrap_or(0))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let nr_array: [WordCoeff; NUM_ELEMENTS] = nr
            .try_into()
            .map_err(|_| Error::CoefficientOutOfRange { value: 0, max: 0 })?;
        Ok(Self {
            nr: nr_array,
            r: [RedundantBit::zero(); NUM_ELEMENTS],
        })
    }
}

impl Default for RedundantPoly {
    fn default() -> Self {
        Self::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_round_trips() -> Result<(), Error> {
        let z = RedundantPoly::zero();
        assert_eq!(z.to_u256(), U256::zero());
        let back = RedundantPoly::from_u256(&U256::zero())?;
        assert_eq!(back, z);
        Ok(())
    }

    #[test]
    fn round_trip_small_value() -> Result<(), Error> {
        let v = U256::from_u64(0x1234_5678);
        let poly = RedundantPoly::from_u256(&v)?;
        assert_eq!(poly.to_u256(), v);
        Ok(())
    }

    #[test]
    fn round_trip_large_value() -> Result<(), Error> {
        let v = U256::from_le_limbs([
            0xDEAD_BEEF_FEED_FACE,
            0x0123_4567_89AB_CDEF,
            0xCAFE_BABE_DEAD_C0DE,
            0x0011_2233_4455_6677,
        ]);
        let poly = RedundantPoly::from_u256(&v)?;
        assert_eq!(poly.to_u256(), v);
        Ok(())
    }
}
