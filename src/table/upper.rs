//! Upper reduction table.
//!
//! `UpperRedTable[i] = 2^(i + 256) mod modulus` for `i` in `0..145`.

use crate::bigint::U256;
use crate::error::Error;
use crate::params::{TARGET_BITS, UPPER_TRI_BITS};

/// Compute the upper reduction table.
///
/// Each entry `i` is `2^(TARGET_BITS + i) mod modulus`.  Computed
/// iteratively by starting with `2^256 mod modulus` and doubling.
///
/// # Errors
///
/// Returns [`Error::DivisionByZero`] if `modulus` is zero.
pub fn compute_upper_red_table(modulus: U256) -> Result<Vec<U256>, Error> {
    if modulus.is_zero() {
        return Err(Error::DivisionByZero);
    }
    // base = 2^TARGET_BITS mod modulus
    let base = compute_two_pow_mod(TARGET_BITS, modulus)?;
    let doubler = U256::from_u64(2);
    // Use try_fold to accumulate the table via repeated doubling mod modulus.
    let (final_table, _) = (0..UPPER_TRI_BITS).try_fold(
        (Vec::with_capacity(UPPER_TRI_BITS), base),
        |(table, current), _| {
            let appended = append_entry(table, current);
            let next = current.mul_mod(doubler, modulus)?;
            Ok::<_, Error>((appended, next))
        },
    )?;
    Ok(final_table)
}

/// Compute `2^exp mod modulus` via square-and-multiply on the exponent bits.
fn compute_two_pow_mod(exp: usize, modulus: U256) -> Result<U256, Error> {
    let two = U256::from_u64(2);
    (0..exp).try_fold(U256::one(), |acc, _| acc.mul_mod(two, modulus))
}

/// Functional vector append.
fn append_entry(mut table: Vec<U256>, entry: U256) -> Vec<U256> {
    table.push(entry);
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_table_has_correct_length() -> Result<(), Error> {
        let m = U256::from_u64(0x1234_5679);
        let table = compute_upper_red_table(m)?;
        assert_eq!(table.len(), UPPER_TRI_BITS);
        Ok(())
    }

    #[test]
    fn first_entry_is_two_pow_target_mod() -> Result<(), Error> {
        let m = U256::from_hex(
            "4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27",
        )?;
        let table = compute_upper_red_table(m)?;
        let expected = compute_two_pow_mod(TARGET_BITS, m)?;
        assert_eq!(table[0], expected);
        Ok(())
    }

    #[test]
    fn successive_entries_differ_by_doubling() -> Result<(), Error> {
        let m = U256::from_u64(0x1234_5679);
        let table = compute_upper_red_table(m)?;
        let doubler = U256::from_u64(2);
        (1..table.len())
            .try_for_each(|i| {
                let expected = table[i - 1].mul_mod(doubler, m)?;
                assert_eq!(table[i], expected);
                Ok::<_, Error>(())
            })?;
        Ok(())
    }
}
