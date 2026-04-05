//! 512-bit unsigned integer.
//!
//! Backed by eight little-endian `u64` limbs.  Used primarily as the
//! intermediate type for [`super::U256`] multiplication and for
//! computing `self mod U256::modulus`.

use core::cmp::Ordering;
use core::ops::{Add, BitAnd, BitOr, BitXor, Shl, Shr, Sub};

use super::U256;
use super::{u128_high, u128_low};
use crate::error::Error;

/// Number of 64-bit limbs in a U512.
const LIMBS: usize = 8;

/// A 512-bit unsigned integer.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[must_use]
pub struct U512 {
    limbs: [u64; LIMBS],
}

impl U512 {
    /// The zero value.
    pub const fn zero() -> Self {
        Self { limbs: [0; LIMBS] }
    }

    /// The value one.
    pub const fn one() -> Self {
        Self {
            limbs: [1, 0, 0, 0, 0, 0, 0, 0],
        }
    }

    /// Construct from eight little-endian `u64` limbs.
    pub const fn from_le_limbs(limbs: [u64; LIMBS]) -> Self {
        Self { limbs }
    }

    /// Zero-extend a `U256` to 512 bits.
    pub fn from_u256(v: U256) -> Self {
        let src = v.limbs();
        Self {
            limbs: [src[0], src[1], src[2], src[3], 0, 0, 0, 0],
        }
    }

    /// Access a limb by index.
    ///
    /// # Errors
    ///
    /// Returns [`Error::IndexOutOfBounds`] if `index >= 8`.
    pub fn limb(&self, index: usize) -> Result<u64, Error> {
        self.limbs.get(index).copied().ok_or(Error::IndexOutOfBounds {
            index,
            length: LIMBS,
        })
    }

    /// Truncate to the low 256 bits.
    pub fn low_u256(&self) -> U256 {
        U256::from_le_limbs([self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3]])
    }

    /// Test whether a specific bit is set.
    #[must_use]
    pub fn bit(&self, position: usize) -> bool {
        match position {
            p if p >= 512 => false,
            p => {
                let limb_index = p / 64;
                let bit_index = p % 64;
                (self.limbs[limb_index] >> bit_index) & 1 == 1
            }
        }
    }

    /// The number of bits required to represent this value.
    #[must_use]
    pub fn bit_length(&self) -> usize {
        (0..LIMBS)
            .rev()
            .find_map(|i| match self.limbs[i] {
                0 => None,
                v => {
                    let bits_in_limb = 64 - usize::try_from(v.leading_zeros()).unwrap_or(0);
                    Some(i * 64 + bits_in_limb)
                }
            })
            .unwrap_or(0)
    }

    /// Whether this value is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.limbs.iter().all(|&l| l == 0)
    }

    /// Add with carry.
    pub fn overflowing_add(self, other: Self) -> (Self, bool) {
        (0..LIMBS)
            .fold(([0_u64; LIMBS], 0_u128), |(arr, carry), i| {
                let sum = u128::from(self.limbs[i]) + u128::from(other.limbs[i]) + carry;
                (set_limb(arr, i, u128_low(sum)), u128::from(u128_high(sum)))
            })
            .pipe(|(limbs, carry_out)| (Self { limbs }, carry_out != 0))
    }

    /// Subtract with borrow.
    pub fn overflowing_sub(self, other: Self) -> (Self, bool) {
        (0..LIMBS)
            .fold(([0_u64; LIMBS], 0_i128), |(arr, borrow), i| {
                let lhs = i128::from(self.limbs[i]);
                let rhs = i128::from(other.limbs[i]);
                let diff = lhs - rhs + borrow;
                if diff < 0 {
                    let adjusted = diff + (1_i128 << 64);
                    let low = u128_low(u128::try_from(adjusted).unwrap_or(0));
                    (set_limb(arr, i, low), -1)
                } else {
                    let low = u128_low(u128::try_from(diff).unwrap_or(0));
                    (set_limb(arr, i, low), 0)
                }
            })
            .pipe(|(limbs, borrow)| (Self { limbs }, borrow != 0))
    }

    /// Compute `self mod modulus`, where `modulus` is a 256-bit value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DivisionByZero`] if `modulus` is zero.
    pub fn rem_u256(self, modulus: U256) -> Result<U256, Error> {
        if modulus.is_zero() {
            Err(Error::DivisionByZero)
        } else {
            Ok(binary_rem_u256(self, modulus))
        }
    }
}

impl Add for U512 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        self.overflowing_add(other).0
    }
}

impl Sub for U512 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        self.overflowing_sub(other).0
    }
}

impl BitAnd for U512 {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        Self {
            limbs: core::array::from_fn(|i| self.limbs[i] & other.limbs[i]),
        }
    }
}

