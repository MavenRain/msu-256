//! Compile-time parameters for the 256-bit MSU.
//!
//! All constants derive from the two configured values, `WORD_BITS` and
//! `WORD_ELEMENTS`, matching the `SystemVerilog` `msu_pkg.sv` derivation.

/// Width of each non-redundant coefficient in bits.
pub const WORD_BITS: usize = 16;

/// Number of full-width coefficient positions in the target value.
pub const WORD_ELEMENTS: usize = 16;

/// Width of the target modular value (`WORD_ELEMENTS * WORD_BITS`).
pub const TARGET_BITS: usize = WORD_ELEMENTS * WORD_BITS;

/// Width of a full coefficient (non-redundant + redundant bit).
pub const FULL_WORD_BITS: usize = WORD_BITS + 1;

/// Number of coefficients in the redundant polynomial.
pub const NUM_ELEMENTS: usize = WORD_ELEMENTS + 1;

/// Total redundant-representation bit width (`NUM_ELEMENTS * FULL_WORD_BITS`).
pub const TOTAL_BITS: usize = NUM_ELEMENTS * FULL_WORD_BITS;

/// Number of outer-triangle adder trees.
pub const OUTER_TRI_TREES: usize = NUM_ELEMENTS - (WORD_ELEMENTS / 2);

/// Bit width of the lower triangle (Montgomery reduction zone).
pub const LOWER_TRI_BITS: usize = OUTER_TRI_TREES * WORD_BITS;

/// Bit width of the upper triangle (pre-calculated reduction zone).
pub const UPPER_TRI_BITS: usize = OUTER_TRI_TREES * WORD_BITS + 1;

/// Mask covering a single non-redundant coefficient.
pub const WORD_BITS_MASK: u32 = (1_u32 << WORD_BITS) - 1;

/// Mask covering a full coefficient.
pub const FULL_WORD_BITS_MASK: u32 = (1_u32 << FULL_WORD_BITS) - 1;
