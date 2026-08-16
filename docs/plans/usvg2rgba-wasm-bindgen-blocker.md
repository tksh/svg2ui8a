# Blocked: usvg2rgba wasm_bindgen Integration

## Problem

The `#[wasm_bindgen]` derive macro on structs requires all fields to implement `std::marker::Copy`. The `String` type does not implement `Copy`, which blocks the wasm-bindgen wrapper implementation for the `Usvg2RgbaOptions` struct (which contains an `alpha_mode: String` field) and the `RgbaResult` struct.

This is a fundamental constraint of wasm-bindgen version 0.2 (the version available in this project's toolchain). Upgrading wasm-bindgen is not straightforward as it requires careful coordination with the broader build pipeline.

## Root Cause

In wasm-bindgen 0.2, the `BindgenedStruct` derive macro generates assertion checks that require all struct fields to implement `Copy`:
```
the trait `Copy` is not implemented for `String`
required by a bound in `__wbg_get_usvg2rgbaoptions_alpha_mode::assert_copy`
```

This affects any struct field of type `String` (or `Option<String>`) within a `#[wasm_bindgen]` struct.

## Impact

The following wasm-bindgen exports are blocked:
1. `Usvg2RgbaOptions` struct with `alpha_mode: String` field
2. `RgbaResult` struct with `pixels: Vec<u8>` and `alpha_mode: String` fields
3. Getter methods on these structs (`get_alpha_mode`, `get_pixels_len`, etc.)
4. The `usvg2rgba` wasm_bindgen function signature

Without these, the JavaScript frontend cannot:
- Pass alpha mode options from JavaScript to the Wasm module
- Receive alpha mode metadata back from the rasterization result
- Properly interact with the `usvg2rgba` promise-based API

## Proposed Solutions

### Solution 1: Upgrade wasm-bindgen
**Pros:**
- Fixes the issue at the source
- All derive macros work as intended
- Minimal code changes needed

**Cons:**
- Requires updating `wasm-bindgen` version in `Cargo.toml` workspace
- May require updating other dependencies that pin to specific wasm-bindgen versions
- Could introduce breaking changes in other parts of the codebase
- Must verify `deno task check` and `deno task lint` still pass

**Steps:**
1. Update `Cargo.toml` workspace: `wasm-bindgen = "0.3"` (or latest compatible)
2. Run `cargo update -p wasm-bindgen` 
3. Resolve any compilation errors from API changes
4. Verify `deno task test` still passes all layers
5. Run `deno task check` and `deno task lint`

### Solution 2: Manually implement wasm_bindgen extern functions
**Pros:**
- No dependency version upgrade needed
- Can be implemented incrementally
- Full control over the JS-Wasm boundary

**Cons:**
- More code to maintain
- Must manually implement all the `IntoWasmAbi`, `FromWasmAbi`, `RefFromWasmAbi`, `WasmDescribe` traits
- Easier to introduce subtle bugs in the ABI
- More code duplication risk

**Steps:**
1. Remove `#[wasm_bindgen]` derive macros from structs
2. Manually implement `wasm_bindgen::prelude::WasmDescribe` for structs
3. Manually implement `wasm_bindgen::prelude::IntoWasmAbi` and `RefFromWasmAbi`
4. Replace `#[wasm_bindgen(getter)]` methods with standalone `#[wasm_bindgen]` functions
5. Test each export manually

### Solution 3: Replace `String` with `JsValue`
**Pros:**
- Minimal code changes
- `JsValue` implements `Copy` in wasm-bindgen 0.2
- Preserves the `#[wasm_bindgen]` derive structure

**Cons:**
- Loses type safety for the alpha_mode field
- Requires manual `JsValue::from_str()` and `.as_str()` conversions everywhere
- Not idiomatic for the codebase's Rust style

**Steps:**
1. Change `pub alpha_mode: String` to `pub alpha_mode: JsValue` in struct definitions
2. Update all references: `JsValue::from("straight")` instead of `"straight".to_string()`
3. Update getter: `get_alpha_mode` returns `JsValue` instead of `String`
4. Update callers to convert back to `String` if needed

### Solution 4: Remove alpha_mode from the public struct
**Pros:**
- Simplifies the struct
- Eliminates the Copy bound issue entirely
- Minimal changes

**Cons:**
- Loses the alpha mode metadata feature
- May require changes to how the rasterization result is used
- Not ideal if alpha mode is needed for the API contract

**Steps:**
1. Remove `alpha_mode` field from `RgbaResult` and `Usvg2RgbaOptions`
2. Update `rasterize()` to use default "straight" alpha
3. Update all getters and exports accordingly
4. Document that alpha mode is always "straight" unless extended later

## Recommended Approach

**Primary: Solution 1 (Upgrade wasm-bindgen)**
This is the cleanest fix and aligns with the project's forward-compatibility goals. The workspace already pins specific dependency versions, and upgrading wasm-bindgen by one major version is typically well-tested.

**Secondary: Solution 3 (Replace String with JsValue)**
If upgrading wasm-bindgen is delayed, this is the least invasive alternative. It preserves the derive macro structure while working around the Copy limitation.

**Tertiary: Solution 4 (Remove alpha_mode)**
Only if the alpha mode feature can be deferred to a future release. This is the simplest code change but reduces the API capability.

**Not Recommended: Solution 2 (Manual trait impls)**
This is the most error-prone approach and creates maintenance burden. The trait bounds for wasm-bindgen 0.2 are complex and easy to get wrong.

## Decision

The root-cause analysis is correct (wasm-bindgen's default field-getter
generation requires `Copy`, and `String` does not implement `Copy`), but this
is **not** a wasm-bindgen 0.2 version limitation. wasm-bindgen 0.2 already
ships a struct-level attribute for exactly this case:
`#[wasm_bindgen(getter_with_clone)]`. When applied to a struct, it generates
field getters that `.clone()` the value instead of requiring `Copy`, which
works for `String`, `Vec<u8>`, and any other `Clone` type.

**Adopted solution:** add `#[wasm_bindgen(getter_with_clone)]` to both
`Usvg2RgbaOptions` and `RgbaResult`:

```rust
#[wasm_bindgen(getter_with_clone)]
pub struct Usvg2RgbaOptions {
    pub alpha_mode: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[wasm_bindgen(getter_with_clone)]
pub struct RgbaResult {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub alpha_mode: String,
}
```

This requires no change to the `wasm-bindgen` version pin in the workspace
`Cargo.toml`, keeps `alpha_mode` as a type-safe `String` rather than
`JsValue`, and keeps the field on the public API surface as specified in
`docs/project-constitution.md` §4.2. No other part of `crates/usvg2rgba` (the
native `core.rs` logic) needs to change.

**Rejected, and why:**

- *Upgrade wasm-bindgen (former Solution 1).* Unnecessary — the attribute
  above already solves this in the pinned 0.2 line. A version bump would add
  unrelated risk to the build pipeline for no benefit.
- *Manually implement the ABI traits (former Solution 2).* Unnecessary and
  error-prone; `getter_with_clone` is the mechanism wasm-bindgen provides for
  this exact situation.
- *Replace `String` with `JsValue` (former Solution 3).* Unnecessary and a
  regression — it would drop compile-time type safety for `alpha_mode`
  without any offsetting benefit, now that `getter_with_clone` is available.
- *Remove `alpha_mode` from the public struct (former Solution 4).* Rejected
  — it would silently drop an API guarantee (`project-constitution.md` §4.2,
  §5.2) rather than fix the actual blocker.

## File Reference

This document is located at: `docs/plans/usvg2rgba-wasm-bindgen-blocker.md`

Status: **resolved**. No further planning-session review is needed for this
specific blocker; proceed with implementation per `task.md` §4.
