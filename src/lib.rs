//! # msu-256
//!
//! A 256-bit Ozturk Modular Squaring Unit built on [`rhdl`] (for hardware
//! description) and [`comp_cat_rs`] (for effectful simulation orchestration).
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
//! - [`hdl`] — RHDL circuit definitions and simulation driver
//! - [`simulate`] — comp-cat-rs-driven `Stream`-based MSU simulation,
//!   golden model, and testbench
//!
//! ## Example
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

pub mod bigint;
pub mod domain;
pub mod error;
pub mod hdl;
pub mod params;
pub mod simulate;
pub mod table;

pub use error::Error;
