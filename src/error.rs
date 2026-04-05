//! The single crate-wide `Error` enum.
//!
//! All fallible operations in `msu-256` return `Result<T, Error>`.
//! No `unwrap`, no `panic`, no `thiserror`, no `anyhow`.

use comp_cat_rs::collapse::free_category::FreeCategoryError;
use rhdl::prelude::RHDLError;

/// All failure modes in the MSU crate.
#[derive(Debug)]
pub enum Error {
    /// Division or modular reduction by zero.
    DivisionByZero,
    /// A modular inverse does not exist (inputs not coprime).
    ModularInverseDoesNotExist,
    /// Hex string failed to parse.
    HexParse(&'static str),
    /// Hex string has wrong length for target type.
    HexLength {
        /// Expected number of hex digits.
        expected: usize,
        /// Actual number of hex digits.
        actual: usize,
    },
    /// A coefficient value exceeded its valid range.
    CoefficientOutOfRange {
        /// The offending value.
        value: u64,
        /// The maximum allowed value (exclusive upper bound).
        max: u64,
    },
    /// An index into an array was out of bounds.
    IndexOutOfBounds {
        /// The offending index.
        index: usize,
        /// The length of the array.
        length: usize,
    },
    /// A signal value had the wrong type for the operation.
    SignalTypeMismatch {
        /// What was expected.
        expected: &'static str,
        /// What was actually provided.
        got: &'static str,
    },
    /// An error from the comp-cat-rs free category.
    CircuitComposition(FreeCategoryError),
    /// An error from the RHDL hardware simulation layer.
    Rhdl(Box<RHDLError>),
    /// The MSU simulation produced results differing from the golden model.
    SimulationMismatch {
        /// The iteration index where the mismatch occurred.
        iteration: usize,
    },
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::ModularInverseDoesNotExist => {
                write!(f, "modular inverse does not exist (inputs not coprime)")
            }
            Self::HexParse(reason) => write!(f, "hex parse failed: {reason}"),
            Self::HexLength { expected, actual } => write!(
                f,
                "hex string length {actual} does not match expected {expected}"
            ),
            Self::CoefficientOutOfRange { value, max } => {
                write!(f, "coefficient value {value} exceeds maximum {max}")
            }
            Self::IndexOutOfBounds { index, length } => {
                write!(f, "index {index} out of bounds for length {length}")
            }
            Self::SignalTypeMismatch { expected, got } => {
                write!(f, "signal type mismatch: expected {expected}, got {got}")
            }
            Self::CircuitComposition(e) => write!(f, "circuit composition error: {e}"),
            Self::Rhdl(e) => write!(f, "rhdl error: {e}"),
            Self::SimulationMismatch { iteration } => {
                write!(f, "simulation mismatch at iteration {iteration}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CircuitComposition(e) => Some(e),
            Self::Rhdl(e) => Some(e),
            Self::DivisionByZero
            | Self::ModularInverseDoesNotExist
            | Self::HexParse(_)
            | Self::HexLength { .. }
            | Self::CoefficientOutOfRange { .. }
            | Self::IndexOutOfBounds { .. }
            | Self::SignalTypeMismatch { .. }
            | Self::SimulationMismatch { .. } => None,
        }
    }
}

impl From<FreeCategoryError> for Error {
    fn from(e: FreeCategoryError) -> Self {
        Self::CircuitComposition(e)
    }
}

impl From<RHDLError> for Error {
    fn from(e: RHDLError) -> Self {
        Self::Rhdl(Box::new(e))
    }
}
