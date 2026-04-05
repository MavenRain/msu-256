//! Triangle parts: the squarer's three-way output split.

use crate::error::Error;
use crate::params::{LOWER_TRI_BITS, TARGET_BITS, UPPER_TRI_BITS};

/// The three regions of the squarer output, as bit slices.
///
/// Each region is stored as a bit vector (big enough to hold its region).
/// The squarer populates these; the reducer consumes them.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct TriangleParts {
    lower: Vec<bool>,
    mid: Vec<bool>,
    upper: Vec<bool>,
}

impl TriangleParts {
    /// Construct from bit vectors.
    ///
    /// # Errors
    ///
    /// Returns [`Error::IndexOutOfBounds`] if any vector has the wrong length.
    pub fn new(lower: Vec<bool>, mid: Vec<bool>, upper: Vec<bool>) -> Result<Self, Error> {
        match (lower.len(), mid.len(), upper.len()) {
            (l, m, u) if l == LOWER_TRI_BITS && m == TARGET_BITS && u == UPPER_TRI_BITS => {
                Ok(Self { lower, mid, upper })
            }
            (l, _, _) if l != LOWER_TRI_BITS => Err(Error::IndexOutOfBounds {
                index: l,
                length: LOWER_TRI_BITS,
            }),
            (_, m, _) if m != TARGET_BITS => Err(Error::IndexOutOfBounds {
                index: m,
                length: TARGET_BITS,
            }),
            (_, _, u) => Err(Error::IndexOutOfBounds {
                index: u,
                length: UPPER_TRI_BITS,
            }),
        }
    }

    /// Access the lower triangle bit vector.
    #[must_use]
    pub fn lower(&self) -> &[bool] {
        &self.lower
    }

    /// Access the middle triangle bit vector.
    #[must_use]
    pub fn mid(&self) -> &[bool] {
        &self.mid
    }

    /// Access the upper triangle bit vector.
    #[must_use]
    pub fn upper(&self) -> &[bool] {
        &self.upper
    }
}
