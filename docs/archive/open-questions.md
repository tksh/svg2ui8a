# Open questions and contradictions

This file records unclear points and contradictions found across the
four governing documents:

1. `AGENTS.md`
2. `docs/project-constitution.md`
3. `docs/system-architecture.md`
4. `docs/engineering-playbook.md`

It is a snapshot of document issues only. It does not propose
resolutions. Items are grouped by theme. Severity labels:

- **Contradiction** — two or more statements cannot both be true as
  written.
- **Unclear** — a decision is implied or required, but not specified
  tightly enough to implement without guessing.
- **Gap** — a cross-reference points at content that is missing, or a
  required artifact is named but never defined.

---

## 1. Agent process and documentation ownership

### 1.1 Plans live under `docs/`, but agents must not modify `docs/`

**Contradiction.**

| Source | Statement |
|--------|-----------|
| `AGENTS.md` §2 | For multi-file / non-trivial work, produce a written plan first, **save it under `./docs/`**, wait for approval. |
| `docs/engineering-playbook.md` §4 | Draft a plan in **`docs/plans/<feature>.md`**. |
| `AGENTS.md` §8 | **Do not modify any file under `./docs/`**. The human owns the documentation. |
| `AGENTS.md` §10 | Stop and ask if the plan would require modifying `AGENTS.md` or any `./docs/*.md`. |

An agent cannot both write `docs/plans/<feature>.md` and obey the
hard ban on modifying `./docs/`.

### 1.2 `AGENTS.md` §8 parenthetical mis-locates itself

**Unclear / minor error.**

`AGENTS.md` §8 says: do not modify any file under `./docs/` (*this
file*, or any of the `docs/*.md` files). `AGENTS.md` is not under
`./docs/`. It is unclear whether the ban is meant to cover `AGENTS.md`
only via the separate escalation rule in §10, or whether the
parenthetical is a drafting slip.

### 1.3 Repository scope: web app vs. package-only

**Unclear.**

| Source | Framing |
|--------|---------|
| `AGENTS.md` §1 | A **Deno-first web application**: `deno bundle`, Deno Deploy, planned dynamic OGP images, plus the `svg2ui8a` package. |
| Constitution / architecture / playbook | Entirely about the **JSR package** `@your-org/svg2ui8a`. No app layout, OGP route, or Deploy wiring. |

Open: is the repo a monorepo that will hold both app and package, or
is the app context only background for why the package exists? Agents
have no package-vs-app boundary for files outside the package tree
described in the architecture.

### 1.4 Package name `@your-org/svg2ui8a`

**Unclear.**

All three package docs use the placeholder scope `@your-org`. The
real JSR scope / package name is not decided in these documents.

---

## 2. `usvg2rgba` return type and the “everything is `Uint8Array`” rule

### 2.1 Constitution §2.2 vs. §4.2 disagree on the return type

**Contradiction.**

| Source | Return type |
|--------|-------------|
| Constitution §2.2 signature and prose | `Promise<Uint8Array>` (raw RGBA bytes only) |
| Constitution §4.2 | `Promise<RgbaResult>` with `{ pixels, width, height }` |
| Architecture §5.2, playbook §3.4 | `Promise<RgbaResult>` |

§4.2, the architecture, and the playbook agree. §2.2 does not.

### 2.2 Mission / anti-goal language conflicts with `RgbaResult`

**Contradiction** (if §4.2 is authoritative).

| Source | Statement |
|--------|-----------|
| Constitution §1 | “Everything in this package is `Uint8Array`.” |
| Constitution §8 anti-goal | “Producing anything other than `Uint8Array` for either function” is out of scope. |
| Constitution §4.2 / architecture / playbook | `usvg2rgba` returns a structured **`RgbaResult` object**, not a bare `Uint8Array`. |
| Playbook §8 default | “as long as it is a `Uint8Array` in and a `Uint8Array` out” |

Either `RgbaResult` is an allowed exception and the mission/anti-goal
wording is too absolute, or the API should return only pixels (and
then width/height need another channel).

### 2.3 Optional `width` / `height` omitted from §2.2 signature

**Contradiction / incomplete.**

Constitution §2.2 shows `usvg2rgba(usvg: Uint8Array)` with no options
argument, while the same section’s prose says the caller may pass
`width` and `height`, and §4.2 defines `Usvg2RgbaOptions`.

---

## 3. Dependencies and the Wasm / TS boundary

### 3.1 `serde_wasm_bindgen` used but not listed

**Contradiction / gap.**

