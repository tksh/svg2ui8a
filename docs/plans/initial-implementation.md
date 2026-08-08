# Plan: Initial implementation of `@tksh/svg2ui8a`

**Status:** awaiting human approval  
**Scope:** scaffold the package from the four governing documents and
ship a working first cut of both subpaths.  
**Out of scope:** `./png` subpath, convenience `svg→rgba` chain,
structured-object input, release publish / tags, parent web app.

Authorized by:

- Constitution §1–§2 (mission, two products)
- Constitution §3–§4 (constraints, API surface)
- Architecture §1–§5 (layout, crates, TS surfaces)
- Playbook §1–§3, §5 (commands, build, tests, CDN-free)

---

## 0. Blocking gate (must run first)

### Task 0 — Verify `usvg::Tree` ↔ `postcard` round-trip

**Why:** Playbook §3.1 and `AGENTS.md` §10 make this the only
blocking technical assumption. If it fails, stop; do not invent an
alternate format (constitution §3.7).

**Work:**

1. Create a minimal probe (temporary crate or the first real
   `crates/svg2usvg` skeleton) that:
   - Parses a simple SVG with the pinned `usvg` workspace version.
   - Runs `postcard::to_stdvec(&tree)`.
   - Decodes with `postcard::from_bytes`.
   - Asserts byte equality after a second encode.
2. Report pass/fail with the exact compiler or runtime error.

**Pre-flight note (probe already run, 2026-08-08):**

Against `usvg = "0.34"` + `postcard = "1"` on this machine:

- `Tree::from_str` requires `use usvg::TreeParsing;` (API detail).
- **`usvg::Tree` does not implement `serde::Serialize`**, so
  `postcard::to_stdvec(&tree)` does not compile.

That means **Task 0 is expected to fail** under the current pins.
Implementation of Tasks 1+ must not proceed until the human decides
how to resolve this (see **Questions for the human** below).

**Exit criteria:**

- [ ] Round-trip test green, **or**
- [ ] Human has chosen a constitution-level resolution and the
      governing docs are updated before any further implementation.

---

## 1. Repository scaffolding

### Task 1 — Workspace and package metadata

Create the skeleton described in architecture §1 (without claiming
Wasm/tests work yet).

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace root; `[workspace.dependencies]` pins `usvg`, `postcard`, `serde` (architecture §3.4). |
| `deno.json` | Tasks: `build`, `test`, `test:rust`, `test:wasm`, `fmt`, `lint`, `check` (architecture §2). |
| `jsr.json` | name `@tksh/svg2ui8a`, version `0.1.0`, exports `.` / `./usvg` / `./rgba`. |
| `.gitignore` | Extend existing: `target/`, `crates/*/pkg/`, `vendor/`, `node_modules/`, `_build/`, `dist/`. Keep `assets/*.wasm` trackable. |
| `CHANGELOG.md` | `## Unreleased` section ready for entries. |
| `README.md` | Minimal consumer-facing overview (imports, two subpaths, link to docs). Do not invent APIs beyond the constitution. |
| `docs/plans/` | Already present after this plan. |
| Empty dirs placeholders | `assets/`, `scripts/`, `tests/`, `src/`, `crates/…` as needed. |

**Dependency changes:** none beyond what architecture already lists
(once Task 0 is resolved).

**Exit criteria:**

- [ ] `deno task` lists the documented tasks (stubs allowed until
      Task 5/6).
- [ ] `cargo metadata` succeeds on the workspace once crates exist
      (Task 2/3 may land in the same PR as Task 1 if preferred).

---

## 2. `svg2usvg` crate (producer)

### Task 2 — Core + bindgen wrapper + native tests

Files (architecture §4.1, §1):

```
crates/svg2usvg/
  Cargo.toml
  src/lib.rs      # #[wasm_bindgen] thin wrapper
  src/core.rs     # svg2usvg_core — native-testable
  tests/          # optional integration tests
```

