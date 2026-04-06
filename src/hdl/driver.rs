//! comp-cat-rs wrappers around hdl-cat simulation and Verilog emission.
//!
//! Every function in this module returns an [`Io`] so that hardware
//! simulation and code generation compose lazily with the rest of
//! the crate's effectful pipelines.  Call `.run()` only at the
//! outermost boundary.
//!
//! ## Simulation walkthrough
//!
//! [`simulate`] drives a [`Sync`] machine for a given number of
//! cycles.  Each cycle produces a [`TimedSample<BitSeq>`]:
//!
//! - `sample.cycle()` — a [`Cycle`] newtype whose `.index()` gives
//!   the zero-based cycle number.
//! - `sample.value()` — a [`BitSeq`] (flat boolean vector) holding
//!   the machine's non-state output wires for that cycle.
//!
//! To recover a typed hardware value, decode the [`BitSeq`] through
//! [`Bits::from_bits_seq`]:
//!
//! ```
//! # fn main() -> Result<(), msu_256::Error> {
//! use msu_256::hdl::{demo, driver};
//! use hdl_cat::prelude::Bits;
//! use hdl_cat::kind::Hw;
//!
//! let counter = demo::demo_counter()?;
//! let samples = driver::simulate(counter, 8).run()?;
//!
//! // One sample per cycle, indexed from zero.
//! assert_eq!(samples.len(), 8);
//! assert_eq!(samples[0].cycle().index(), 0);
//! assert_eq!(samples[7].cycle().index(), 7);
//!
//! // Decode each BitSeq into a Bits<64>, then into u128.
//! let values: Vec<u128> = samples
//!     .iter()
//!     .map(|s| Bits::<64>::from_bits_seq(s.value()).map(Bits::to_u128))
//!     .collect::<Result<Vec<_>, _>>()?;
//! assert_eq!(values, vec![0, 1, 2, 3, 4, 5, 6, 7]);
//! # Ok(()) }
//! ```
//!
//! Zero cycles produces an empty result (no simulation work):
//!
//! ```
//! # fn main() -> Result<(), msu_256::Error> {
//! use msu_256::hdl::{demo, driver};
//!
//! let counter = demo::demo_counter()?;
//! let samples = driver::simulate(counter, 0).run()?;
//! assert!(samples.is_empty());
//! # Ok(()) }
//! ```
//!
//! ## Verilog emission walkthrough
//!
//! [`emit_verilog`] composes two hdl-cat steps via
//! [`Io::flat_map`]: first `verilog::emit_sync_graph` builds a
//! typed Verilog AST ([`Module`]), then `module.render()` pretty-
//! prints it.  Both steps stay inside [`Io`]; the caller calls
//! `.run()` once at the boundary.
//!
//! The emitted module follows a standard pattern:
//!
//! 1. **Port list** — `clk` and `rst` inputs, plus data output
//!    ports.  State wires that double as outputs become
//!    `output reg`.
//! 2. **Internal wires** — intermediate combinational signals
//!    (e.g. the constant `1`, the adder output).
//! 3. **Continuous assigns** — one `assign` per IR instruction.
//! 4. **`always_ff` blocks** — one per state register, with
//!    synchronous reset to the machine's initial state.
//!
//! ```
//! # fn main() -> Result<(), msu_256::Error> {
//! use msu_256::hdl::{demo, driver};
//!
//! let counter = demo::demo_counter()?;
//! let text = driver::emit_verilog(&counter, "demo_ctr").run()?;
//!
//! // The module declaration opens with the given name.
//! assert!(text.contains("module demo_ctr"));
//!
//! // Ports: clk, rst, and the 64-bit count register.
//! assert!(text.contains("input clk"));
//! assert!(text.contains("input rst"));
//! assert!(text.contains("output reg [63:0]"));
//!
//! // Body: the constant 1 as a 64-bit literal,
//! // the adder producing next_state, and the
//! // always_ff block driving the register.
//! assert!(text.contains("64'd1"));
//! assert!(text.contains("+"));
//! assert!(text.contains("always_ff @(posedge clk)"));
//!
//! // Reset drives the register to zero.
//! assert!(text.contains("64'd0"));
//!
//! // The module ends cleanly.
//! assert!(text.contains("endmodule"));
//! # Ok(()) }
//! ```