| Source | Claim |
|--------|-------|
| Architecture §3.2 `Cargo.toml` | Dependencies: `usvg`, `postcard`, `resvg`, `tiny-skia`, `wasm-bindgen`, `serde`. |
| Architecture §4.2 sample | Uses `serde_wasm_bindgen::from_value(v)` for options. |
| Architecture §9 | “No other dependencies.” |

`serde_wasm_bindgen` is required by the sample API and absent from the
declared graph. Either it must be added (with justification) or
options must be decoded another way.

### 3.2 Direct `.wasm` import vs. wasm-pack JS glue

**Unclear / tension.**

| Source | Model |
|--------|-------|
| Architecture §5.1–5.2 | `import init, { … } from "../assets/svg2usvg_bg.wasm"` (and same for rgba). |
| Playbook §2.1–2.2 | `wasm-pack` emits `pkg/<crate>.js` **consumed by** `src/usvg.ts` / `src/rgba.ts`; Wasm is copied to `assets/`. |

It is not specified whether the committed wrapper imports the raw
`.wasm`, the generated `.js` glue (and from where that glue lives
after copy), or a hybrid. Deno’s import story and wasm-pack’s
`deno` target output need a single chosen shape.

### 3.3 Generated wrappers vs. optional hand-written wrappers

**Tension.**

| Source | Rule |
|--------|------|
| Architecture §5, §5.3 | Wrappers are auto-generated, committed; agent must **not** hand-edit `src/` files generated by `scripts/build.ts`. |
| Playbook §2.1 | “Emit the TS glue **(or a hand-written wrapper** that uses the wasm-bindgen output)”. |
| Playbook §2.3 | Do not hand-edit `src/usvg.ts` or `src/rgba.ts`; regenerate them. |

“Hand-written wrapper” in the build track conflicts with
regenerate-only / do-not-hand-edit, unless hand-written means
“written once by the build script’s template,” which is not stated.

### 3.4 Does `usvg::Tree` round-trip through `postcard`?

**Unclear (blocking technical assumption).**

Constitution and architecture assume:

- `postcard::to_stdvec(&tree)` after `Tree::from_str`
- `postcard::from_bytes` back to `usvg::Tree`

Whether `usvg::Tree` (at the pinned `0.34` or any current version)
implements the `serde` traits needed for stable postcard encode/decode
is not verified in the docs. If it does not, the entire intermediate
format is blocked and the constitution forbids alternate formats
(§3.7).

### 3.5 Sample `tree.render(...)` API may not match `resvg` / `usvg`

**Unclear.**

Architecture §4.2 calls `tree.render(transform, &mut pixmap.as_mut())`
as if render were a method on `usvg::Tree`. In the `resvg` ecosystem
the usual entry point is along the lines of `resvg::render(...)`, not
a method on `Tree`. The doc says the exact return wiring is decided
at implementation time, but the render call shape is still presented
as normative sample code.

### 3.6 Independent `usvg` pins on two crates

**Unclear / risk.**

Architecture pins `usvg = "0.34"` separately in each crate and states
the crates share no source. Architecture §8.5 and constitution §6
warn about version skew between producer and consumer postcard
layouts, but nothing requires the two crates to use the **same**
`usvg` (and `postcard`) versions, or to fail the build if they drift.

### 3.7 `npm:` vs. AGENTS dependency tiers

**Unclear.**

Constitution §3.6 allows Deno-style specifiers including **`npm:`**.
`AGENTS.md` §5 ranks JSR → local/vendored → `deno.land/x` / `esm.sh`,
and does not mention `npm:`. Whether `npm:` is allowed for package
code, only for the parent app, or never in the published package is
unspecified.

---

## 4. Vendoring, commits, and CDN-free builds

### 4.1 `vendor/` must be used and must not be committed

**Contradiction.**

| Source | Rule |
|--------|------|
| `AGENTS.md` §5.1 | Non-JSR deps are vendored under `./vendor/`. |
| Playbook §5.1 | Vendored tree under `./vendor/<name>/`, with `VENDORED.md` and `scripts/vendor.ts`. |
| `AGENTS.md` §7 | Do **not** commit `vendor/` (except explicitly vendored Wasm binaries). |

If `./vendor/` is gitignored / not committed, clean checkouts cannot
reproduce CDN-free builds from the repo alone unless another
mechanism (e.g. regenerate-from-lock in CI) is defined. It is not.

### 4.2 What exactly is committed under `assets/`?

**Mostly aligned, slightly unclear.**

Architecture and playbook treat `assets/*_bg.wasm` as build outputs
that are committed after `deno task build`. `AGENTS.md` §7 exempts
“Wasm binaries the project explicitly vendors.” Confirm that
`assets/*.wasm` are that exemption, and that intermediate
`crates/*/pkg/` is not committed.