**Work:**

1. Implement `svg2usvg_core(svg: &str) -> Result<Vec<u8>, …>`:
   parse with default `usvg::Options`, encode with postcard.
2. Map errors without panicking (malformed SVG → error).
3. Thin `#[wasm_bindgen] pub fn svg2usvg` → `Result<Vec<u8>, JsError>`.
4. Native tests from playbook §3.1:
   - simple `<rect>` → non-empty bytes
   - `<path>` stability across two calls
   - malformed SVG → error, not panic
   - determinism: same SVG → same bytes
   - **postcard round-trip** (gate, already Task 0)

**Error type:** architecture sample uses `thiserror`. That crate is
**not** listed in architecture §3.1 / §9. Prefer a small manual
`enum` + `Display`/`Error` unless the human approves adding
`thiserror` (see questions).

**Exit criteria:**

- [ ] `cargo test` in `crates/svg2usvg` green (after Task 0 resolved).

---

## 3. `usvg2rgba` crate (consumer)

### Task 3 — Core + bindgen wrapper + native tests

Files (architecture §4.2):

```
crates/usvg2rgba/
  Cargo.toml
  src/lib.rs
  src/core.rs
  tests/
```

**Dependencies (architecture §3.2):** workspace `usvg` / `postcard` /
`serde`; plus `resvg = "0.34"`, `tiny-skia = "0.11"`,
`serde-wasm-bindgen = "0.6"`, `wasm-bindgen`.

**Work:**

1. `RgbaOptions` / `AlphaMode` / `RgbaResultCore` / `Usvg2RgbaError`
   as in architecture §4.2.
2. `usvg2rgba_core`:
   - decode postcard → `Tree`
   - reject zero natural size
   - resolve width/height with **independent scaling** (constitution
     §4.3): missing side uses natural size; both set → exact
     `width × height`
   - allocate zero-initialized pixmap
   - render via the real `resvg` API for the pinned version
     (architecture leaves the exact call as an impl-time confirm;
     expected shape is roughly
     `resvg::render(&tree, transform, &mut pixmap.as_mut())`)
   - `alphaMode: straight` (default) → unpremultiply;
     `premultiplied` → raw pixmap bytes
3. Bindgen wrapper: decode options with `serde_wasm_bindgen`,
   call core, serialize result to `JsValue` (simplest correct
   approach — architecture §4.2 leaves the choice open).
4. **Serde field names:** TS uses camelCase (`alphaMode`, not
   `alpha_mode`). Options decoding must accept the public TS names
   (e.g. `#[serde(rename_all = "camelCase")]` or per-field renames).
5. Native tests from playbook §3.2:
   - size / only-width / only-height / non-uniform both
   - bad postcard payload → error
   - zero natural size → error
   - alpha straight vs premultiplied golden pixels
   - renderer determinism (same input → same bytes)

**Cross-crate note:** producer and consumer share only workspace
pins; do not share source. For rgba tests that need postcard bytes,
either encode a tree in-test via the same postcard path or call
`svg2usvg_core` only if a deliberate test dependency is approved
(default: encode inside the rgba test crate using the same
workspace `usvg`+`postcard`, without depending on the `svg2usvg`
package).

**Exit criteria:**

- [ ] `cargo test` in `crates/usvg2rgba` green.

---

## 4. TypeScript public surface (hand-written pieces)

### Task 4 — `src/mod.ts` and type contracts

1. Hand-write `src/mod.ts` re-exports (architecture §5.3):
   `svg2usvg`, `usvg2rgba`, `AlphaMode`, `Usvg2RgbaOptions`,
   `RgbaResult`.
2. Do **not** hand-edit final `src/usvg.ts` / `src/rgba.ts` after
   the build script owns them (Task 5). Initial bootstrap may
   land template-shaped files that Task 5 regenerates.

**Exit criteria:**

- [ ] Public API types match constitution §4.1–§4.2.