use comp_cat_rs::effect::io::Io;
use hdl_cat::prelude::*;
use hdl_cat::sim::TimedSample;

use crate::error::Error;

/// Simulate an hdl-cat [`Sync`] machine for `cycles` clock ticks.
///
/// For machines with no data input ([`CircuitUnit`]), each cycle
/// receives an empty [`BitSeq`].  The returned vector contains one
/// [`TimedSample`] per cycle whose `value` holds the machine's
/// non-state output wires as a flat [`BitSeq`].
///
/// Decode the [`BitSeq`] back to typed values with
/// [`Bits::from_bits_seq`]:
///
/// ```
/// # fn main() -> Result<(), msu_256::Error> {
/// use msu_256::hdl::{demo, driver};
/// use hdl_cat::prelude::Bits;
/// use hdl_cat::kind::Hw;
///
/// let counter = demo::demo_counter()?;
/// let samples = driver::simulate(counter, 4).run()?;
///
/// let third_cycle = &samples[2];
/// assert_eq!(third_cycle.cycle().index(), 2);
///
/// let count = Bits::<64>::from_bits_seq(third_cycle.value())?;
/// assert_eq!(count.to_u128(), 2);
/// # Ok(()) }
/// ```
///
/// # Errors
///
/// Returns [`Error::HdlCat`] if the simulation fails (e.g. width
/// mismatch between initial state and state wires).
#[must_use]
pub fn simulate<S, I, O>(
    machine: Sync<S, I, O>,
    cycles: usize,
) -> Io<Error, Vec<TimedSample<hdl_cat::kind::BitSeq>>>
where
    S: 'static,
    I: 'static,
    O: 'static,
{
    let inputs: Vec<hdl_cat::kind::BitSeq> =
        (0..cycles).map(|_| hdl_cat::kind::BitSeq::new()).collect();
    Testbench::new(machine)
        .run(inputs)
        .map_error(Error::from)
}

