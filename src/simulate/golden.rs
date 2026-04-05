//! Behavioral golden model: Montgomery squaring via direct big-integer arithmetic.
//!
//! Given an initial value `a_mont` and a modulus, produces a stream of
//! successive Montgomery squares.  Used to verify the MSU circuit.

use std::sync::Arc;

use comp_cat_rs::effect::io::Io;
use comp_cat_rs::effect::stream::Stream;

use crate::bigint::U256;
use crate::domain::MsuConfig;
use crate::error::Error;

/// Produce a stream of Montgomery squares starting from `initial`.
///
/// Each emitted value `v_{n+1}` equals `(v_n^2 * R^(-1)) mod modulus`,
/// i.e. the Montgomery square of the previous.
#[must_use]
pub fn golden_model(initial: U256, config: Arc<MsuConfig>) -> Stream<Error, U256> {
    Stream::unfold(
        initial,
        Arc::new(move |state: U256| {
            let cfg = Arc::clone(&config);
            Io::suspend(move || {
                let squared = state.mul_mod(state, cfg.modulus())?;
                let next = squared.mul_mod(cfg.r_inv(), cfg.modulus())?;
                Ok(Some((next, next)))
            })
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_modulus() -> Result<U256, Error> {
        U256::from_hex("4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27")
    }

    #[test]
    fn golden_produces_expected_sequence() -> Result<(), Error> {
        let config = Arc::new(MsuConfig::generate(test_modulus()?)?);
        let initial = U256::from_u64(0x1234_5678);
        // Compute manually: v_1 = initial^2 * R^(-1) mod m
        let squared = initial.mul_mod(initial, config.modulus())?;
        let expected_1 = squared.mul_mod(config.r_inv(), config.modulus())?;
        // Take first two values
        let results = golden_model(initial, Arc::clone(&config))
            .take(2)
            .collect()
            .run()?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], expected_1);
        Ok(())
    }
}
