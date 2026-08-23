# `@tksh/svg2ui8a` — Implementation Task List

This checklist turns `AGENTS.md`, `docs/project-constitution.md`,
`docs/system-architecture.md`, and `docs/engineering-playbook.md` into a
concrete, ordered build plan. It assumes the repository currently contains only
documentation (no source yet).

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
  - [x] Apply sizing rule: both omitted → natural size; one set → other derived
        from natural size; both set → exact non-uniform scaling from natural
        size; both set → exact non-uniform scaling (`project-constitution.md`
        §4.3).
  - [x] Zero-initialize the buffer before drawing (`project-constitution.md`
        §5.2).
  - [x] Default to straight (non-premultiplied) alpha; support a
        premultiplied/"as-is" option (`project-constitution.md` §4.2, §5.2).
  - [x] Return dimensions and alpha-mode metadata alongside `pixels`.
- [x] `lib.rs`: add the `#[wasm_bindgen]` wrapper exposing `Promise<RgbaResult>`
      with the optional `Usvg2RgbaOptions` shape from `project-constitution.md`
      §4.2 (implementer chooses field/type names).
  - [x] wasm-bindgen String/Copy limitation resolved with
        #[wasm_bindgen(getter_with_clone)]
        `docs/plans/usvg2rgba-wasm-bindgen-blocker.md` for detailed analysis and
        proposed solutions.
- [x] Confirm no PNG/WebP/JPEG encoder or decoder anywhere in this crate
      (`project-constitution.md` §3.3, §3.4).
- [x] Rust native tests (`engineering-playbook.md` §3.2), at minimum:
  - [x] Simple SVG → pixmap of expected natural size.
  - [x] Only `width` set → `width × natural_h`.
  - [x] Only `height` set → `natural_w × height`.
  - [x] Both set, non-uniform aspect ratio → exact `width × height`.
  - [x] Non-canonical / non-CBOR / non-package payload → error, not panic.
  - [x] Unknown identifier / unsupported version / malformed DTO / unsupported
        DTO variant → error, not panic.
  - [x] Zero-sized SVG → error.
  - [x] Default alpha: 50%-opaque red → `(255, 0, 0, 128)`.
  - [x] Premultiplied alpha: same input → `(128, 0, 0, 128)`.
  - [x] Renderer determinism across two consecutive calls.
- [x] Verify resolved Cargo features exclude `text`, `system-fonts`,
      `memmap-fonts`, `raster-images`; confirm `tiny-skia` is reached only
      through `resvg` (`engineering-playbook.md` §3.6).

---

## 5. Wasm build pipeline

- [x] Write `scripts/build.ts` (initial scaffolding, not follow-up work —
      `engineering-playbook.md` §2.1):
  - [x] Compile `svg2usvg` to Wasm; copy output to `assets/svg2usvg_bg.wasm`.
  - [x] Compile `usvg2rgba` to Wasm; copy output to `assets/usvg2rgba_bg.wasm`.
  - [x] Regenerate `src/usvg.ts` and `src/rgba.ts` from templates.
  - [x] Run the CDN-free check (§7 below) as part of the build; abort the build
        on failure.
- [x] Write `scripts/vendor.ts`: reproducible vendoring of an approved new
      dependency into `vendor/<name>/`, with a pinned version recorded in a
      `VENDORED.md` (`engineering-playbook.md` §5.1). Not needed for the
      baseline dependency set, but the script must exist and work.
- [x] Write `scripts/check-cdn-free.ts`: bundle each of the three subpath entry
      points and grep for `https://`; fail on any match
      (`engineering-playbook.md` §5.2).
- [x] Confirm both Wasm artifacts are independently loadable and neither crate
      is bundled into the other (`project-constitution.md` §3.9).

---

## 6. TypeScript surface

- [x] `src/usvg.ts` (generated, never hand-edited): wraps
      `svg2usvg(svg: string): Promise<Uint8Array>`, manages a single
      `initialized` flag.
- [x] `src/rgba.ts` (generated, never hand-edited): wraps
      `usvg2rgba(usvg: Uint8Array, options?: Usvg2RgbaOptions): Promise<RgbaResult>`,
      manages a single `initialized` flag.
- [x] `src/mod.ts` (hand-written): re-exports `svg2usvg`, `usvg2rgba`, and the
      public `RgbaResult` / `Usvg2RgbaOptions` type names.
- [x] Confirm neither wrapper adds caching, memoization, pre-warming, a
      `dispose()` method, or a class-based API (`project-constitution.md` §4.4,
      `engineering-playbook.md` §7.4, §7.7).

---

## 7. Wasm-layer and Deno-layer tests

- [x] Wasm tests (`deno task test:wasm`, `engineering-playbook.md` §3.3):
  - [x] Each Wasm entry point returns the correct `Promise` type.
  - [x] Wasm output matches the corresponding Rust-native `core` output for the
        same input.
