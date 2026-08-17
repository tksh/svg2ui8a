# `@tksh/svg2ui8a` — Implementation Task List

This checklist turns `AGENTS.md`, `docs/project-constitution.md`,
`docs/system-architecture.md`, and `docs/engineering-playbook.md`
into a concrete, ordered build plan. It assumes the repository currently
contains only documentation (no source yet).

Process reminder (`engineering-playbook.md` §4): draft a plan in
`docs/plans/baseline-implementation.md` referencing the constitution section
that authorizes it, get human approval, then implement. This file is that
top-level plan for the first, baseline implementation.

---

## 0. Preconditions

- [x] Re-read `AGENTS.md`, `docs/project-constitution.md`,
      `docs/system-architecture.md`, and `docs/engineering-playbook.md` in that
      order before writing any code.
- [x] Confirm no part of this plan proposes: a third serialization format, a
      single merged Wasm artifact, PNG/WebP output, font/text support,
      raster-image support, a Node.js target, or a runtime CDN import
      (`project-constitution.md` §3, `AGENTS.md` §5).
- [x] Draft `docs/plans/baseline-implementation.md` from this checklist and get
      human sign-off before writing source files (`engineering-playbook.md` §4,
      step 3).

---

## 1. Repository scaffolding

- [x] Create the workspace root `Cargo.toml` with the three members
      (`crates/intermediate`, `crates/svg2usvg`, `crates/usvg2rgba`) and
      `resolver = "2"`.
- [x] Pin `[workspace.dependencies]`: `cbor-core = "0.10.1"`,
      `usvg = { version = "0.47.0", default-features = false }`,
      `resvg = { version = "0.47.0", default-features = false }`
      (`system-architecture.md` §3.4).
- [x] Create `jsr.json` with the three exports: `.`, `./usvg`, `./rgba`
      (`system-architecture.md` §2).
- [x] Create `deno.json` with the `build`, `test`, `test:rust`, `test:wasm`,
      `fmt`, `lint`, `check` tasks (`system-architecture.md` §2,
      `engineering-playbook.md` §1).
- [x] Create `.gitignore`: `vendor/` (except release snapshots), `crates/*/pg/`
      if not committed, build scratch directories.
- [x] Create `CHANGELOG.md` with an `Unreleased` section.
- [x] Confirm `docs/plans/` exists as the agent-writable working-artifact
      directory (`system-architecture.md` §1).

---

## 2. `crates/intermediate` (shared, non-Wasm)

This crate is the single source of truth for the versioned DTO and codec
(`system-architecture.md` §1, §4).

- [x] Define `IntermediateV1`: the version-1 DTO drawing fields, chosen to cover
      the supported `usvg::Tree` subset (lines/curves/fills/paint — per the
      implementer's choice, documented with a 1–3 sentence rationale per field,
      `system-architecture.md` §4).
- [x] Implement the canonical-CBOR envelope with `cbor_core`: top-level map,
      integer keys `0` (identifier `"svg2ui8a/usvg"`), `1` (unsigned format
      version, currently `1`), `2` (DTO payload) (`project-constitution.md`
      §2.3).
- [x] Implement `encode(&IntermediateV1) -> Vec<u8>` producing canonical CBOR
      only.
- [x] Implement `decode(&[u8]) -> Result<IntermediateV1, DecodeError>` that:
  - [x] Rejects non-canonical CBOR.
  - [x] Validates the format identifier exactly.
  - [x] Validates the format version is supported.
  - [x] Validates required fields, field types, and numeric ranges.
  - [x] Rejects unsupported DTO variants.
  - [x] Never panics on malformed input.
- [x] Implement conversion `usvg::Tree subset -> IntermediateV1` (used by
      `svg2usvg`).
- [x] Implement conversion `IntermediateV1 -> usvg::Tree` (used by `usvg2rgba`).
- [x] Confirm the DTO excludes text, raster images, BBoxes, animation state, and
      external resources (`project-constitution.md` §2.3, §3.1–§3.3).
- [x] Write unit tests: encode/decode round-trip, envelope shape, rejection of
      each invalid-input category above.

---

## 3. `crates/svg2usvg` (producer)

- [x] `core.rs`: implement the native function
      `svg(&str) -> Result<Vec<u8>, Error>`:
  - [x] Parse with feature-disabled `usvg`.
  - [x] Reject `<text>` content with an error (not a panic, not a best-effort
        payload).
  - [x] Reject `<image>` / raster-image content with an error.
  - [x] Convert the parsed tree to `IntermediateV1` via `intermediate`.
  - [x] Encode via `intermediate::encode`.
- [x] `lib.rs`: add the `#[wasm_bindgen]` wrapper exposing `Promise<Uint8Array>`
      and nothing else (no options parameter — `project-constitution.md` §4.1,
      §4.4).
