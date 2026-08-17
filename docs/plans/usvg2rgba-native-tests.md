# Plan: complete the §3.2 native tests for `usvg2rgba` (task.md §4)

## Status

`task.md` §4 is fully checked except the "Rust native tests
(`engineering-playbook.md` §3.2)" block (10 unchecked items). Implementing
those tests requires fixing several bugs in the current `usvg2rgba` /
`intermediate` code, because the tests assert behavior that is not yet
implemented. This plan covers both the fixes and the tests.

## Bugs discovered that block the tests

1. **`size` is dropped by the encoder.** `IntermediateV1::encode` writes only
   `shapes` in the payload; `decode` hard-codes `size = (0, 0)`. The
   constitution (§5.1) requires the natural dimensions to be encoded inside the
   intermediate. Blocks the natural-size, sizing, and zero-size tests.
2. **No native core function.** `lib.rs`'s `usvg2rgba` wasm wrapper does
   decode/size/render inline and swallows errors (returns a default empty
   result). The playbook (§3.2) requires a native core function tested at the
   Rust layer, and the error tests require errors to surface.
3. **`to_tree` ignores `Shape.fill`** (always emits `#000000`). Blocks the
   50%-opaque-red alpha tests.
4. **Pixel extraction is wrong.** It reads bytes as B,G,R,A and passes them
   through unchanged. `tiny_skia` 0.12.0 stores **premultiplied RGBA**
   (confirmed in the vendored source: "byteorder: RGBA"). Straight-alpha mode
   must un-premultiply; byte order must be R,G,B,A. Blocks the alpha tests.
5. **Sizing math is wrong for the single-dimension cases** (divides by the
   aspect ratio instead of multiplying), and **no scale transform is applied**
   to the render, so sized output would not scale the drawing. Blocks the
   sizing tests.
6. **`RgbaResult` has no `alpha_mode` field.** The constitution (§4.2) requires
   the result to carry the alpha mode of the returned pixels. This is part of
   §4's "Return dimensions and alpha-mode metadata alongside `pixels`" item.

## Sizing interpretation (needs human confirmation)

Constitution §4.3 and playbook §3.2 both read literally: when only one of
`width`/`height` is given, the other dimension is the **natural SVG value**
(`width × natural_h`; `natural_w × height`). The approved
`docs/plans/baseline-implementation.md` instead said "one set → the other
scaled by the natural aspect ratio". These differ:

- literal: natural 20×10, `width=100` → **100×10**; `height=100` → **20×100**
- aspect-preserving: natural 20×10, `width=100` → **100×50**; `height=100` →
  **50×100**

I plan to follow the literal reading, because it matches the wording the human
put in `task.md` and the playbook. **Confirm or override.**

## Files to change

1. `crates/intermediate/src/lib.rs`
   - `encode`: include `size` in the payload map.
   - `decode`: read and validate `size` from the payload.
   - `to_tree`: honor `Shape.fill` color (emit the stored color, not `#000000`).
2. `crates/usvg2rgba/src/core.rs`
   - Add native `RgbaOptions`, `RgbaResult` (with `alpha_mode`), and
     `rasterize(bytes: &[u8], options: &RgbaOptions) -> Result<RgbaResult, String>`.
   - Fix single-dimension sizing; apply `Transform::from_scale` for the sized
     cases; return an error on zero natural size.
   - Correct R,G,B,A byte order; un-premultiply for straight alpha; passthrough
     for premultiplied.
3. `crates/usvg2rgba/src/lib.rs`
   - Make `core` a `pub mod` so integration tests can reach `core::rasterize`.
   - The `#[wasm_bindgen] usvg2rgba` delegates to `core::rasterize` and returns
     `Result<RgbaResult, JsValue>` (so malformed payloads reject the promise,
     per constitution §4.2). Add `alpha_mode` to the wasm `RgbaResult`.
4. `crates/usvg2rgba/Cargo.toml`
   - `crate-type = ["cdylib", "rlib"]` (integration tests need an `rlib`).
   - dev-dependency `cbor-core` (workspace). Justified: it is the mandated
     codec (constitution §3.7) and already a workspace dependency; needed only
     to construct malformed-envelope fixtures (unknown identifier, unsupported
     version, malformed DTO, unsupported variant).
5. `crates/usvg2rgba/tests/core.rs` (new) — the 10 tests.
6. `task.md` — check off the §4 native-test items as they pass.

## Tests (engineering-playbook §3.2)

Fixtures are built with the existing envelope helper pattern: construct an
`IntermediateV1` and call `.encode()` (per the human's earlier instruction to
reuse the envelope-construction helpers).

1. Natural size: no sizing options → pixmap equals natural size,
   `pixels.len() == width * height * 4`.
2. Width-only: `width=100`, natural 20×10 → **100×10**.
3. Height-only: `height=100`, natural 20×10 → **20×100**.
4. Both set, non-uniform aspect: `width=100, height=30`, natural 20×10 →
   exactly 100×30.
5. Non-canonical / non-CBOR / non-package payload → error, not panic.
6. Unknown identifier / unsupported version / malformed DTO / unsupported DTO
   variant → error, not panic.
7. Zero-sized SVG (`size = (0,0)`) → error.
8. Straight alpha (default): 50%-opaque red → `(255, 0, 0, 128)`.
9. Premultiplied alpha: same input → `(128, 0, 0, 128)`.
10. Renderer determinism: two consecutive calls → identical bytes.

## Verify

- `cargo test -p usvg2rgba` — all 10 new tests pass.
- `cargo test -p intermediate -p svg2usvg` — still green (intermediate changed).
- `cargo fmt` on changed `.rs` files.
- `cargo check --workspace`.

## Deviations from the approved plan

- Sizing single-dimension case: `baseline-implementation.md` said
  aspect-preserving; I follow the governing docs' literal reading instead
  (pending confirmation above).
- The wasm `usvg2rgba` return type changes from `RgbaResult` to
  `Result<RgbaResult, JsValue>`. Not stated in `baseline-implementation.md`
  (which kept a sync wrapper reading a `JsValue` options object); required for
  the constitution §4.2 error contract and the §3.4 Deno test that a malformed
  payload rejects the promise.