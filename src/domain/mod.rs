//! MSU signal types: coefficients, redundant polynomial, triangle parts, config.

pub mod coeff;
pub mod config;
pub mod poly;
pub mod triangle;

pub use coeff::{FullWordCoeff, RedundantBit, WordCoeff};
pub use config::MsuConfig;
pub use poly::RedundantPoly;
pub use triangle::TriangleParts;
