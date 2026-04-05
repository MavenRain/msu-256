//! One MSU squaring step: `a_mont -> a_mont^2 * R^(-1) mod m`.
//!
//! Combines the squarer (polynomial squaring + triangle split) and the
//! reducer (Montgomery + pass-through + upper reduction) into a single
//! pure function.  This is the reference algorithm that the RHDL
//! circuit must match.

use crate::bigint::{U256, U512};
use crate::domain::MsuConfig;
use crate::error::Error;
use crate::params::{LOWER_TRI_BITS, TARGET_BITS, UPPER_TRI_BITS};

/// Apply one Montgomery squaring step.
///
/// Given `a_mont` (a value in Montgomery form with `a_mont < modulus`),
/// returns `(a_mont^2 * R^(-1)) mod modulus = (a^2)_mont`.
///
/// # Errors
///
/// Returns an error if the underlying big-integer reduction fails.
pub fn msu_step(a_mont: U256, config: &MsuConfig) -> Result<U256, Error> {
    // 1. Square: produces U512.  Since a_mont < m < 2^256, squared < 2^512.
    let squared = a_mont.widening_mul(a_mont);

    // 2. Split into lower [0, 144), mid [144, 400), upper [400, ...).
    let lower_mask = (U256::one() << LOWER_TRI_BITS) - U256::one();
    let lower = squared.low_u256() & lower_mask;
    let mid = (squared >> LOWER_TRI_BITS).low_u256();
    let upper = (squared >> (LOWER_TRI_BITS + TARGET_BITS)).low_u256();

    // 3. Sum selected Montgomery table entries (in U512 to avoid overflow).
    let lower_sum = (0..LOWER_TRI_BITS).fold(U512::zero(), |acc, i| {
        if lower.bit(i) {
            acc + U512::from_u256(config.mont_red_table()[i])
        } else {
            acc
        }
    });

    // 4. Sum selected upper reduction table entries.
    let upper_sum = (0..UPPER_TRI_BITS).fold(U512::zero(), |acc, i| {
        if upper.bit(i) {
            acc + U512::from_u256(config.upper_red_table()[i])
        } else {
            acc
        }
    });

    // 5. Total = lower_sum + mid + upper_sum.
    let total = lower_sum + U512::from_u256(mid) + upper_sum;

    // 6. Reduce mod modulus.
    total.rem_u256(config.modulus())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_modulus() -> Result<U256, Error> {
        U256::from_hex("4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27")
    }

    #[test]
    fn msu_step_zero_is_zero() -> Result<(), Error> {
        let config = MsuConfig::generate(test_modulus()?)?;
        let result = msu_step(U256::zero(), &config)?;
        assert_eq!(result, U256::zero());
        Ok(())
    }

    #[test]
    fn msu_step_matches_mont_square_formula() -> Result<(), Error> {
        // For arbitrary a_mont < m, msu_step should compute
        //   (a_mont^2 * R^(-1)) mod m
        // Verify directly via big-integer arithmetic.
        let config = MsuConfig::generate(test_modulus()?)?;
        let a_mont = U256::from_u64(0x1234_5678);
        let msu_result = msu_step(a_mont, &config)?;
        // Expected: a_mont^2 * R^(-1) mod m
        let squared_mod = a_mont.mul_mod(a_mont, config.modulus())?;
        let expected = squared_mod.mul_mod(config.r_inv(), config.modulus())?;
        assert_eq!(msu_result, expected);
        Ok(())
    }

    #[test]
    fn msu_step_large_value() -> Result<(), Error> {
        let config = MsuConfig::generate(test_modulus()?)?;
        // Large value close to modulus
        let a_mont = config.modulus() - U256::from_u64(1);
        let msu_result = msu_step(a_mont, &config)?;
        let squared_mod = a_mont.mul_mod(a_mont, config.modulus())?;
        let expected = squared_mod.mul_mod(config.r_inv(), config.modulus())?;
        assert_eq!(msu_result, expected);
        Ok(())
    }
}
