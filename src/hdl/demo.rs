//! A minimal hdl-cat demo: 64-bit free-running counter.
//!
//! This module establishes the integration pattern that a full MSU
//! circuit would extend: build a [`hdl_cat::sync::Sync`] machine,
//! simulate it with [`driver::simulate`], and emit Verilog with
//! [`driver::emit_verilog`].
//!
//! ## Simulation: decoding cycle outputs
//!
//! Each simulation cycle produces a [`TimedSample`] whose `value()`
//! is a raw [`BitSeq`] (a flat boolean vector).  To recover typed
//! values, decode through [`Bits::from_bits_seq`]:
//!
//! ```
//! # fn main() -> Result<(), msu_256::Error> {
//! use msu_256::hdl::{demo, driver};
//! use hdl_cat::prelude::Bits;
//! use hdl_cat::kind::Hw;
//!
//! let counter = demo::demo_counter()?;
//! let samples = driver::simulate(counter, 6).run()?;
//!
//! // Each sample carries a cycle index and the output BitSeq.
//! // The counter has no data input, so outputs are purely
//! // state-driven: the current count before incrementing.
//! let values: Vec<u128> = samples
//!     .iter()
//!     .map(|s| Bits::<64>::from_bits_seq(s.value()).map(Bits::to_u128))
//!     .collect::<Result<Vec<_>, _>>()?;
//! assert_eq!(values, vec![0, 1, 2, 3, 4, 5]);
//!
//! // Cycle indices are zero-based and monotonically increasing.
//! assert_eq!(samples[0].cycle().index(), 0);
//! assert_eq!(samples[5].cycle().index(), 5);
//! # Ok(()) }
//! ```
//!
//! ## Verilog emission: inspecting the output
//!
//! [`driver::emit_verilog`] returns the full rendered module text.
//! The 64-bit counter produces a module with:
//!
//! - `clk` and `rst` input ports (synchronous reset)
//! - An `output reg [63:0]` port for the count register
//! - A `wire [63:0]` for the constant `1`
//! - A `wire [63:0]` for the `next_state = state + 1` sum
//! - An `always_ff` block that resets the register to zero
//!   on `rst` and advances it to `next_state` otherwise
//!
//! ```
//! # fn main() -> Result<(), msu_256::Error> {
//! use msu_256::hdl::{demo, driver};
//!
//! let counter = demo::demo_counter()?;
//! let verilog = driver::emit_verilog(&counter, "counter64").run()?;
//!
//! // Module declaration names the counter.
//! assert!(verilog.contains("module counter64"));
//!
//! // Clock and reset are explicit ports.
//! assert!(verilog.contains("input clk"));
//! assert!(verilog.contains("input rst"));
//!
//! // The count register is both state and output, so it
//! // appears as `output reg` with 64-bit width.
//! assert!(verilog.contains("output reg [63:0]"));
//!
//! // The increment constant (1) is a 64-bit literal.
//! assert!(verilog.contains("64'd1"));
//!
//! // The adder feeds the next-state value.
//! assert!(verilog.contains("+"));
//!
//! // Synchronous reset to zero, else advance.
//! assert!(verilog.contains("always_ff @(posedge clk)"));
//! assert!(verilog.contains("64'd0"));
//!
//! // Module closes properly.
//! assert!(verilog.contains("endmodule"));
//! # Ok(()) }
//! ```
//!
//! ## Combined: simulate then emit
//!
//! Because [`emit_verilog`] takes the machine by reference,
//! you can inspect the machine's simulation behaviour first
//! and then emit Verilog from the same machine without
//! rebuilding it.
//!
//! ```
//! # fn main() -> Result<(), msu_256::Error> {
//! use msu_256::hdl::{demo, driver};
//! use hdl_cat::prelude::Bits;
//! use hdl_cat::kind::Hw;
//!
//! let counter = demo::demo_counter()?;
//!
//! // Emit Verilog first (borrows the machine).
//! let verilog = driver::emit_verilog(&counter, "msu_ctr").run()?;
//! assert!(verilog.contains("module msu_ctr"));
//!
//! // Then simulate (consumes the machine).
//! let samples = driver::simulate(counter, 3).run()?;
//! let values: Vec<u128> = samples
//!     .iter()
//!     .map(|s| Bits::<64>::from_bits_seq(s.value()).map(Bits::to_u128))
//!     .collect::<Result<Vec<_>, _>>()?;
//! assert_eq!(values, vec![0, 1, 2]);
//! # Ok(()) }
//! ```

use hdl_cat::prelude::*;

use crate::error::Error;

/// The type of the 64-bit demo counter machine.
///
/// This is a [`Sync`] machine with:
///
/// - **State**: [`Obj<Bits<64>>`] (a single 64-bit register)
/// - **Input**: [`CircuitUnit`] (no data input; the counter is free-running)
/// - **Output**: [`Obj<Bits<64>>`] (the current count before incrementing)
///
/// The underlying IR graph contains two instructions: a constant
/// `1` and an adder `state + 1`.  The state-threading and
/// `always_ff` generation are handled by the [`Sync`] / Verilog
/// layers automatically.
pub type DemoCounter = hdl_cat::std_lib::CounterSync<64>;

/// Build a 64-bit free-running counter.
///
/// The returned [`Sync`] machine has `Bits<64>` state starting at
/// zero, no data input ([`CircuitUnit`]), and outputs the current
/// count each cycle.
///
/// # Errors
///
/// Returns [`Error::HdlCat`] if the counter construction fails.
///
/// # Examples
///
/// Inspect the machine's structure before simulation:
///
/// ```
/// # fn main() -> Result<(), msu_256::Error> {
/// use msu_256::hdl::demo;
///
/// let counter = demo::demo_counter()?;
///
/// // One state wire (the 64-bit register).
/// assert_eq!(counter.state_wire_count(), 1);
///
/// // Initial state is 64 zero-bits.
/// assert_eq!(counter.initial_state().len(), 64);
/// assert!(counter.initial_state().as_slice().iter().all(|b| !*b));
///
/// // The IR has exactly 2 instructions: Const(1) and Add.
/// assert_eq!(counter.graph().instructions().len(), 2);
/// # Ok(()) }
/// ```
pub fn demo_counter() -> Result<DemoCounter, Error> {
    std_lib::counter::<64>().map_err(Error::from)
}
