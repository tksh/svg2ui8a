# Fix Natural-Size Aspect-Ratio Calculation

## Goal

Preserve the fractional natural SVG size while calculating a requested raster
dimension, so `svg2rgba` does not lose aspect-ratio precision by truncating
`usvg::Tree::size()` to `u32` too early.

## Scope

- Modify `crates/svg2rgba/src/core.rs`.
- Extend `crates/svg2rgba/tests/core.rs` with fractional natural-size cases.
- Keep the public output dimensions as integer pixel sizes (`u32`).
- Keep both-dimensions-specified behavior as exact, independently scaled output.
- Do not add dependencies or change the package's Wasm/API boundary.

## Implementation

1. Keep `tree.size().width()` and `tree.size().height()` as `f32` values after
   parsing.
2. Validate the natural dimensions before converting them to output pixels.
3. When both output dimensions are omitted, round each natural dimension to the
   nearest positive `u32` pixel size.
4. When only height is specified, calculate width from the fractional natural
   aspect ratio and round the result to the nearest pixel.
5. When only width is specified, calculate height from the fractional natural
   aspect ratio and round the result to the nearest pixel.
6. Preserve exact requested dimensions when both options are specified.
7. Reject a rounded natural dimension or derived dimension that becomes zero.

## Tests

- Fractional natural size with height-only output preserves the fractional
  aspect ratio before rounding.
- Fractional natural size with width-only output preserves the fractional aspect
  ratio before rounding.
- Fractional natural size with both options omitted produces rounded natural
  pixel dimensions.
- Existing integer sizing, alpha, malformed-input, feature, and determinism
  tests remain unchanged and pass.

## Validation

- `cargo fmt --check`
- `cargo test --workspace`
- `deno task test:rust`
- `deno task check`
- `deno task lint`
- `deno fmt --check`

No commit will be created. Suggested commit message after implementation:

```text
fix: preserve fractional natural size for aspect-ratio sizing
```
