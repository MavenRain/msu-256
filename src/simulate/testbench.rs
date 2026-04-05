//! MSU testbench: compares the MSU algorithmic model against the golden model.

use std::sync::Arc;

use comp_cat_rs::effect::io::Io;
use comp_cat_rs::effect::stream::Stream;

use crate::bigint::U256;
use crate::domain::MsuConfig;
use crate::error::Error;

use super::golden::golden_model;
use super::msu_step::msu_step;

/// Outcome of the MSU testbench.
#[derive(Clone, Debug)]
#[must_use]
pub struct TestResult {
    iterations: usize,
    first_mismatch: Option<usize>,
}

impl TestResult {
    /// Whether all iterations matched the golden model.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.first_mismatch.is_none()
    }

    /// Number of iterations run.
    #[must_use]
    pub fn iterations(&self) -> usize {
        self.iterations
    }

    /// Index of the first mismatched iteration, if any.
    #[must_use]
    pub fn first_mismatch(&self) -> Option<usize> {
        self.first_mismatch
    }

    /// Assert that the result passed; return an error otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SimulationMismatch`] if there was a mismatch.
    pub fn assert_passed(&self) -> Result<(), Error> {
        match self.first_mismatch {
            Some(iteration) => Err(Error::SimulationMismatch { iteration }),
            None => Ok(()),
        }
    }
}

/// Produce a stream of MSU squaring results.
fn msu_stream(initial: U256, config: Arc<MsuConfig>) -> Stream<Error, U256> {
    Stream::unfold(
        initial,
        Arc::new(move |state: U256| {
            let cfg = Arc::clone(&config);
            Io::suspend(move || {
                let next = msu_step(state, &cfg)?;
                Ok(Some((next, next)))
            })
        }),
    )
}

/// Run the MSU testbench for `iterations` squaring steps.
///
/// Uses a deterministic initial value derived from the modulus.
/// Compares the algorithmic MSU step output against the golden model.
///
/// # Errors
///
/// Returns an error if config generation or simulation fails.
#[must_use]
pub fn run_testbench(iterations: usize) -> Io<Error, TestResult> {
    Io::suspend(move || {
        let modulus = U256::from_hex(
            "4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27",
        )?;
        let config = Arc::new(MsuConfig::generate(modulus)?);
        // Deterministic initial value: x mod m where x is some non-trivial constant
        let initial_raw = U256::from_hex(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )?;
        let initial = initial_raw.reduce(config.modulus())?;
        // Convert to Montgomery form.
        let initial_mont = config.to_montgomery(initial)?;
        // Run both streams.
        let msu_results = msu_stream(initial_mont, Arc::clone(&config))
            .take(iterations)
            .collect()
            .run()?;
        let golden_results = golden_model(initial_mont, Arc::clone(&config))
            .take(iterations)
            .collect()
            .run()?;
        // Compare.
        let first_mismatch = msu_results
            .iter()
            .zip(golden_results.iter())
            .enumerate()
            .find_map(|(i, (m, g))| if m == g { None } else { Some(i) });
        Ok(TestResult {
            iterations,
            first_mismatch,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testbench_100_iterations_passes() -> Result<(), Error> {
        let result = run_testbench(100).run()?;
        result.assert_passed()?;
        assert_eq!(result.iterations(), 100);
        Ok(())
    }

    #[test]
    fn testbench_single_iteration_passes() -> Result<(), Error> {
        let result = run_testbench(1).run()?;
        result.assert_passed()?;
        Ok(())
    }
}
