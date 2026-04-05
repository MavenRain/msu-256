//! A minimal RHDL `Synchronous` circuit demonstrating DFF state.
//!
//! This circuit stores a 64-bit counter in a DFF and increments it
//! each cycle when enabled.  It establishes the integration pattern
//! that a full MSU circuit would extend: derive `Synchronous`,
//! implement `SynchronousIO`, and wire a `#[kernel]` function that
//! produces `(output, next_state)` from `(clock_reset, input, q_state)`.

use rhdl::prelude::*;
use rhdl_fpga::core::dff::DFF;

/// A 64-bit counter with enable input.  The output is the current count.
#[derive(Clone, Debug, Synchronous, SynchronousDQ)]
#[rhdl(dq_no_prefix)]
pub struct DemoCircuit {
    count: DFF<Bits<64>>,
}

impl Default for DemoCircuit {
    fn default() -> Self {
        Self {
            count: DFF::new(bits::<64>(0)),
        }
    }
}

impl SynchronousIO for DemoCircuit {
    type I = bool;
    type O = Bits<64>;
    type Kernel = demo_kernel;
}

/// The combinational kernel: reads the DFF, computes the next count.
///
/// When `enable` is true and not resetting, increments the stored value.
/// Outputs the current stored value.
#[kernel]
pub fn demo_kernel(cr: ClockReset, enable: bool, q: Q) -> (Bits<64>, D) {
    let next_count = if enable {
        q.count + bits::<64>(1)
    } else {
        q.count
    };
    let next_count = if cr.reset.any() {
        bits::<64>(0)
    } else {
        next_count
    };
    (q.count, D { count: next_count })
}
