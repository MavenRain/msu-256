//! Montgomery constant Mu and reduction table.
//!
//! Given a 256-bit modulus `m` and `R = 2^144` (`BtoTheN`), computes:
//!
//! - `Mu = 2^144 - (m^(-1) mod 2^144)` (144 bits)
//! - `MontRedTable[i] = floor(((2^i * Mu) mod 2^144) * m / 2^144) + 1`
//!   for `i` in `0..144`.

use crate::bigint::U256;
use crate::error::Error;
use crate::params::LOWER_TRI_BITS;

use super::mod_inverse::mod_inverse;

/// Compute the Montgomery constant `Mu = R - (m^(-1) mod R)` where `R = 2^144`.
///
/// # Errors
///
/// Returns [`Error::ModularInverseDoesNotExist`] if `m` is even
/// (no inverse modulo `2^144` exists).
pub fn compute_mu(modulus: U256) -> Result<U256, Error> {
    let r = U256::one() << LOWER_TRI_BITS;
    // We need m^(-1) mod R where R = 2^144.  The modulus m must be odd.
    let m_reduced = modulus.reduce(r)?;
    let m_inv = mod_inverse(m_reduced, r)?;
    Ok(r - m_inv)
}

/// Compute the Montgomery reduction table.
///
/// `MontRedTable[i] = floor(((2^i * Mu) mod 2^144) * modulus / 2^144) + 1`
///
/// # Errors
///
/// Returns the first error encountered during computation.
pub fn compute_mont_red_table(modulus: U256, mu: U256) -> Result<Vec<U256>, Error> {
    let r_mask = (U256::one() << LOWER_TRI_BITS) - U256::one();
    Ok((0..LOWER_TRI_BITS)
        .map(|i| compute_mont_red_entry(i, modulus, mu, r_mask))
        .collect())
}

/// Compute a single Montgomery reduction table entry.
fn compute_mont_red_entry(i: usize, modulus: U256, mu: U256, r_mask: U256) -> U256 {
    // t1 = 2^i
    let t1 = U256::one() << i;
    // t2 = (t1 * mu) mod 2^144
    let t2_full = t1.widening_mul(mu);
    let t2 = t2_full.low_u256() & r_mask;
    // t3 = t2 * modulus (up to 400 bits), shifted right by 144
    let t3 = t2.widening_mul(modulus);
    // t4 = (t3 >> 144) + 1
    let shifted = t3 >> LOWER_TRI_BITS;
    shifted.low_u256() + U256::one()
}

/// Compute the Montgomery R inverse: `(2^144)^(-1) mod modulus`.
///
/// # Errors
///
/// Returns [`Error::ModularInverseDoesNotExist`] if `gcd(R, modulus) != 1`.
pub fn compute_r_inv(modulus: U256) -> Result<U256, Error> {
    let r = U256::one() << LOWER_TRI_BITS;
    mod_inverse(r, modulus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mu_verifies_montgomery_identity() -> Result<(), Error> {
        // For a small odd modulus m, check that m * (R - mu) ≡ -1 (mod R)
        // i.e. m * m_inv ≡ 1 (mod R) where m_inv = R - mu
        let m = U256::from_u64(0x1234_5679); // odd
        let mu = compute_mu(m)?;
        let r = U256::one() << LOWER_TRI_BITS;
        let m_inv = r - mu;
        let r_mask = r - U256::one();
        let product = m.widening_mul(m_inv).low_u256() & r_mask;
        assert_eq!(product, U256::one());
        Ok(())
    }

    #[test]
    fn r_inv_verifies() -> Result<(), Error> {
        let m = U256::from_hex(
            "4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27",
        )?;
        let r_inv = compute_r_inv(m)?;
        let r = U256::one() << LOWER_TRI_BITS;
        let product = r.mul_mod(r_inv, m)?;
        assert_eq!(product, U256::one());
        Ok(())
    }

    #[test]
    fn mont_red_table_has_correct_length() -> Result<(), Error> {
        let m = U256::from_u64(0x1234_5679);
        let mu = compute_mu(m)?;
        let table = compute_mont_red_table(m, mu)?;
        assert_eq!(table.len(), LOWER_TRI_BITS);
        Ok(())
    }
}