/// Emit a rendered Verilog module from a [`Sync`] machine.
///
/// Internally composes two lazy steps via [`Io::flat_map`]:
///
/// 1. `verilog::emit_sync_graph` inspects the machine's IR graph,
///    state wire count, input/output wires, and initial state to
///    build a typed Verilog [`Module`] AST.
/// 2. `module.render()` pretty-prints the AST to a `String`.
///
/// The emitted module has:
///
/// - `input clk` and `input rst` ports (synchronous reset)
/// - `output reg [W-1:0]` for each state wire that is also an
///   output (e.g. the counter's count register)
/// - `wire` declarations for intermediate combinational signals
/// - `assign` statements for each IR instruction
/// - `always_ff @(posedge clk)` blocks that reset state registers
///   to the machine's initial state on `rst` and advance them to
///   their next-state expressions otherwise
///
/// # Errors
///
/// Returns [`Error::HdlCat`] if the initial-state width does not
/// match the state wire widths, or if rendering fails.
///
/// # Examples
///
/// Emit and inspect a 64-bit counter module:
///
/// ```
/// # fn main() -> Result<(), msu_256::Error> {
/// use msu_256::hdl::{demo, driver};
///
/// let counter = demo::demo_counter()?;
/// let text = driver::emit_verilog(&counter, "msu_counter64").run()?;
///
/// // The top-level structure: module header, ports, body, footer.
/// assert!(text.contains("module msu_counter64"));
/// assert!(text.contains("input clk"));
/// assert!(text.contains("input rst"));
/// assert!(text.contains("output reg [63:0]"));
/// assert!(text.contains("always_ff @(posedge clk)"));
/// assert!(text.contains("endmodule"));
///
/// // The increment logic: a 64-bit constant 1 and an adder.
/// assert!(text.contains("64'd1"));
/// assert!(text.contains("+"));
///
/// // Reset drives the register to 64'd0.
/// assert!(text.contains("64'd0"));
/// # Ok(()) }
/// ```
///
/// Use the same machine for both emission and simulation:
///
/// ```
/// # fn main() -> Result<(), msu_256::Error> {
/// use msu_256::hdl::{demo, driver};
/// use hdl_cat::prelude::Bits;
/// use hdl_cat::kind::Hw;
///
/// let counter = demo::demo_counter()?;
///
/// // Borrow for Verilog, then consume for simulation.
/// let verilog = driver::emit_verilog(&counter, "dual_use").run()?;
/// assert!(verilog.contains("module dual_use"));
///
/// let samples = driver::simulate(counter, 3).run()?;
/// let values: Vec<u128> = samples
///     .iter()
///     .map(|s| Bits::<64>::from_bits_seq(s.value()).map(Bits::to_u128))
///     .collect::<Result<Vec<_>, _>>()?;
/// assert_eq!(values, vec![0, 1, 2]);
/// # Ok(()) }
/// ```
#[must_use]
pub fn emit_verilog<S, I, O>(
    machine: &Sync<S, I, O>,
    name: &str,
) -> Io<Error, String> {
    let name_owned = name.to_string();
    verilog::emit_sync_graph(
        machine.graph(),
        &name_owned,
        machine.state_wire_count(),
        machine.input_wires(),
        machine.output_wires(),
        machine.initial_state(),
    )
    .flat_map(|module| module.render())
    .map_error(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hdl::demo::demo_counter;

    #[test]
    fn demo_counter_counts_up() -> Result<(), Error> {
        let counter = demo_counter()?;
        let samples = simulate(counter, 5).run()?;
        assert_eq!(samples.len(), 5);

        let values: Vec<u128> = samples
            .iter()
            .map(|s| {
                hdl_cat::bits::Bits::<64>::from_bits_seq(s.value())
                    .map(hdl_cat::bits::Bits::to_u128)
            })
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(values, vec![0, 1, 2, 3, 4]);
        Ok(())
    }

    #[test]
    fn demo_counter_simulation_has_cycle_indices() -> Result<(), Error> {
        let counter = demo_counter()?;
        let samples = simulate(counter, 3).run()?;
        assert_eq!(samples[0].cycle().index(), 0);
        assert_eq!(samples[1].cycle().index(), 1);
        assert_eq!(samples[2].cycle().index(), 2);
        Ok(())
    }

    #[test]
    fn demo_counter_emits_verilog_with_clk_rst() -> Result<(), Error> {
        let counter = demo_counter()?;
        let text = emit_verilog(&counter, "test_counter").run()?;
        assert!(text.contains("module test_counter"));
        assert!(text.contains("input clk"));
        assert!(text.contains("input rst"));
        assert!(text.contains("always_ff @(posedge clk)"));
        Ok(())
    }

    #[test]
    fn demo_counter_verilog_has_output_reg_and_increment() -> Result<(), Error> {
        let counter = demo_counter()?;
        let text = emit_verilog(&counter, "counter_v").run()?;
        assert!(text.contains("output reg [63:0]"));
        assert!(text.contains("64'd1"));
        assert!(text.contains('+'));
        assert!(text.contains("64'd0"));
        assert!(text.contains("endmodule"));
        Ok(())
    }

    #[test]
    fn zero_cycles_produces_empty_output() -> Result<(), Error> {
        let counter = demo_counter()?;
        let samples = simulate(counter, 0).run()?;
        assert!(samples.is_empty());
        Ok(())
    }

    #[test]
    fn emit_then_simulate_same_machine() -> Result<(), Error> {
        let counter = demo_counter()?;
        let verilog = emit_verilog(&counter, "dual").run()?;
        assert!(verilog.contains("module dual"));

        let samples = simulate(counter, 4).run()?;
        let values: Vec<u128> = samples
            .iter()
            .map(|s| {
                hdl_cat::bits::Bits::<64>::from_bits_seq(s.value())
                    .map(hdl_cat::bits::Bits::to_u128)
            })
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(values, vec![0, 1, 2, 3]);
        Ok(())
    }
}