### 4.3 CDN-free check targets “the browser bundle”

**Unclear for a library package.**

Playbook §5.2: after build, `deno bundle` an entry point and fail on
`https://` in the output. For a JSR library with subpath exports, it
is unclear:

- which entry point(s) are bundled for the check (root, `/usvg`,
  `/rgba`, all?);
- whether the check is about **this package’s** ship artifacts or
  about a **consumer app** bundle that depends on it;
- how embedded/loaded Wasm URLs or data URLs are treated if they
  appear as strings.

### 4.4 IHDR helper is required by AGENTS but missing from the playbook

**Gap.**

| Source | Claim |
|--------|-------|
| `AGENTS.md` §6, §8 | Use the “PNG IHDR-only size helper from `./docs/engineering-playbook.md`”; do not add a PNG decoder dependency for dimensions. |
| Constitution §3.4 | IHDR helper is a **consumer** concern; the package itself contains **no** PNG-decoding code. |
| Playbook | **No IHDR helper is defined** anywhere in the file. |

Open: does the helper belong in the parent app, in package tests
only, or in the package? The playbook does not contain the prescribed
snippet.

---

## 5. Testing

### 5.1 Visual / pixel tests: “if any” vs. “there are none”

**Tension (soft).**

| Source | Statement |
|--------|-----------|
| `AGENTS.md` §6 | Visual / pixel-equality tests, **if any**, need human review before pass. |
| Playbook §3.5 | **There are none.** Package outputs RGBA, not PNG. |

RGBA buffer equality tests are not the same as PNG visual diffs.
Whether byte-level RGBA golden tests are allowed, forbidden, or
“visual” under AGENTS §6 is unclear. Playbook required cases check
**size** and flow, not pixel values.

### 5.2 `deno task test:wasm` vs. “do not invent commands”

**Tension.**

| Source | Rule |
|--------|------|
| Playbook §1 | Agent must not invent commands; only the listed `deno task`s. |
| Playbook §3.3 | Run via `deno task test:wasm` (**agent should add this task if it does not exist**). |
| Architecture `deno.json` example | Tasks: `build`, `test`, `fmt`, `lint`, `check` — **no** `test:wasm`. |

Adding `test:wasm` is both required and arguably “inventing” a
command relative to the architecture example.

### 5.3 How native `cargo test` calls `#[wasm_bindgen]` exports

**Unclear.**

Playbook §3.1–3.2 require native tests of `svg2usvg` / `usvg2rgba`
directly. Those functions are shown as `#[wasm_bindgen]` exports
returning `Result<…, JsError>`. The docs do not say whether a pure
Rust core (non-bindgen) is required for native tests, or how
`JsError` / bindgen types are expected to behave under plain
`cargo test`.

### 5.4 Playbook `deno task test` “Deno + Rust” vs. task split

**Unclear.**

Playbook §1: `deno task test` runs the test suite **(Deno + Rust)**.
§3 still describes separate layers (`cargo test`, `wasm-pack test`,
`deno test`). Whether `deno task test` orchestrates all four layers
or only Deno tests is not specified; architecture’s example is only
`deno test -A`.

---

## 6. Formatting, release, and layout completeness

### 6.1 Format only touched lines vs. format the tree

**Tension.**

| Source | Rule |
|--------|------|
| `AGENTS.md` §4 | Run `deno fmt` before done; if it would change lines you did not edit, **leave them alone** — only format lines you changed. |
| Playbook §1 | `deno task fmt` formats **all** `.ts` files in `src/` and `tests/`. |

Operational expectation for agents is ambiguous when the tree has
pre-existing drift.

### 6.2 Version bump and changelog “as part of every change”

**Unclear (process).**

Playbook §6: keep `CHANGELOG.md` up to date and bump `jsr.json`
version **as part of every change**, and tag `v<version>`.
Constitution §7 defines strict semver including “minors: additions…
none planned.” For pre-release / `0.x` work it is unclear whether
every PR is a version bump + tag, or only human-driven releases.
`AGENTS.md` forbids push without approval; local tags without push
are allowed but easy to get out of sync with JSR.

### 6.3 Architecture package tree omits files other docs require

**Gap.**

Architecture §1 layout lists `jsr.json`, `README`, `LICENSE`,
`deno.json`, `src/`, `crates/`, `assets/`. Other docs require or
imply at least:

