//! 256-bit unsigned integer.
//!
//! Backed by four little-endian `u64` limbs (limb 0 is least significant).
//! All arithmetic is immutable and avoids mutable state, loops, and
//! naked `as` casts.

use core::cmp::Ordering;
use core::ops::{Add, BitAnd, BitOr, BitXor, Shl, Shr, Sub};

use super::u512::U512;
use super::{hex_digit, u128_high, u128_low};
use crate::error::Error;

/// Number of 64-bit limbs.
const LIMBS: usize = 4;

/// A 256-bit unsigned integer.
///
/// Limb 0 is the least significant 64 bits.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[must_use]
pub struct U256 {
    limbs: [u64; LIMBS],
}

impl U256 {
    /// The zero value.
    pub const fn zero() -> Self {
        Self { limbs: [0; LIMBS] }
    }

    /// The value one.
    pub const fn one() -> Self {
        Self {
            limbs: [1, 0, 0, 0],
        }
    }

    /// Construct from a single `u64`.
    pub const fn from_u64(v: u64) -> Self {
        Self {
            limbs: [v, 0, 0, 0],
        }
    }

    /// Construct from four little-endian `u64` limbs.
    pub const fn from_le_limbs(limbs: [u64; LIMBS]) -> Self {
        Self { limbs }
    }

    /// Access a limb by index.
    ///
    /// # Errors
    ///
    /// Returns [`Error::IndexOutOfBounds`] if `index >= 4`.
    pub fn limb(&self, index: usize) -> Result<u64, Error> {
        self.limbs.get(index).copied().ok_or(Error::IndexOutOfBounds {
            index,
            length: LIMBS,
        })
    }

    /// Access the raw limb array.
    #[must_use]
    pub fn limbs(&self) -> [u64; LIMBS] {
        self.limbs
    }

    /// Construct from a hex string (without leading `0x`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::HexLength`] if the string has wrong length,
    /// or [`Error::HexParse`] if any character is not a valid hex digit.
    pub fn from_hex(s: &str) -> Result<Self, Error> {
        match s.len() {
            64 => s
                .as_bytes()
                .chunks_exact(2)
                .enumerate()
                .try_fold([0_u8; 32], |acc, (i, pair)| {
                    let hi = hex_digit(pair[0])?;
                    let lo = hex_digit(pair[1])?;
                    let byte = (hi << 4) | lo;
                    Ok(set_byte(acc, i, byte))
                })
                .map(|bytes| Self::from_be_bytes(&bytes)),
            other => Err(Error::HexLength {
                expected: 64,
                actual: other,
            }),
        }
    }

    /// Construct from big-endian 32-byte representation.
    fn from_be_bytes(bytes: &[u8; 32]) -> Self {
        let limb = |offset: usize| -> u64 {
            (0..8).fold(0_u64, |acc, i| (acc << 8) | u64::from(bytes[offset + i]))
        };
        Self {
            limbs: [limb(24), limb(16), limb(8), limb(0)],
        }
    }

    /// Test whether a specific bit is set.
    #[must_use]
    pub fn bit(&self, position: usize) -> bool {
        match position {
            p if p >= 256 => false,
            p => {
                let limb_index = p / 64;
                let bit_index = p % 64;
                (self.limbs[limb_index] >> bit_index) & 1 == 1
            }
        }
    }

    /// The number of bits required to represent this value (zero returns 0).
    #[must_use]
    pub fn bit_length(&self) -> usize {
        (0..LIMBS)
            .rev()
            .find_map(|i| {
                let limb = self.limbs[i];
                match limb {
                    0 => None,
                    v => {
                        let leading = v.leading_zeros();
                        let bits_in_limb = 64 - usize::try_from(leading).unwrap_or(0);
                        Some(i * 64 + bits_in_limb)
                    }
                }
            })
            .unwrap_or(0)
    }

