//! comp-cat-rs-driven MSU simulation and golden model.

pub mod golden;
pub mod msu_step;
pub mod testbench;

pub use golden::golden_model;
pub use msu_step::msu_step;
pub use testbench::{run_testbench, TestResult};