---

## 5. Build pipeline

### Task 5 — `scripts/build.ts` + Wasm artifacts + wrapper regen

Playbook §2 is **baseline scaffolding**, not follow-up.

**Work:**

1. Record a pinned `wasm-pack` version inside `scripts/build.ts`
   (install strategy: document in README / script comment; agent
   must not use an unrecorded version).
2. For each crate:
   - `wasm-pack build --target deno`
   - copy `pkg/<crate>_bg.wasm` → `assets/<crate>_bg.wasm`
   - leave `pkg/*.js` uncommitted (gitignore)
3. Regenerate template-shaped `src/usvg.ts` and `src/rgba.ts` that
   import the **raw** `.wasm` from `assets/` (architecture §5,
   playbook §2.1).
4. Invoke CDN-free check as the last step of build (Task 7 can
   land the checker in the same change).

**Risk:** Deno raw `.wasm` imports with named `#[wasm_bindgen]`
exports may require extra glue or a specific wasm-bindgen/Deno
pattern. If the template import shape fails, **stop and escalate**
rather than quietly shipping the intermediate `pkg/*.js`.

**Toolchain prerequisites (this environment, 2026-08-08):**

- `rustc` / `cargo` / `deno` present.
- **`wasm-pack` not installed.**
- **No Chrome/Chromium** for headless `wasm-pack test` (Task 6).

**Exit criteria:**

- [ ] `deno task build` produces architecture §2.4 outputs.
- [ ] `assets/*.wasm` committed after a successful build.
- [ ] Regenerated wrappers are not hand-tweaked after generation.

---

## 6. Wasm-boundary and Deno tests

### Task 6a — `scripts/test-wasm.ts` + `deno task test:wasm`

Playbook §3.3:

- Headless browser tests that the Wasm exports match native `core`
  outputs for the same input.
- Wire `test:wasm` as documented.

If Chrome is unavailable in CI/dev, escalate with options (install
Chrome, use Firefox headless if wasm-pack supports it, or defer
Wasm-layer tests with human approval). Do not silently skip.

### Task 6b — Deno integration tests (`tests/` + optional `src/`)

Playbook §3.4 minimum cases:

- `svg2usvg` → `Uint8Array`
- `usvg2rgba` → `RgbaResult` with `pixels.length === w*h*4`
- `alphaMode` echo / default `"straight"`
- end-to-end `svg2usvg` → `usvg2rgba`
- init idempotence / no cross-Wasm init interference
- determinism of usvg bytes from JS
- malformed SVG / malformed usvg → rejected promises

**Exit criteria:**

- [ ] `deno task test` runs `test:rust` → `test:wasm` → `deno test -A`
      and all layers pass.

---

## 7. CDN-free check and vendor stub

### Task 7 — `scripts/check-cdn-free.ts` (+ hook into build)

Playbook §5.2:

- `deno bundle` each of `src/mod.ts`, `src/usvg.ts`, `src/rgba.ts`
- fail on any `https://` in the bundle text
- part of `deno task build`; failure aborts the build

### Task 8 — `scripts/vendor.ts` stub

Playbook §5.1:

- Stub (or minimal real script) that can recreate `./vendor/` from
  pinned sources when a non-JSR dep is approved later.
- `vendor/` gitignored in normal dev; release snapshot is
  **human-driven** (`deno task release` is not agent work for v0).

**No new runtime JS dependencies** expected for v0 if Rust crates
cover the logic and wrappers only import local Wasm.

**Exit criteria:**

- [ ] Build fails if a CDN URL appears in any of the three bundles.
- [ ] Vendor script exists and is documented; no accidental
      `vendor/` commit in day-to-day work.

---

## 8. Polish and handoff

### Task 9 — Changelog + README consumer docs

- `CHANGELOG.md` Unreleased: describe initial package surface.
- README: install via JSR, subpath examples, link to constitution
  constraints (no fonts, RGBA not PNG, cache-key guidance pointer).