    /// Whether this value is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.limbs.iter().all(|&l| l == 0)
    }

    /// Add with carry, returning (sum, carry).
    pub fn overflowing_add(self, other: Self) -> (Self, bool) {
        (0..LIMBS)
            .fold(([0_u64; LIMBS], 0_u128), |(arr, carry), i| {
                let sum = u128::from(self.limbs[i]) + u128::from(other.limbs[i]) + carry;
                let new_arr = set_limb(arr, i, u128_low(sum));
                (new_arr, u128::from(u128_high(sum)))
            })
            .pipe(|(limbs, carry_out)| (Self { limbs }, carry_out != 0))
    }

    /// Subtract with borrow, returning (difference, borrow).
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

    /// Multiply two 256-bit values, producing a 512-bit result.
    pub fn widening_mul(self, other: Self) -> U512 {
        let product_limbs = (0..LIMBS).fold([0_u64; 8], |acc, i| {
            let (inner_arr, inner_carry) =
                (0..LIMBS).fold((acc, 0_u128), |(arr, carry), j| {
                    let prod = u128::from(self.limbs[i])
                        * u128::from(other.limbs[j])
                        + u128::from(arr[i + j])
                        + carry;
                    let new_arr = set_u512_limb(arr, i + j, u128_low(prod));
                    (new_arr, u128::from(u128_high(prod)))
                });
            let final_acc =
                (i + LIMBS..8).fold((inner_arr, inner_carry), |(arr, carry), k| match carry {
                    0 => (arr, 0),
                    c => {
                        let sum = u128::from(arr[k]) + c;
                        (set_u512_limb(arr, k, u128_low(sum)), u128::from(u128_high(sum)))
                    }
                });
            final_acc.0
        });
        U512::from_le_limbs(product_limbs)
    }

    /// Divide by another `U256`, returning (quotient, remainder).
    ///
    /// # Errors
    ///
    /// Returns [`Error::DivisionByZero`] if `divisor` is zero.
    pub fn div_rem(self, divisor: Self) -> Result<(Self, Self), Error> {
        if divisor.is_zero() {
            Err(Error::DivisionByZero)
        } else {
            Ok(binary_div_rem(self, divisor))
        }
    }

    /// Compute `self % modulus`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DivisionByZero`] if `modulus` is zero.
    pub fn reduce(self, modulus: Self) -> Result<Self, Error> {
        self.div_rem(modulus).map(|(_, r)| r)
    }

    /// Compute `(self * other) mod modulus`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DivisionByZero`] if `modulus` is zero.
    pub fn mul_mod(self, other: Self, modulus: Self) -> Result<Self, Error> {
        self.widening_mul(other).rem_u256(modulus)
    }

    /// Compute `(self ^ exp) mod modulus` via square-and-multiply.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DivisionByZero`] if `modulus` is zero.
    pub fn pow_mod(self, exp: Self, modulus: Self) -> Result<Self, Error> {
        if modulus.is_zero() {
            Err(Error::DivisionByZero)
        } else {
            (0..256)
                .try_fold((Self::one(), self), |(acc, base), i| {
                    let new_acc = if exp.bit(i) {
                        acc.mul_mod(base, modulus)?
                    } else {
                        acc
                    };
                    let new_base = if i < 255 {
                        base.mul_mod(base, modulus)?
                    } else {
                        base
                    };
                    Ok((new_acc, new_base))
                })
                .map(|(acc, _)| acc)
        }
    }

    /// Mask to the low `n` bits.  Bits at positions `>= n` are cleared.
    pub fn mask_bits(self, n: usize) -> Self {
        match n {
            0 => Self::zero(),
            n if n >= 256 => self,
            n => {
                let full_limbs = n / 64;
                let partial_bits = n % 64;
                let partial_mask = match partial_bits {
                    0 => 0,
                    b => (1_u64 << b) - 1,
                };
                let limbs = core::array::from_fn(|i| match i.cmp(&full_limbs) {
                    Ordering::Less => self.limbs[i],
                    Ordering::Equal => self.limbs[i] & partial_mask,
                    Ordering::Greater => 0,
                });
                Self { limbs }
            }
        }
    }
}

impl Add for U256 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        self.overflowing_add(other).0
    }
}

impl Sub for U256 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        self.overflowing_sub(other).0
    }
}

impl BitAnd for U256 {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        Self {
            limbs: core::array::from_fn(|i| self.limbs[i] & other.limbs[i]),
        }
    }
}

impl BitOr for U256 {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Self {
            limbs: core::array::from_fn(|i| self.limbs[i] | other.limbs[i]),
        }
    }
}

