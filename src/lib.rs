//! # msu-256
//!
//! A 256-bit Ozturk Modular Squaring Unit built on [`hdl_cat`] (for hardware
//! description and Verilog emission) and [`comp_cat_rs`] (for effectful
//! simulation orchestration).
//!
//! The MSU is the core accelerator for VDF (Verifiable Delay Function)
//! evaluation: given a value `a` in Montgomery form, it computes repeated
//! `a^2 mod m` with minimum latency via a self-feeding pipeline.
//!
//! ## Architecture
//!
//! Values are stored as a **redundant polynomial**: 17 coefficients, each
//! split into a 16-bit non-redundant part and a 1-bit redundant bit.  This
//! avoids full-width carry propagation on every cycle.
//!
//! The MSU has two combinational stages composed into a single clock cycle:
//!
//! 1. **Squarer**: polynomial product of input with itself, split into three
//!    triangle parts (lower / middle / upper) based on bit position.
//! 2. **Reducer**: Montgomery reduction (lower), pass-through (middle),
//!    pre-calculated reduction (upper), summed into a new redundant polynomial.
//!
//! ## Modules
//!
//! - [`bigint`] — fixed-width `U256` / `U512` arithmetic for table generation
//! - [`params`] — compile-time MSU-256 parameters
//! - [`domain`] — signal types (`RedundantPoly`, `TriangleParts`, `MsuConfig`)
//! - [`table`] — Montgomery and upper reduction table generation
//! - [`hdl`] — hdl-cat circuit definitions, simulation driver, and Verilog
//!   emission
//! - [`simulate`] — comp-cat-rs-driven `Stream`-based MSU simulation,
//!   golden model, and testbench
//!
//! ## Example: MSU golden-model testbench
//!
//! ```
//! use msu_256::{simulate::run_testbench, error::Error};
//!
//! fn main() -> Result<(), Error> {
//!     let result = run_testbench(10).run()?;
//!     result.assert_passed()
//! }
//! # fn dummy() -> Result<(), Error> { main() }
//! ```
//!
//! ## Example: hdl-cat simulation with typed decoding
//!
//! Build a 64-bit counter, simulate for several cycles, and decode
//! each cycle's raw [`BitSeq`] output into a [`Bits<64>`] value:
//!
//! ```
//! # fn main() -> Result<(), msu_256::Error> {
//! use msu_256::hdl::{demo, driver};
//! use hdl_cat::prelude::Bits;
//! use hdl_cat::kind::Hw;
//!
//! let counter = demo::demo_counter()?;
//! let samples = driver::simulate(counter, 5).run()?;
//!
//! // Decode BitSeq -> Bits<64> -> u128 for each cycle.
//! let values: Vec<u128> = samples
//!     .iter()
//!     .map(|s| Bits::<64>::from_bits_seq(s.value()).map(Bits::to_u128))
//!     .collect::<Result<Vec<_>, _>>()?;
//! assert_eq!(values, vec![0, 1, 2, 3, 4]);
//! # Ok(()) }
//! ```
//!
//! ## Example: Verilog emission and inspection
//!
//! Lower the same counter to Verilog and verify its structure:
//!
//! ```
//! # fn main() -> Result<(), msu_256::Error> {
//! use msu_256::hdl::{demo, driver};
//!
//! let counter = demo::demo_counter()?;
//! let verilog = driver::emit_verilog(&counter, "msu_counter64").run()?;
//!
//! // Module header with clock and reset.
//! assert!(verilog.contains("module msu_counter64"));
//! assert!(verilog.contains("input clk"));
//! assert!(verilog.contains("input rst"));
//!
//! // The count register: 64-bit output reg.
//! assert!(verilog.contains("output reg [63:0]"));
//!
//! // Increment logic: constant 1, adder, always_ff with
//! // synchronous reset to zero.
//! assert!(verilog.contains("64'd1"));
//! assert!(verilog.contains("+"));
//! assert!(verilog.contains("always_ff @(posedge clk)"));
//! assert!(verilog.contains("64'd0"));
//! assert!(verilog.contains("endmodule"));
//! # Ok(()) }
//! ```

pub mod bigint;
pub mod domain;
pub mod error;
pub mod hdl;
pub mod params;
pub mod simulate;
pub mod table;

pub use error::Error;
