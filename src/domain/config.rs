//! MSU configuration: modulus plus precomputed Montgomery and upper tables.

use crate::bigint::U256;
use crate::error::Error;
use crate::params::LOWER_TRI_BITS;
use crate::table::montgomery::{compute_mont_red_table, compute_mu, compute_r_inv};
use crate::table::upper::compute_upper_red_table;

/// The 256-bit MSU modulus, modulo-R constants, and lookup tables.
#[derive(Clone, Debug)]
#[must_use]
pub struct MsuConfig {
    modulus: U256,
    r: U256,
    mu: U256,
    r_inv: U256,
    mont_red_table: Vec<U256>,
    upper_red_table: Vec<U256>,
}

impl MsuConfig {
    /// Generate all constants and tables for the given modulus.
    ///
    /// # Errors
    ///
    /// Returns an error if the modulus is zero, even, or otherwise
    /// not coprime with `R = 2^144`.
    pub fn generate(modulus: U256) -> Result<Self, Error> {
        let r = U256::one() << LOWER_TRI_BITS;
        let mu = compute_mu(modulus)?;
        let r_inv = compute_r_inv(modulus)?;
        let mont_red_table = compute_mont_red_table(modulus, mu)?;
        let upper_red_table = compute_upper_red_table(modulus)?;
        Ok(Self {
            modulus,
            r,
            mu,
            r_inv,
            mont_red_table,
            upper_red_table,
        })
    }

    /// The modulus.
    pub fn modulus(&self) -> U256 {
        self.modulus
    }

    /// The Montgomery radix `R = 2^144`.
    pub fn r(&self) -> U256 {
        self.r
    }

    /// The Montgomery constant `Mu`.
    pub fn mu(&self) -> U256 {
        self.mu
    }

    /// The Montgomery R inverse: `(2^144)^(-1) mod modulus`.
    pub fn r_inv(&self) -> U256 {
        self.r_inv
    }

    /// The Montgomery reduction table (144 entries).
    pub fn mont_red_table(&self) -> &[U256] {
        &self.mont_red_table
    }

    /// The upper reduction table (145 entries).
    pub fn upper_red_table(&self) -> &[U256] {
        &self.upper_red_table
    }

    /// Convert a value to Montgomery form: `a * R mod modulus`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DivisionByZero`] if the modulus is zero.
    pub fn to_montgomery(&self, a: U256) -> Result<U256, Error> {
        a.mul_mod(self.r, self.modulus)
    }

    /// Convert a value from Montgomery form: `a * R^(-1) mod modulus`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DivisionByZero`] if the modulus is zero.
    pub fn from_montgomery(&self, a: U256) -> Result<U256, Error> {
        a.mul_mod(self.r_inv, self.modulus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{LOWER_TRI_BITS, UPPER_TRI_BITS};

    fn test_modulus() -> Result<U256, Error> {
        U256::from_hex("4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27")
    }

    #[test]
    fn config_generates_successfully() -> Result<(), Error> {
        let config = MsuConfig::generate(test_modulus()?)?;
        assert_eq!(config.mont_red_table().len(), LOWER_TRI_BITS);
        assert_eq!(config.upper_red_table().len(), UPPER_TRI_BITS);
        Ok(())
    }

    #[test]
    fn montgomery_round_trip() -> Result<(), Error> {
        let config = MsuConfig::generate(test_modulus()?)?;
        let a = U256::from_u64(0x1234_5678);
        let mont = config.to_montgomery(a)?;
        let back = config.from_montgomery(mont)?;
        assert_eq!(back, a);
        Ok(())
    }
}