- [x] Deno tests (`deno test -A`, `engineering-playbook.md` §3.4):
  - [x] `svg2usvg` returns a `Uint8Array`.
  - [x] `usvg2rgba` returns `RgbaResult` with
        `pixels.length === width * height * 4`.
  - [x] `RgbaResult.alphaMode` reflects the requested (or default) mode.
  - [x] End-to-end: `svg2usvg` → `usvg2rgba({ width, height })` produces
        correctly sized RGBA.
  - [x] **`.cbor` file round trip**: write `svg2usvg` output to a fixture
        `.cbor` file, read it back as `Uint8Array`, pass unchanged to
        `usvg2rgba`, and confirm the same `RgbaResult`
        (`cbor-file-intermediate.md` "Verification"; straightlines-vision.md
        §2.3's `Deno.readFile` scenario).
  - [x] `init` idempotency: repeated calls to one function do not re-initialize;
        calling both functions does not cross-interfere.
  - [x] Determinism: same SVG string → same `usvg` bytes, verified from the JS
        side.
  - [x] Malformed SVG rejects the `svg2usvg` promise.
  - [x] Malformed `usvg` payload rejects the `usvg2rgba` promise.
- [x] Confirm `deno task test` runs all layers in order: `test:rust` →
      `test:wasm` → `deno test -A` (`engineering-playbook.md` §3).

---

## 8. Browser-verified Wasm tests (Astral, headless Chrome)

`engineering-playbook.md` §3.3 requires headless-browser execution. §7's
`test:wasm` currently runs entirely inside the Deno runtime and does not launch
a real browser. This section closes that gap using Astral (CDP, no chromedriver,
no npm).

- [x] **8.1 Plan.** Write `docs/plans/wasm-browser-tests.md` per
      `engineering-playbook.md` §4. Do not implement anything in this step. Wait
      for explicit approval before starting §8.2.
- [x] **8.2 Pinned Chrome acquisition only.** Add a setup script/task that
      fetches a specific, pinned Chrome build via Astral, with no `sudo`, no
      `apt`, no npm package added anywhere. Verify it runs and prints the
      resolved binary path/version. No test harness yet.
- [x] **8.3 Minimal harness, one entry point.** A single browser test that loads
      `src/usvg.ts` in headless Chrome via Astral/CDP and calls `svg2usvg` once,
      asserting a non-empty `Uint8Array`. Nothing else yet.
- [x] **8.4 Second entry point.** Extend the harness to also load `src/rgba.ts`
      and call `usvg2rgba`, asserting correct `RgbaResult` shape.
- [x] **8.5 Full §3.3 required cases.** Fill in the remaining required test
      cases from `engineering-playbook.md` §3.3 (Promise types, output matches
      Rust-native core, etc.) inside the browser harness.
- [x] **8.6 Wire into `deno task test`.** Add the browser layer to the pipeline
      and confirm order: `test:rust` → `test:wasm` (Deno-side) → browser layer →
      `deno test -A`.
- [x] **8.7 Document the decision.** Update `engineering-playbook.md` §3.3 to
      state Astral/CDP as the fixed interpretation of "or equivalent", and add a
      CHANGELOG "Unreleased" entry.
- [x] **8.8 CI note.** Record (in the plan or playbook) what a CI runner would
      need to do differently from local WSL2, if anything.

---

## 9. Formatting, linting, and CI hygiene

- [x] `cargo fmt` on all changed `.rs` files.
- [x] `deno fmt` on all changed `.ts` / `.md` files.
- [x] `deno task lint` passes.
- [x] `deno task check` passes (type-check all `.ts`).
- [x] Confirm formatters did not touch lines outside the current change; revert
      any incidental reformatting (`engineering-playbook.md` §1).

---

## 10. Documentation and changelog

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

## 11. Final verification before declaring done

- [x] All four test layers pass (§7 and §3 above).
- [x] `deno task build` succeeds from a clean checkout and regenerates exactly:
      `assets/svg2usvg_bg.wasm`, `assets/usvg2rgba_bg.wasm`, `src/usvg.ts`,
      `src/rgba.ts`, `src/mod.ts`.
- [x] The CDN-free check passes for all three subpath bundles.
- [x] Two independent Wasm artifacts exist; neither includes the other's code
      path (`project-constitution.md` §3.9).
- [x] Resolved Cargo features for `usvg` and `resvg` exclude `text`,
      `system-fonts`, `memmap-fonts`, `raster-images` in both artifacts.
- [x] A `.cbor` file produced by `svg2usvg`, written to disk, and read back
      independently of `svg2usvg` rasterizes correctly via `usvg2rgba` —
      confirming the format is usable as a standalone file, not just an
      in-memory handle (this is the property a future Straightlines producer
      would rely on; see straightlines-vision.md §4.1).
- [x] No `postcard`, MessagePack, bincode, JSON, PNG, WebP, font, or
      raster-image dependency appears anywhere in either Wasm artifact's
      dependency tree.
- [x] Report completion per `AGENTS.md` §9.
