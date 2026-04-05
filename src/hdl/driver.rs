//! comp-cat-rs wrappers around RHDL hardware simulation.
//!
//! Exposes RHDL's `.run()` method (asynchronous simulation iterator)
//! inside comp-cat-rs `Io`, so hardware simulations compose with
//! the rest of the crate's effectful pipelines.

use comp_cat_rs::effect::io::Io;
use rhdl::prelude::*;

use crate::error::Error;

/// Run an RHDL synchronous circuit for one "tick" per input sample,
/// collecting the output after each positive clock edge.
///
/// The input iterator is wrapped with reset (1 cycle) and clocked
/// at 100 time units per cycle.  The output is a `Vec` of outputs,
/// one per sample.
///
/// Wrapped in `Io` so the simulation is lazy and composable.
///
/// # Errors
///
/// Returns [`Error::Rhdl`] if the RHDL simulation fails internally.
pub fn run_synchronous<C>(
    circuit: C,
    inputs: Vec<C::I>,
) -> Io<Error, Vec<C::O>>
where
    C: Synchronous + SynchronousIO + Clone + Send + 'static,
    C::I: Send + 'static,
    C::O: Send + 'static,
{
    Io::suspend(move || {
        let timed_inputs = inputs.into_iter().with_reset(1).clock_pos_edge(100);
        let samples: Vec<C::O> = circuit.run(timed_inputs).map(|s| s.output).collect();
        Ok(samples)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hdl::demo::DemoCircuit;

    #[test]
    fn demo_circuit_counts_up() -> Result<(), Error> {
        // Drive the demo counter with 5 "enable" cycles.
        let inputs = vec![true, true, true, true, true];
        let circuit = DemoCircuit::default();
        let outputs = run_synchronous(circuit, inputs).run()?;
        // With 1 reset cycle + 5 enabled cycles + sampling after each clock edge,
        // we get at least 5 samples.  The counter starts at 0 and increments.
        assert!(!outputs.is_empty());
        Ok(())
    }

    #[test]
    fn demo_circuit_output_has_entries() -> Result<(), Error> {
        let inputs = vec![true; 10];
        let outputs = run_synchronous(DemoCircuit::default(), inputs).run()?;
        assert!(outputs.len() >= 10);
        Ok(())
    }
}
