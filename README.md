# msu-256

A 256-bit Ozturk Modular Squaring Unit built on [RHDL](https://github.com/samitbasu/rhdl) and
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
| `hdl` | RHDL `Synchronous` circuit scaffolding + comp-cat-rs driver |
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

```rust
use msu_256::{simulate::run_testbench, error::Error};

fn run() -> Result<(), Error> {
    let result = run_testbench(100).run()?;
    result.assert_passed()
}
```

## RHDL Integration

`hdl::demo` contains a minimal RHDL `Synchronous` circuit (a 64-bit enable
counter) demonstrating the DFF state pattern.  `hdl::driver` wraps RHDL's
`.run()` simulation in comp-cat-rs `Io` for composition with the rest of
the crate.

A full MSU squarer/reducer as RHDL kernels remains future work: it requires
expressing the 17-coefficient polynomial operations within RHDL's kernel
subset (128-bit maximum `Bits<N>` width, bounded loops only, no dyn / no
closures).

## License

Dual-licensed under MIT OR Apache-2.0.