- [x] Confirm this crate depends on `intermediate` and `usvg` only — no `resvg`,
      no `tiny-skia`, no `png` (`system-architecture.md` §3.1,
      `engineering-playbook.md` §7.2).
- [x] Rust native tests (`engineering-playbook.md` §3.1), at minimum:
  - [x] Simple SVG → non-empty `Vec<u8>`.
  - [x] Same SVG → identical bytes across two calls (determinism).
  - [x] Malformed SVG → error, not panic.
  - [x] `<text>` content → error, not payload.
  - [x] `<image>` content → error, not payload.
  - [x] Round-trip test through `intermediate`'s encode/decode.
  - [x] Envelope test: identifier, version, DTO key positions.
- [x] Verify resolved Cargo features exclude `text`, `system-fonts`,
      `memmap-fonts`, `raster-images` (`engineering-playbook.md` §3.6).

---

## 4. `crates/usvg2rgba` (consumer)

- [x] `core.rs`: implement the native function
      `rasterize(bytes: &[u8], options: Options) -> Result<RgbaResult, Error>`:
  - [x] Decode + semantically validate via `intermediate::decode`.
  - [x] Reconstruct a supported `usvg::Tree` from `IntermediateV1`.
  - [x] Rasterize with feature-disabled `resvg` into a `tiny_skia::Pixmap`.
  - [x] Apply sizing rule: both omitted → natural size; one set → other derived from natural size; both set → exact non-uniform scaling
        from natural size; both set → exact non-uniform scaling
        (`project-constitution.md` §4.3).
  - [x] Zero-initialize the buffer before drawing (`project-constitution.md`
        §5.2).
  - [x] Default to straight (non-premultiplied) alpha; support a
        premultiplied/"as-is" option (`project-constitution.md`
        §4.2, §5.2).
  - [x] Return dimensions and alpha-mode metadata alongside `pixels`.
- [x] `lib.rs`: add the `#[wasm_bindgen]` wrapper exposing `Promise<RgbaResult>`
      with the optional `Usvg2RgbaOptions` shape from `project-constitution.md`
      §4.2 (implementer chooses field/type names).
  - [x] wasm-bindgen String/Copy limitation resolved with #[wasm_bindgen(getter_with_clone)]
        `docs/plans/usvg2rgba-wasm-bindgen-blocker.md` for detailed analysis
        and proposed solutions.
- [ ] Confirm no PNG/WebP/JPEG encoder or decoder anywhere in this crate
      (`project-constitution.md` §3.3, §3.4).
- [ ] Rust native tests (`engineering-playbook.md` §3.2), at minimum:
  - [ ] Simple SVG → pixmap of expected natural size.
  - [ ] Only `width` set → `width × natural_h`.
  - [ ] Only `height` set → `natural_w × height`.
  - [ ] Both set, non-uniform aspect ratio → exact `width × height`.
  - [ ] Non-canonical / non-CBOR / non-package payload → error, not panic.
  - [ ] Unknown identifier / unsupported version / malformed DTO / unsupported
        DTO variant → error, not panic.
  - [ ] Zero-sized SVG → error.
  - [ ] Default alpha: 50%-opaque red → `(255, 0, 0, 128)`.
  - [ ] Premultiplied alpha: same input → `(128, 0, 0, 128)`.
  - [ ] Renderer determinism across two consecutive calls.
- [x] Verify resolved Cargo features exclude `text`, `system-fonts`,
      `memmap-fonts`, `raster-images`; confirm `tiny-skia` is reached only
      through `resvg` (`engineering-playbook.md` §3.6).

---

## 5. Wasm build pipeline

- [ ] Write `scripts/build.ts` (initial scaffolding, not follow-up work —
      `engineering-playbook.md` §2.1):
  - [ ] Compile `svg2usvg` to Wasm; copy output to `assets/svg2usvg_bg.wasm`.
  - [ ] Compile `usvg2rgba` to Wasm; copy output to `assets/usvg2rgba_bg.wasm`.
  - [ ] Regenerate `src/usvg.ts` and `src/rgba.ts` from templates.
  - [ ] Run the CDN-free check (§7 below) as part of the build; abort the build
        on failure.
- [ ] Write `scripts/vendor.ts`: reproducible vendoring of an approved new
      dependency into `vendor/<name>/`, with a pinned version recorded in a
      `VENDORED.md` (`engineering-playbook.md` §5.1). Not needed for the
      baseline dependency set, but the script must exist and work.
- [ ] Write `scripts/check-cdn-free.ts`: bundle each of the three subpath entry
      points and grep for `https://`; fail on any match
      (`engineering-playbook.md` §5.2).
- [ ] Confirm both Wasm artifacts are independently loadable and neither crate
      is bundled into the other (`project-constitution.md` §3.9).

---

## 6. TypeScript surface