impl BitOr for U512 {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Self {
            limbs: core::array::from_fn(|i| self.limbs[i] | other.limbs[i]),
        }
    }
}

impl BitXor for U512 {
    type Output = Self;
    fn bitxor(self, other: Self) -> Self {
        Self {
            limbs: core::array::from_fn(|i| self.limbs[i] ^ other.limbs[i]),
        }
    }
}

impl Shl<usize> for U512 {
    type Output = Self;
    fn shl(self, n: usize) -> Self {
        match n {
            0 => self,
            n if n >= 512 => Self::zero(),
            n => {
                let limb_shift = n / 64;
                let bit_shift = n % 64;
                let limbs = core::array::from_fn(|i| {
                    let src = match i.checked_sub(limb_shift) {
                        Some(s) if s < LIMBS => self.limbs[s],
                        Some(_) | None => 0,
                    };
                    let carry = match bit_shift {
                        0 => 0,
                        _ => match i.checked_sub(limb_shift + 1) {
                            Some(s) if s < LIMBS => self.limbs[s] >> (64 - bit_shift),
                            Some(_) | None => 0,
                        },
                    };
                    match bit_shift {
                        0 => src,
                        _ => (src << bit_shift) | carry,
                    }
                });
                Self { limbs }
            }
        }
    }
}

impl Shr<usize> for U512 {
    type Output = Self;
    fn shr(self, n: usize) -> Self {
        match n {
            0 => self,
            n if n >= 512 => Self::zero(),
            n => {
                let limb_shift = n / 64;
                let bit_shift = n % 64;
                let limbs = core::array::from_fn(|i| {
                    let src = match i.checked_add(limb_shift) {
                        Some(s) if s < LIMBS => self.limbs[s],
                        Some(_) | None => 0,
                    };
                    let carry = match bit_shift {
                        0 => 0,
                        _ => match i.checked_add(limb_shift + 1) {
                            Some(s) if s < LIMBS => self.limbs[s] << (64 - bit_shift),
                            Some(_) | None => 0,
                        },
                    };
                    match bit_shift {
                        0 => src,
                        _ => (src >> bit_shift) | carry,
                    }
                });
                Self { limbs }
            }
        }
    }
}

impl PartialOrd for U512 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for U512 {
    fn cmp(&self, other: &Self) -> Ordering {
        (0..LIMBS)
            .rev()
            .find_map(|i| match self.limbs[i].cmp(&other.limbs[i]) {
                Ordering::Equal => None,
                Ordering::Less => Some(Ordering::Less),
                Ordering::Greater => Some(Ordering::Greater),
            })
            .unwrap_or(Ordering::Equal)
    }
}

/// Binary remainder: compute `dividend mod divisor` for 512-bit / 256-bit.
/// Precondition: `divisor != 0`.
fn binary_rem_u256(dividend: U512, divisor: U256) -> U256 {
    let divisor_512 = U512::from_u256(divisor);
    (0..512)
        .rev()
        .fold(U512::zero(), |remainder, i| {
            let bit_val = if dividend.bit(i) {
                U512::one()
            } else {
                U512::zero()
            };
            let shifted = (remainder << 1) | bit_val;
            if shifted >= divisor_512 {
                shifted - divisor_512
            } else {
                shifted
            }
        })
        .low_u256()
}

/// Functional limb update.
fn set_limb(arr: [u64; LIMBS], index: usize, value: u64) -> [u64; LIMBS] {
    core::array::from_fn(|i| if i == index { value } else { arr[i] })
}

/// Pipe extension for functional chaining.
trait Pipe: Sized {
    fn pipe<B>(self, f: impl FnOnce(Self) -> B) -> B {
        f(self)
    }
}

impl<T> Pipe for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_zero() {
        assert!(U512::zero().is_zero());
    }

    #[test]
    fn from_u256_preserves_value() {
        let v = U256::from_le_limbs([1, 2, 3, 4]);
        let w = U512::from_u256(v);
        assert_eq!(w.low_u256(), v);
    }

    #[test]
    fn rem_u256_basic() -> Result<(), Error> {
        let v = U512::from_le_limbs([1000, 0, 0, 0, 0, 0, 0, 0]);
        let m = U256::from_u64(7);
        assert_eq!(v.rem_u256(m)?, U256::from_u64(6));
        Ok(())
    }

    #[test]
    fn rem_u256_of_m_is_zero() -> Result<(), Error> {
        let m = U256::from_hex(
            "4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27",
        )?;
        let v = U512::from_u256(m);
        assert_eq!(v.rem_u256(m)?, U256::zero());
        Ok(())
    }

    #[test]
    fn shl_shr_round_trip() {
        let v = U512::one() << 100;
        assert_eq!(v >> 100, U512::one());
    }
}