- Do **not** bump `jsr.json` version or tag (playbook §6:
  human-driven).

### Task 10 — Full verification report

Per `AGENTS.md` §9:

```
deno task fmt     # only on touched files / revert incidental
deno task lint
deno task check
deno task build
deno task test
```

Report commands, results, file list, deviations, open follow-ups.

---

## Suggested implementation order (DAG)

```
Task 0 (postcard gate) ──fail──► STOP / human decision
        │
       pass
        │
        ▼
Task 1 scaffolding ──► Task 2 svg2usvg ──► Task 3 usvg2rgba
                              │                    │
                              └────────┬───────────┘
                                       ▼
                                 Task 4 src/mod.ts
                                       │
                                       ▼
                                 Task 5 build pipeline
                                       │
                          ┌────────────┼────────────┐
                          ▼            ▼            ▼
                      Task 6a      Task 6b      Task 7
                      test:wasm    deno tests   CDN-free
                                       │
                                       ▼
                                 Task 8 vendor stub
                                       │
                                       ▼
                                 Task 9 README/CHANGELOG
                                       │
                                       ▼
                                 Task 10 verify + report
```

Logical commits (one concern each), example messages:

1. `svg2ui8a: scaffold workspace, deno.json, jsr.json`
2. `svg2ui8a: add svg2usvg core and native tests`
3. `svg2ui8a: add usvg2rgba core and native tests`
4. `svg2ui8a: add Wasm build pipeline and TS wrappers`
5. `svg2ui8a: add Deno and Wasm boundary tests`
6. `svg2ui8a: add CDN-free check and vendor stub`

---

## Dependency changes (planned)

| Dep | Where | Tier | Justification |
|-----|-------|------|---------------|
| `usvg` (workspace pin `0.34`) | both crates | crates.io via Cargo | Required for SVG → Tree (architecture). |
| `postcard` (workspace) | both crates | crates.io | Constitution §3.7 fixed format. |
| `serde` (workspace) | both crates | crates.io | Required by postcard / options. |
| `wasm-bindgen` | both crates | crates.io | Wasm exports. |
| `resvg` `0.34` | usvg2rgba only | crates.io | Rasterize Tree → pixmap. |
| `tiny-skia` `0.11` | usvg2rgba only | crates.io | Pixmap buffer. |
| `serde-wasm-bindgen` `0.6` | usvg2rgba only | crates.io | JS options → Rust (architecture §3.2). |
| `wasm-bindgen-test` | both, dev | crates.io | Wasm tests. |
| `thiserror` | ??? | — | **Not approved** in architecture §9. Prefer manual errors unless human approves. |

No JSR/TS runtime dependencies planned for v0 beyond Deno std if
genuinely needed by scripts (justify in the change that adds them).

**Not in scope for agent:** publishing to JSR, pushing tags,
committing release `vendor/` snapshots.

---

## Files expected to exist when done

Matches architecture §1, plus this plan under `docs/plans/`.
Governing docs remain human-owned and unchanged by the agent.

---

## Questions for the human (blockers / judgment)

These need answers before or at Task 0 resolution. See the
message that accompanies this plan for the same list in short form.

1. **Postcard / `usvg::Tree` Serialize (BLOCKER)** — probe failed.
2. **`thiserror` vs manual errors**
3. **Tooling: install `wasm-pack`? Chrome for `test:wasm`?**
4. **`usvg` / `resvg` pin `0.34`** — keep, or allow newer matched pair
   after constitution/architecture update?
5. **Raw `.wasm` import fallback policy** if Deno cannot bind named
   exports without `pkg/*.js`
6. **Commit strategy** — one PR for all tasks after gate, or stacked
   PRs per DAG stage?

---

## Approval

Please reply with:

- approval to proceed after answering the questions, **or**
- a revised Task 0 resolution path if the postcard gate stays red.