- [ ] `src/usvg.ts` (generated, never hand-edited): wraps
      `svg2usvg(svg: string): Promise<Uint8Array>`, manages a single
      `initialized` flag.
- [ ] `src/rgba.ts` (generated, never hand-edited): wraps
      `usvg2rgba(usvg: Uint8Array, options?: Usvg2RgbaOptions): Promise<RgbaResult>`,
      manages a single `initialized` flag.
- [ ] `src/mod.ts` (hand-written): re-exports `svg2usvg`, `usvg2rgba`, and the
      public `RgbaResult` / `Usvg2RgbaOptions` type names.
- [ ] Confirm neither wrapper adds caching, memoization, pre-warming, a
      `dispose()` method, or a class-based API (`project-constitution.md` §4.4,
      `engineering-playbook.md` §7.4, §7.7).

---

## 7. Wasm-layer and Deno-layer tests

- [ ] Wasm tests (`deno task test:wasm`, `engineering-playbook.md` §3.3):
  - [ ] Each Wasm entry point returns the correct `Promise` type.
  - [ ] Wasm output matches the corresponding Rust-native `core` output for the
        same input.
- [ ] Deno tests (`deno test -A`, `engineering-playbook.md` §3.4):
  - [ ] `svg2usvg` returns a `Uint8Array`.
  - [ ] `usvg2rgba` returns `RgbaResult` with
        `pixels.length === width * height * 4`.
  - [ ] `RgbaResult.alphaMode` reflects the requested (or default) mode.
  - [ ] End-to-end: `svg2usvg` → `usvg2rgba({ width, height })` produces
        correctly sized RGBA.
  - [ ] **`.cbor` file round trip**: write `svg2usvg` output to a fixture
        `.cbor` file, read it back as `Uint8Array`, pass unchanged to
        `usvg2rgba`, and confirm the same `RgbaResult`
        (`cbor-file-intermediate.md` "Verification"; straightlines-vision.md
        §2.3's `Deno.readFile` scenario).
  - [ ] `init` idempotency: repeated calls to one function do not re-initialize;
        calling both functions does not cross-interfere.
  - [ ] Determinism: same SVG string → same `usvg` bytes, verified from the JS
        side.
  - [ ] Malformed SVG rejects the `svg2usvg` promise.
  - [ ] Malformed `usvg` payload rejects the `usvg2rgba` promise.
- [ ] Confirm `deno task test` runs all layers in order: `test:rust` →
      `test:wasm` → `deno test -A` (`engineering-playbook.md` §3).

---

## 8. Formatting, linting, and CI hygiene

- [ ] `cargo fmt` on all changed `.rs` files.
- [ ] `deno fmt` on all changed `.ts` / `.md` files.
- [ ] `deno task lint` passes.
- [ ] `deno task check` passes (type-check all `.ts`).
- [ ] Confirm formatters did not touch lines outside the current change; revert
      any incidental reformatting (`engineering-playbook.md` §1).

---

## 9. Documentation and changelog

- [x] Add an `Unreleased` entry to `CHANGELOG.md` describing the baseline
      implementation.
- [x] If any implementer judgment call was made that future agents should not
      re-litigate (DTO field names, error type, pixel-arithmetic details, Wasm
      import shape), record it in `docs/plans/baseline-implementation.md` with a
      short rationale (`engineering-playbook.md` §0).
- [x] Confirm no implementation detail contradicts the already-fixed boundary in
      `docs/project-constitution.md` or `docs/system-architecture.md`; if a
      constraint turns out to be unworkable, stop and escalate rather than
      silently deviating (`engineering-playbook.md` §0, `system-architecture.md`
      §4).

---

## 10. Final verification before declaring done

- [ ] All four test layers pass (§7 and §3 above).
- [ ] `deno task build` succeeds from a clean checkout and regenerates exactly:
      `assets/svg2usvg_bg.wasm`, `assets/usvg2rgba_bg.wasm`, `src/usvg.ts`,
      `src/rgba.ts`, `src/mod.ts`.
- [ ] The CDN-free check passes for all three subpath bundles.
- [ ] Two independent Wasm artifacts exist; neither includes the other's code
      path (`project-constitution.md` §3.9).
- [ ] Resolved Cargo features for `usvg` and `resvg` exclude `text`,
      `system-fonts`, `memmap-fonts`, `raster-images` in both artifacts.
- [ ] A `.cbor` file produced by `svg2usvg`, written to disk, and read back
      independently of `svg2usvg` rasterizes correctly via `usvg2rgba` —
      confirming the format is usable as a standalone file, not just an
      in-memory handle (this is the property a future Straightlines producer
      would rely on; see straightlines-vision.md §4.1).
- [ ] No `postcard`, MessagePack, bincode, JSON, PNG, WebP, font, or
      raster-image dependency appears anywhere in either Wasm artifact's
      dependency tree.
- [ ] Report completion per `AGENTS.md` §9.
