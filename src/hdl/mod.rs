//! RHDL circuit definitions for the MSU.
//!
//! Due to RHDL's 128-bit `Bits<N>` limit, full 256-bit MSU operations
//! require composite types.  This module provides:
//!
//! - A minimal [`demo::DemoCircuit`] demonstrating the RHDL `Synchronous`
//!   pattern with a DFF-held state.  This is the scaffolding a full MSU
//!   kernel would follow.
//! - A [`driver`] layer that wraps RHDL's `.run()` in comp-cat-rs `Io`,
//!   so hardware simulation composes with the rest of the crate's
//!   effectful machinery.
//!
//! The full MSU squarer and reducer as RHDL kernels remain future work:
//! they require composing the 17-coefficient polynomial operations at
//! the bit level within RHDL's kernel subset.

pub mod demo;
pub mod driver;
