//! Extended Euclidean algorithm for modular inverse.
//!
//! Given `a` and `m`, computes `a^(-1) mod m`: the value `z` such that
//! `a * z ≡ 1 (mod m)`.  Fails if `gcd(a, m) != 1`.
//!
//! Uses a modular-arithmetic formulation: both `t` and `new_t` are
//! maintained in `[0, m)` throughout.  All intermediates use `U512`
//! for the `q * new_t` product, which is then reduced modulo `m`.

use crate::bigint::U256;
use crate::error::Error;

/// Compute `a^(-1) mod m` via the extended Euclidean algorithm.
///
/// # Errors
///
/// Returns [`Error::ModularInverseDoesNotExist`] if `gcd(a, m) != 1`.
/// Returns [`Error::DivisionByZero`] if `m` is zero.
pub fn mod_inverse(a: U256, m: U256) -> Result<U256, Error> {
    if m.is_zero() {
        Err(Error::DivisionByZero)
    } else if m == U256::one() {
        Ok(U256::zero())
    } else {
        let a_reduced = a.reduce(m)?;
        if a_reduced.is_zero() {
            Err(Error::ModularInverseDoesNotExist)
        } else {
            iterate_inverse(a_reduced, m)
        }
    }
}

/// State: `(t, new_t, r, new_r)`.  `t`, `new_t` in `[0, m)`.
#[derive(Clone, Copy)]
struct State {
    t: U256,
    new_t: U256,
    r: U256,
    new_r: U256,
}

/// Outcome of a single iteration: either terminate successfully, or error out.
enum Step {
    Done(Box<State>),
    Failed(Box<Error>),
}

fn iterate_inverse(a: U256, m: U256) -> Result<U256, Error> {
    let init = State {
        t: U256::zero(),
        new_t: U256::one(),
        r: m,
        new_r: a,
    };
    // Bound: 512 iterations is safe for 256-bit inputs.
    let step_result = (0..512).try_fold(init, |state, _| {
        if state.new_r.is_zero() {
            Err(Step::Done(Box::new(state)))
        } else {
            advance(&state, m).map_err(|e| Step::Failed(Box::new(e)))
        }
    });
    match step_result {
        Ok(state) => finalize(&state),
        Err(Step::Done(state)) => finalize(&state),
        Err(Step::Failed(e)) => Err(*e),
    }
}

/// Given a terminated GCD state, produce the inverse (if gcd == 1) or error.
fn finalize(state: &State) -> Result<U256, Error> {
    if state.r == U256::one() {
        Ok(state.t)
    } else {
        Err(Error::ModularInverseDoesNotExist)
    }
}

/// Advance the GCD state by one iteration.
fn advance(state: &State, m: U256) -> Result<State, Error> {
    let (q, _) = state.r.div_rem(state.new_r)?;
    let q_times_new_t = q.mul_mod(state.new_t, m)?;
    let updated_new_t = modular_sub(state.t, q_times_new_t, m);
    let updated_new_r = state.r - q.widening_mul(state.new_r).low_u256();
    Ok(State {
        t: state.new_t,
        new_t: updated_new_t,
        r: state.new_r,
        new_r: updated_new_r,
    })
}

/// Compute `(a - b) mod m` where `a, b in [0, m)`.
fn modular_sub(a: U256, b: U256, m: U256) -> U256 {
    if a >= b {
        a - b
    } else {
        m - (b - a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_of_three_mod_seven() -> Result<(), Error> {
        // 3 * 5 = 15 = 2 * 7 + 1, so 3^(-1) mod 7 = 5
        let result = mod_inverse(U256::from_u64(3), U256::from_u64(7))?;
        assert_eq!(result, U256::from_u64(5));
        Ok(())
    }

    #[test]
    fn inverse_of_two_mod_eleven() -> Result<(), Error> {
        // 2 * 6 = 12 = 1 * 11 + 1, so 2^(-1) mod 11 = 6
        let result = mod_inverse(U256::from_u64(2), U256::from_u64(11))?;
        assert_eq!(result, U256::from_u64(6));
        Ok(())
    }

    #[test]
    fn no_inverse_when_not_coprime() {
        // gcd(6, 9) = 3, no inverse exists
        let result = mod_inverse(U256::from_u64(6), U256::from_u64(9));
        assert!(result.is_err());
    }

    #[test]
    fn inverse_verification() -> Result<(), Error> {
        let a = U256::from_u64(17);
        let m = U256::from_u64(101);
        let inv = mod_inverse(a, m)?;
        let product = a.mul_mod(inv, m)?;
        assert_eq!(product, U256::one());
        Ok(())
    }

    #[test]
    fn inverse_large_modulus() -> Result<(), Error> {
        let m = U256::from_hex(
            "4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27",
        )?;
        let a = U256::from_u64(0x1234_5678);
        let inv = mod_inverse(a, m)?;
        let product = a.mul_mod(inv, m)?;
        println!("a = {a}");
        println!("m = {m}");
        println!("inv = {inv}");
        println!("a * inv mod m = {product}");
        assert_eq!(product, U256::one());
        Ok(())
    }

    #[test]
    fn r_inverse_against_msu_modulus() -> Result<(), Error> {
        let m = U256::from_hex(
            "4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27",
        )?;
        let r = U256::one() << 144;
        let r_inv = mod_inverse(r, m)?;
        let product = r.mul_mod(r_inv, m)?;
        assert_eq!(product, U256::one());
        Ok(())
    }
}
