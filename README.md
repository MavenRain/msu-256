# msu-256

A 256-bit Ozturk Modular Squaring Unit built on
[hdl-cat](https://github.com/MavenRain/hdl-cat) and
[comp-cat-rs](https://github.com/MavenRain/comp-cat-rs).

The MSU is the core accelerator for **VDF (Verifiable Delay Function)** evaluation:
given a value `a` in Montgomery form, it computes repeated `a² mod m` with
minimum latency via a self-feeding pipeline.

Based on the SystemVerilog reference at
[supranational/hardware](https://github.com/supranational/hardware/tree/master/rtl/msu).

## Layout

| Module | Purpose |
|---|---|
| `bigint` | `U256` / `U512` fixed-width unsigned integer arithmetic |
| `params` | Compile-time MSU-256 parameters (word widths, triangle sizes) |
| `domain` | `RedundantPoly`, `TriangleParts`, `MsuConfig` signal types |
| `table` | Montgomery `Mu`, `MontRedTable`, `UpperRedTable` generation |
| `hdl` | hdl-cat `Sync` machine demo, simulation driver, Verilog emission |
| `simulate` | `Stream`-based MSU loop, golden model, testbench |

## MSU-256 Parameters

| Parameter | Value | Formula |
|---|---|---|
| `WORD_BITS` | 16 | |
| `WORD_ELEMENTS` | 16 | |
| `TARGET_BITS` | 256 | `WORD_ELEMENTS * WORD_BITS` |
| `FULL_WORD_BITS` | 17 | `WORD_BITS + 1` |
| `NUM_ELEMENTS` | 17 | `WORD_ELEMENTS + 1` |
| `OUTER_TRI_TREES` | 9 | `NUM_ELEMENTS - WORD_ELEMENTS/2` |
| `LOWER_TRI_BITS` | 144 | `OUTER_TRI_TREES * WORD_BITS` |
| `UPPER_TRI_BITS` | 145 | `OUTER_TRI_TREES * WORD_BITS + 1` |
| `R` | `2^144` | Montgomery radix |
| modulus | `0x4903d72a9ea2fb2795496eb04ee87dde57113bd8a8192f26db4e763141802c27` | |

## Algorithm

One MSU step computes `a_mont → a_mont² · R⁻¹ mod m`:

1. **Square**: compute `x = a_mont²` (produces up to 512 bits).
2. **Split** `x` into three regions:
   - `lower = x & (2^144 − 1)`
   - `mid = (x >> 144) & (2^256 − 1)`
   - `upper = x >> 400`
3. **Reduce**:
   - `lower_sum = Σᵢ MontRedTable[i]` for each set bit `i` in `lower`
   - `upper_sum = Σᵢ UpperRedTable[i]` for each set bit `i` in `upper`
   - `result = (lower_sum + mid + upper_sum) mod m`

where
- `MontRedTable[i] = ⌊((2ⁱ · μ) mod 2¹⁴⁴) · m / 2¹⁴⁴⌋ + 1`
- `UpperRedTable[i] = 2^(i+256) mod m`
- `μ = 2¹⁴⁴ − (m⁻¹ mod 2¹⁴⁴)`

## Verification

The testbench runs the algorithmic MSU step against a big-integer golden model
that computes `a² · R⁻¹ mod m` directly, and compares every iteration.

```bash
cargo test
RUSTFLAGS="-D warnings" cargo clippy --all-targets
cargo test --doc
```

## Usage

### Golden-model testbench

```rust
use msu_256::{simulate::run_testbench, error::Error};

fn run() -> Result<(), Error> {
    let result = run_testbench(100).run()?;
    result.assert_passed()
}
```

### Simulation with typed decoding

Build a 64-bit counter, simulate for several cycles, and decode
each cycle's raw `BitSeq` output into a typed `Bits<64>` value:

```rust
use msu_256::hdl::{demo, driver};
use hdl_cat::prelude::Bits;
use hdl_cat::kind::Hw;

fn run() -> Result<(), msu_256::Error> {
    let counter = demo::demo_counter()?;
    let samples = driver::simulate(counter, 5).run()?;

    // Each sample carries a cycle index and a BitSeq of output bits.
    // Decode BitSeq -> Bits<64> -> u128 for each cycle.
    let values: Vec<u128> = samples
        .iter()
        .map(|s| Bits::<64>::from_bits_seq(s.value()).map(Bits::to_u128))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(values, vec![0, 1, 2, 3, 4]);
    Ok(())
}
```

### Verilog emission and inspection

Lower a counter to Verilog and verify the emitted module's structure:

```rust
use msu_256::hdl::{demo, driver};

fn run() -> Result<(), msu_256::Error> {
    let counter = demo::demo_counter()?;
    let verilog = driver::emit_verilog(&counter, "msu_counter64").run()?;

    // Module header with clock and reset.
    assert!(verilog.contains("module msu_counter64"));
    assert!(verilog.contains("input clk"));
    assert!(verilog.contains("input rst"));

    // The count register: 64-bit output reg.
    assert!(verilog.contains("output reg [63:0]"));

    // Increment logic: constant 1, adder, always_ff with
    // synchronous reset to zero.
    assert!(verilog.contains("64'd1"));
    assert!(verilog.contains("+"));
    assert!(verilog.contains("always_ff @(posedge clk)"));
    assert!(verilog.contains("64'd0"));
    assert!(verilog.contains("endmodule"));
    Ok(())
}
```

### Emit then simulate the same machine

Because `emit_verilog` borrows the machine, you can inspect the
Verilog first and then simulate without rebuilding:

```rust
use msu_256::hdl::{demo, driver};
use hdl_cat::prelude::Bits;
use hdl_cat::kind::Hw;

fn run() -> Result<(), msu_256::Error> {
    let counter = demo::demo_counter()?;

    // Borrow for Verilog emission.
    let verilog = driver::emit_verilog(&counter, "dual_use").run()?;
    assert!(verilog.contains("module dual_use"));

    // Consume for simulation.
    let samples = driver::simulate(counter, 3).run()?;
    let values: Vec<u128> = samples
        .iter()
        .map(|s| Bits::<64>::from_bits_seq(s.value()).map(Bits::to_u128))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(values, vec![0, 1, 2]);
    Ok(())
}
```

## hdl-cat Integration

`hdl::demo` builds a 64-bit free-running counter as an hdl-cat
`Sync<Obj<Bits<64>>, CircuitUnit, Obj<Bits<64>>>` machine.  The
underlying IR graph contains two instructions (a constant `1` and
an adder); state threading, synchronous reset, and Verilog code
generation are handled by the `Sync` / `verilog` layers
automatically.

`hdl::driver` provides two comp-cat-rs `Io` wrappers:

- `simulate` drives any `Sync` machine through hdl-cat's
  `Testbench`, returning `Vec<TimedSample<BitSeq>>`.
- `emit_verilog` composes `verilog::emit_sync_graph` and
  `Module::render` via `Io::flat_map`, returning the rendered
  module text.

Unlike the previous RHDL backend, hdl-cat has no `Bits<N>` width
ceiling, so full 256-bit MSU polynomial operations are expressible
directly as `#[kernel]` functions.

## License

Dual-licensed under MIT OR Apache-2.0.