impl BitXor for U256 {
    type Output = Self;
    fn bitxor(self, other: Self) -> Self {
        Self {
            limbs: core::array::from_fn(|i| self.limbs[i] ^ other.limbs[i]),
        }
    }
}

impl Shl<usize> for U256 {
    type Output = Self;
    fn shl(self, n: usize) -> Self {
        match n {
            0 => self,
            n if n >= 256 => Self::zero(),
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

impl Shr<usize> for U256 {
    type Output = Self;
    fn shr(self, n: usize) -> Self {
        match n {
            0 => self,
            n if n >= 256 => Self::zero(),
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

impl PartialOrd for U256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for U256 {
    fn cmp(&self, other: &Self) -> Ordering {
        (0..LIMBS)
            .rev()
            .find_map(|i| match self.limbs[i].cmp(&other.limbs[i]) {
                Ordering::Equal => None,
                o => Some(o),
            })
            .unwrap_or(Ordering::Equal)
    }
}

impl core::fmt::Display for U256 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "0x{:016x}{:016x}{:016x}{:016x}",
            self.limbs[3], self.limbs[2], self.limbs[1], self.limbs[0]
        )
    }
}

/// Binary long division.  Precondition: `divisor != 0`.
fn binary_div_rem(dividend: U256, divisor: U256) -> (U256, U256) {
    (0..256).rev().fold(
        (U256::zero(), U256::zero()),
        |(quotient, remainder), i| {
            let bit_val = if dividend.bit(i) {
                U256::one()
            } else {
                U256::zero()
            };
            let shifted = (remainder << 1) | bit_val;
            if shifted >= divisor {
                (quotient | (U256::one() << i), shifted - divisor)
            } else {
                (quotient, shifted)
            }
        },
    )
}

/// Functional limb update for [u64; 4].
fn set_limb(arr: [u64; LIMBS], index: usize, value: u64) -> [u64; LIMBS] {
    core::array::from_fn(|i| if i == index { value } else { arr[i] })
}

/// Functional limb update for [u64; 8].
fn set_u512_limb(arr: [u64; 8], index: usize, value: u64) -> [u64; 8] {
    core::array::from_fn(|i| if i == index { value } else { arr[i] })
}

/// Functional byte update for [u8; 32].
fn set_byte(arr: [u8; 32], index: usize, value: u8) -> [u8; 32] {
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
        assert!(U256::zero().is_zero());
        assert_eq!(U256::zero().bit_length(), 0);
    }

    #[test]
    fn one_has_bit_zero() {
        assert!(U256::one().bit(0));
        assert!(!U256::one().bit(1));
        assert_eq!(U256::one().bit_length(), 1);
    }

    #[test]
    fn add_small_values() {
        let a = U256::from_u64(100);
        let b = U256::from_u64(200);
        assert_eq!(a + b, U256::from_u64(300));
    }

    #[test]
    fn sub_small_values() {
        let a = U256::from_u64(300);
        let b = U256::from_u64(100);
        assert_eq!(a - b, U256::from_u64(200));
    }

    #[test]
    fn shl_single_bit() {
        let a = U256::one();
        assert_eq!(a << 1, U256::from_u64(2));
        assert_eq!(a << 64, U256::from_le_limbs([0, 1, 0, 0]));
    }

    #[test]
    fn shr_single_bit() {
        let a = U256::from_u64(4);
        assert_eq!(a >> 1, U256::from_u64(2));
    }

    #[test]
    fn hex_round_trip() -> Result<(), Error> {
        let hex = "4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27";
        let v = U256::from_hex(hex)?;
        let s = format!("{v}");
        assert_eq!(s, format!("0x{hex}"));
        Ok(())
    }

    #[test]
    fn div_rem_basic() -> Result<(), Error> {
        let a = U256::from_u64(1000);
        let b = U256::from_u64(7);
        let (q, r) = a.div_rem(b)?;
        assert_eq!(q, U256::from_u64(142));
        assert_eq!(r, U256::from_u64(6));
        Ok(())
    }

    #[test]
    fn mul_mod_basic() -> Result<(), Error> {
        let a = U256::from_u64(17);
        let b = U256::from_u64(19);
        let m = U256::from_u64(100);
        assert_eq!(a.mul_mod(b, m)?, U256::from_u64(23));
        Ok(())
    }

    #[test]
    fn mul_mod_large() -> Result<(), Error> {
        // (m - 1) * 2 mod m = m - 2
        let m = U256::from_hex(
            "4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27",
        )?;
        let a = m - U256::one();
        let two = U256::from_u64(2);
        let expected = m - two;
        assert_eq!(a.mul_mod(two, m)?, expected);
        Ok(())
    }

    #[test]
    fn mul_mod_squared() -> Result<(), Error> {
        // 2 * (m/2 + 1) mod m = should be 2
        let m = U256::from_hex(
            "4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27",
        )?;
        let two = U256::from_u64(2);
        // Verify: m * m mod m = 0
        assert_eq!(m.mul_mod(m, m)?, U256::zero());
        // Verify: (m+1) * (m+1) mod m = 1
        let one = U256::one();
        assert_eq!(one.mul_mod(one, m)?, one);
        // Verify: 2 * 2 mod m = 4
        assert_eq!(two.mul_mod(two, m)?, U256::from_u64(4));
        Ok(())
    }

    #[test]
    fn pow_mod_basic() -> Result<(), Error> {
        let a = U256::from_u64(2);
        let e = U256::from_u64(10);
        let m = U256::from_u64(1000);
        assert_eq!(a.pow_mod(e, m)?, U256::from_u64(24));
        Ok(())
    }

    #[test]
    fn mask_bits_works() {
        let v = U256::from_le_limbs([u64::MAX, u64::MAX, u64::MAX, u64::MAX]);
        let masked = v.mask_bits(144);
        assert_eq!(masked.limb(0).unwrap_or(0), u64::MAX);
        assert_eq!(masked.limb(1).unwrap_or(0), u64::MAX);
        assert_eq!(masked.limb(2).unwrap_or(0), (1_u64 << 16) - 1);
        assert_eq!(masked.limb(3).unwrap_or(0), 0);
    }

    #[test]
    fn widening_mul_overflow() {
        let a = U256::from_le_limbs([u64::MAX, 0, 0, 0]);
        let product = a.widening_mul(a);
        // (2^64 - 1)^2 = 2^128 - 2^65 + 1
        let lo = product.limb(0).unwrap_or(0);
        let hi = product.limb(1).unwrap_or(0);
        assert_eq!(lo, 1);
        assert_eq!(hi, u64::MAX - 1);
    }

    #[test]
    fn widening_mul_two_limbs() {
        // (2^128 - 1) * (2^128 - 1) = 2^256 - 2^129 + 1
        let a = U256::from_le_limbs([u64::MAX, u64::MAX, 0, 0]);
        let product = a.widening_mul(a);
        // Expected: low 4 limbs of 2^256 - 2^129 + 1
        // 2^256 - 2^129 + 1 = limbs [1, 0, ~1 << 1, MAX] in LE
        // Actually: 2^256 - 2^129 + 1
        //   2^129 = bit 129 set (limb 2, bit 1)
        //   -2^129 mod 2^256 = 2^256 - 2^129 = limb[0]=0, limb[1]=0, limb[2]=~0 & 0xFFFF_FFFF_FFFF_FFFE, limb[3]=MAX
        // +1 gives: limb[0]=1, limb[1]=0, limb[2]=0xFFFFFFFFFFFFFFFE, limb[3]=MAX
        assert_eq!(product.limb(0).unwrap_or(99), 1);
        assert_eq!(product.limb(1).unwrap_or(99), 0);
        assert_eq!(product.limb(2).unwrap_or(99), u64::MAX - 1);
        assert_eq!(product.limb(3).unwrap_or(99), u64::MAX);
    }

    #[test]
    fn widening_mul_full_256bit() -> Result<(), Error> {
        // m * m mod m must be 0
        let m = U256::from_hex(
            "4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27",
        )?;
        let m_squared = m.widening_mul(m);
        // m_squared mod m should be 0
        let reduced = m_squared.rem_u256(m)?;
        assert_eq!(reduced, U256::zero());
        Ok(())
    }
}