- `scripts/build.ts` (and playbook: `scripts/vendor.ts`)
- `tests/`
- `docs/` (and playbook: `docs/plans/`)
- `CHANGELOG.md`
- `vendor/` (when used)
- `AGENTS.md`

Not fatal, but the “canonical” tree is incomplete relative to the
playbook.

### 6.4 Root re-export and tree-shaking

**Unclear.**

Architecture §2: root re-exports both subpaths; bundlers are
“expected to tree-shake” unused subpaths. With separate Wasm modules
and lazy `init()` on first call, whether a static
`export { svg2usvg } from "./usvg.ts"` in `mod.ts` pulls both Wasm
artifacts into a bundle that only imports one symbol is
bundler-dependent. The docs do not state a supported bundler matrix
beyond “Deno and modern browser bundlers” (constitution §8).

---

## 7. Rendering semantics (underspecified contracts)

### 7.1 Premultiplied vs. straight alpha

**Unclear.**

Constitution §5.2 defines packed `R, G, B, A` byte order and layout.
It does not say whether channels are **premultiplied** (common for
`tiny-skia` pixmaps) or straight alpha. Consumers encoding PNG or
drawing to canvas need this.

### 7.2 Non-integer or zero natural sizes; only one of width/height set

**Unclear.**

- Natural size comes from `tree.size()` cast with `as u32` in the
  architecture sample (truncation? rounding?).
- Constitution §4.2: errors if width/height are not positive integers
  when requested — behavior when **only one** of `width` / `height`
  is set is not defined (preserve aspect ratio? use natural for the
  other? stretch?).
- Architecture scales with independent X/Y factors from the two
  dimensions, which implies **non-uniform stretch** when only one
  side is overridden — if that is intentional, it should be stated
  in the constitution.

### 7.3 Transparent background and “no background” vs. pixmap clear value

**Unclear (minor).**

Constitution §4.3: no background parameter; canvas is transparent.
Architecture allocates a new pixmap and renders. Whether the pixmap
is guaranteed cleared to zero alpha before render is left to
`tiny-skia` defaults and should be part of the contract if tests will
assert pixel values later.

### 7.4 End-to-end diagram mentions cached PNG

**Unclear (scope bleed).**

Architecture §6.3 cache-hit path: “return cached RGBA / **PNG**.”
The package does not produce PNG. This is consumer-side narrative,
but it sits inside the package architecture doc and can be read as
package behavior.

---

## 8. Cross-document consistency checklist (quick reference)

| Topic | AGENTS | Constitution | Architecture | Playbook | Status |
|-------|--------|--------------|--------------|----------|--------|
| Two separate Wasm artifacts | Yes | §3.9 | §3, §9 | §2, §7.9 | Aligned |
| postcard only | Yes | §3.7 | §3–4 | §7.3 | Aligned |
| No fonts / no BBox / no native / no Node | Yes | §3.1–3.6 | — | — | Aligned |
| `usvg2rgba` returns | — | §2.2 `Uint8Array` **vs** §4.2 `RgbaResult` | `RgbaResult` | `RgbaResult` | **Conflict** |
| Plans under `docs/` | §2 write / §8 forbid | — | — | §4 write | **Conflict** |
| Commit `vendor/` | §7 forbid | — | — | §5.1 use | **Conflict** |
| IHDR helper | Required from playbook | Consumer-only; not in package | — | **Missing** | **Gap** |
| `serde_wasm_bindgen` | — | — | Used, not listed | — | **Gap** |
| `test:wasm` task | — | — | Not in example | Add if missing | **Tension** |
| Visual tests | Review if any | — | — | None | Soft tension |

---

## 9. Suggested resolution order (non-binding)

These are reading aids only; the human decides:

1. Resolve **§2** (`RgbaResult` vs bare `Uint8Array` / mission wording) —
   blocks API and tests.
2. Resolve **§1.1** (where plans live vs. docs ownership) — blocks any
   multi-file agent work process.
3. Resolve **§3.4** (postcard ↔ `usvg::Tree` feasibility) and **§3.1**
   (options decoding dependency) — block the Rust design.
4. Resolve **§3.2–3.3** (wrapper generation and import shape) — block
   the build script.
5. Resolve **§4.1** and **§4.4** (vendor commit policy; IHDR helper
   home) — block dependency and test tooling policy.
6. Then rendering semantics (**§7**) and process polish (**§5–6**).

---

## 10. Document history

- Created to capture contradictions and unclear points across
  `AGENTS.md`, `docs/project-constitution.md`,
  `docs/system-architecture.md`, and `docs/engineering-playbook.md`
  as they stood when this file was written.
- No resolutions are recorded here; update or delete items when the
  human edits the source documents.
