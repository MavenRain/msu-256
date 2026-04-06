//! hdl-cat circuit definitions for the MSU.
//!
//! Unlike the previous RHDL backend, hdl-cat lifts the 128-bit
//! `Bits<N>` ceiling, so full 256-bit MSU operations become
//! expressible as `#[kernel]` functions.  This module provides:
//!
//! - A [`demo`] module with a 64-bit free-running counter that
//!   demonstrates the hdl-cat `Sync` machine pattern, including
//!   cycle-accurate simulation and Verilog emission.
//! - A [`driver`] layer that wraps hdl-cat's `Testbench` and
//!   `verilog::emit_sync_graph` in comp-cat-rs `Io`, so hardware
//!   simulation and code generation compose with the rest of the
//!   crate's effectful pipelines.

pub mod demo;
pub mod driver;
