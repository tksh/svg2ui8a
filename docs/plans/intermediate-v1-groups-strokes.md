# Plan: Intermediate v1 extension — layers (`<g>`), strokes, and the Straightlines fixture

Task: extend `crates/intermediate` so the representation faithfully carries
layers (`<g>` groups, including group opacity), stroke color (`stroke`), stroke
width (`stroke-width`), stroke opacity (`stroke-opacity`), and the remaining
stroke properties usvg resolves — so that at minimum
`tests/fixtures/straightlines-sample.svg` survives `svg2usvg` → `.cbor` →
`usvg2rgba` without losing any graphical element, and
`tests/fixtures/straightlines-sample.cbor` can be generated from the `.svg`.

Status: **draft — awaiting human approval before any implementation**
(`engineering-playbook.md` §4 step 3).

---

## 1. Authorization — quoted

### 1.1 `docs/project-constitution.md` §2.3 (DTO fields are implementation-defined)

> The DTO's individual drawing fields are implementation-defined, within these
> constraints:
>
> - Be encoded as canonical **CBOR** (RFC 8949).
> - Carry enough information for `usvg2rgba` to render the same visual result
>   that the original SVG would have produced.
> - Be the **same format** on both ends — `svg2usvg` writes it and `usvg2rgba`
>   reads it, including when the bytes came from a `.cbor` file.
> - Be **deterministic** for a given supported SVG input, so that the bytes can
>   be hashed to produce a stable cache key.
> - Be a package-owned DTO mapped to and from the supported `usvg::Tree` subset;
>   it must not directly serialize `usvg::Tree`.
> - Exclude text, raster images, BBoxes, animation state, and external
>   resources.

This section authorizes changing the version-1 DTO drawing fields. The second
bullet is the one the current DTO violates for this fixture: stroke-only paths
are dropped entirely (`collect_shapes` skips any `Node::Path` without a
supported _fill_), so the rendered result is not "the same visual result that
the original SVG would have produced".

### 1.2 `docs/system-architecture.md` §4 (the plan must record the DTO)

> The implementation's plan records:
>
> - The version-1 DTO drawing fields and its mapping to the supported tree
>   subset.
> - The mapping between the representation and `cbor_core::Value`.
> - A short rationale (1–3 sentences) for each.

Section 3 below is that record.

### 1.3 `docs/project-constitution.md` §7 (why FORMAT_VERSION stays 1)

> Patches: Wasm runtime tweaks that preserve supported format versions and their
> output bytes for any input. Minors: additions to the API surface or support
> for a new, backward-compatible format version; version 1 remains decodable.
> Majors: incompatible changes to an existing format version …

**Human correction during plan approval:** the package is _not_ unpublished —
`@tksh/svg2ui8a@0.1.0` was manually published to jsr.io by the human. Per that
same approval, `FORMAT_VERSION` **stays `1`** and the change ships as package
version **`0.1.1`** (`deno.json` bumped in this change set; the actual
`deno publish` remains the human's action, playbook §6). This is the owner's
explicit decision for major-zero development: semver marks `0.y.z` as
initial-development where anything may change, and no external consumer bytes
are known to depend on the old v1 DTO shape. Payloads encoded by the old
pre-extension code will be rejected by the new decoder (missing required keys) —
acceptable per the approval. No committed `.cbor` fixture existed before; this
task creates the first one. The decision is recorded here rather than by
amending constitution §7 (which the agent must not edit, `AGENTS.md` §8).

### 1.4 Constraints honored (not restated in full)

CBOR via `cbor-core` 0.10.1 only (§3.7); no fonts/raster images/BBox (§3.1–
§3.3); two independent Wasm artifacts, both rebuilt and recommitted (§3.9); no
TS API surface change at all (§4 — `Usvg2RgbaOptions`/`RgbaResult` untouched).

---

## 2. Why the fixture is currently lossy

`tests/fixtures/straightlines-sample.svg` is four `<g id="LayerN">` groups of
stroke-only `<path>` elements:

| Group    | Inherited / own properties                            | Paths |
| -------- | ----------------------------------------------------- | ----- |
| `Layer0` | `stroke="#888" stroke-width="31"`                     | 2     |
| `Layer1` | `stroke="#000" stroke-width="1"`                      | 3     |
| `Layer2` | `opacity="0.8" stroke="#333" stroke-width="3"`        | 3     |
| `Layer3` | `stroke="#fff" stroke-opacity="0.9" stroke-width="5"` | 3     |

Today `collect_shapes` (`crates/intermediate/src/lib.rs:335`) keeps a path only
if `path.fill()` is `Some` with a `Paint::Color` paint, records only
`path_data` + `fill` + one merged `opacity`, and never looks at `path.stroke()`.
Every path in the fixture has zero-area geometry (its visible appearance is 100%
stroke), so the produced DTO renders an empty canvas: all eleven graphical
elements are lost. Group membership and group `opacity` are flattened away as
well.

---

## 3. Design decisions (implementer judgment, recorded per playbook §0)

### 3.1 Hierarchical DTO, not flattened shapes — chosen

```rust
pub enum Paint { Color(u32) }              // unchanged

pub enum LineCap { Butt, Round, Square }   // mirrors usvg::LineCap
pub enum LineJoin { Miter, MiterClip, Round, Bevel } // mirrors usvg::LineJoin

pub struct Stroke {
    pub paint: Paint,            // Color only (see 3.4)
    pub opacity: f32,            // stroke-opacity, [0,1]
    pub width: f32,              // stroke-width, > 0, finite
    pub linecap: LineCap,
    pub linejoin: LineJoin,
    pub miterlimit: f32,         // >= 1.0, finite
    pub dasharray: Option<Vec<f32>>, // finite, non-negative entries
    pub dashoffset: f32,         // finite
}

pub struct Shape {
    pub path_data: Vec<u8>,      // unchanged (SVG `d` text, abs coords)
    pub fill: Option<Paint>,     // unchanged meaning
    pub fill_opacity: f32,       // renamed from `opacity` (now unambiguous)
    pub stroke: Option<Stroke>,
}

pub struct Group {
    pub opacity: f32,            // group compositing opacity, [0,1]
    pub children: Vec<Node>,
}

pub enum Node { Shape(Shape), Group(Group) }

pub struct IntermediateV1 {
    pub root: Group,             // replaces `shapes: Vec<Shape>`
    pub size: (u32, u32),        // unchanged
}
```

Rationale: group `opacity` is a _compositing_ property, not a per-shape paint
property. Flattening it into each child's effective opacity (what
`collect_shapes` does today) renders differently from the original SVG wherever
children of a translucent group overlap — each overlap pixel gets darkened once
per shape instead of once for the whole group. The fixture's `Layer2` happens to
contain non-overlapping strokes, but baking that coincidence into the format
would contradict constitution §2.3's "same visual result" bullet. Preserving the
tree costs one recursive enum and lets `to_tree` emit nested `<g
opacity="…">`,
so `resvg` performs true group compositing on the consumer side.

_Rejected alternative:_ flat `Vec<Shape>` with group opacity multiplied into a
per-shape effective opacity. Smaller diff, visually equivalent for this specific
fixture, unfaithful in general. Not chosen.

### 3.2 Full usvg `Stroke` subset, not just color/width/opacity — chosen

`dasharray`, `dashoffset`, `linecap`, `linejoin`, `miterlimit` ride along even
though the fixture uses defaults, because usvg 0.47 has already resolved them
onto every `Path` (`crates/svg2usvg` pays nothing), and omitting them would make
any dashed/capped/joined SVG silently render with wrong stroke style — exactly
the class of silent loss this task exists to eliminate. Decode validates each:
width finite > 0, miterlimit finite ≥ 1, opacity ∈ [0,1], dasharray entries
finite ≥ 0, dashoffset finite, enum tags ∈ known variants.

### 3.3 `Shape.opacity` renamed to `fill_opacity`

Now that strokes carry their own opacity, the bare name is ambiguous. Pre-
release rename, no compatibility shim.

### 3.4 Unsupported paints: whole path dropped (status quo, documented)

If a path's fill **or** stroke paint is a gradient/pattern, the path is skipped
entirely (same policy as today for fills). Representing such a path with a
substituted paint would corrupt rendering more than omitting it. A path with
`fill="none"` (fill `None`) and/or no stroke is kept — including a path with
neither, which renders nothing but preserves structure losslessly. Text/image
rejection upstream in `svg_core.rs` is unchanged.

### 3.5 `to_tree` keeps its SVG-text reconstruction strategy

Reconstruction emits explicit, fully-resolved attributes per path (`fill`,
`fill-opacity`, `stroke`, `stroke-opacity`, `stroke-width`, `stroke-linecap`,
`stroke-linejoin`, `stroke-miterlimit`, `stroke-dasharray`, `stroke-dashoffset`)
plus nested `<g opacity="…">`. Inheritance correctness is guaranteed because
every value is emitted explicitly; groups carry only opacity (group `id`s are
dropped — ids matter only for clip/mask/filter references, none of which v1
supports). Rust `f32` `Display` round-trips coordinates losslessly, and the
fixture's coordinates are small integers, so byte-exact pixel equality against a
reference render (section 5.3) is achievable. The existing empty-`path_data` →
full-canvas `<rect>` shorthand is preserved verbatim (consumer native tests
depend on it).

### 3.6 Decode enforces a max group-nesting depth

Recursive decode/re-encode/tree-reconstruction of adversarially deep nesting
could overflow the stack ("never panics on malformed input", task.md §2). Decode
rejects nesting deeper than a constant (128) with `InvalidPayload`.

### 3.7 Canonical-CBOR mapping (recorded per architecture §4)

Same envelope: map keys `0`/`1`/`2` → identifier `"svg2ui8a/usvg"`, version `1`,
payload. Payload map gains no new top-level keys (`root` replaces `shapes`;
`size` unchanged). Encoding follows the existing pattern — string keys,
insertion order, `Value::map`/`Value::array`/`Value::from` — so determinism
continues to hold by construction (same input → same insertion order → same
bytes):

- `root`: recursive map — `"t": 1` (group) with `"opacity"` + `"children"`
  (array of node maps), or `"t": 0` (shape) with the shape map inline:
  `"path_data"` (bytes), `"fill"` (`0` = none | `[1, u32]` = Color, unchanged),
  `"fill_opacity"` (f64), `"stroke"` (`0` = none | map), `"size"` unchanged.
- stroke map: `"paint"` (`[1, u32]`), `"opacity"`, `"width"`, `"linecap"`
  (`0..=2`), `"linejoin"` (`0..=3`), `"miterlimit"`, `"dasharray"` (`0` = none |
  array of f64), `"dashoffset"`.

The integer discriminator `t` distinguishes shape from group without relying on
key presence. Decode validates every field's type and range before constructing
the DTO (existing style).

---

## 4. Files touched

| File                                                                                | Change                                                                                                                                                |
| ----------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/intermediate/src/lib.rs`                                                    | DTO types, encode, decode + validation (incl. depth limit), `from_tree`, `to_tree`                                                                    |
| `crates/intermediate/tests/core.rs`                                                 | Round-trip + rejection tests for the extended schema                                                                                                  |
| `crates/svg2usvg/src/lib.rs`                                                        | Tests only (core path is unchanged — `from_tree` keeps its signature); fixture structural test added                                                  |
| `crates/usvg2rgba/tests/core.rs`                                                    | Hand-built payload helpers updated to new schema; golden pixel-equality test added                                                                    |
| `crates/svg2usvg/examples/dump_core.rs`, `crates/svg2usvg/src/svg_core.rs`          | Expected zero-diff (verified during impl)                                                                                                             |
| `tests/fixtures/straightlines-sample.cbor`                                          | **New**, generated from the `.svg`, committed                                                                                                         |
| `deno.json`                                                                         | Version bump `0.1.0` → `0.1.1` (human-directed, §1.3); optional one-line `fixtures:regen` task (shell redirect; Linux-only like the rest of the repo) |
| `scripts/test-wasm.ts`                                                              | Fixture SVG added to `SVGS`                                                                                                                           |
| `scripts/test-wasm-browser.ts`                                                      | Same addition (list is intentionally duplicated)                                                                                                      |
| `tests/conformance.test.ts`                                                         | Fixture-based tests (section 5.4)                                                                                                                     |
| `assets/svg2usvg_bg.wasm`, `assets/usvg2rgba_bg.wasm`, `src/usvg.ts`, `src/rgba.ts` | Regenerated via `deno task build` (never hand-edited)                                                                                                 |
| `CHANGELOG.md`                                                                      | Existing entries re-headed `## 0.1.0` (published); new entry under a fresh `Unreleased`                                                               |
| `task.md`                                                                           | New section 13 (proposed separately)                                                                                                                  |

Not touched: `AGENTS.md`, the three other governing docs, `src/mod.ts`,
`README.md` (its stated hard constraints are unaffected; factual accuracy
re-checked during impl), anything under `notes/`.

Both Wasm artifacts are affected (both link `intermediate`); neither bundles the
other (§3.9 preserved — no new crate, no new subpath).

Dependencies: **none added, none changed.** Workspace pins stay
`cbor-core
0.10.1`, `usvg 0.47.0`, `resvg 0.47.0`, default features off.

---

## 5. Tests (all four layers, playbook §3)

### 5.1 `crates/intermediate/tests/core.rs`

- Round-trip: nested groups (≥ 3 levels), mixed shape/group children, strokes
  with non-default cap/join/dash — `decode(encode(x)) == x` (structural
  `PartialEq`).
- Rejections (error, not panic): stroke width `0` / negative / NaN; miterlimit <
  1; opacity out of [0,1] (both kinds); unknown `linecap` / `linejoin` tag;
  unknown `t` discriminator; NaN/∞ in dasharray or dashoffset; missing `stroke`
  / `fill_opacity` / `children` keys; group nesting deeper than the depth limit;
  legacy payload without `root` (documents 1.3).

### 5.2 `crates/svg2usvg/src/lib.rs` (native producer tests)

- Fixture structural test: parse `tests/fixtures/straightlines-sample.svg` →
  `svg()` → decode → assert 11 shapes total under 4 child groups of root;
  per-group stroke colors `#888/#000/#333/#fff`, widths `31/1/3/5`; Layer3 paths
  carry stroke-opacity `0.9`; Layer2 group opacity `0.8`; fills present (usvg
  default black) with `fill_opacity` 1.
- Determinism re-run including the fixture (two consecutive calls,
  byte-identical).

### 5.3 `crates/usvg2rgba/tests/core.rs` (golden, playbook §3.5 — unit test, no human review needed)

- **No-loss proof**: reference-render the original fixture SVG directly
  (`usvg::Tree::from_str` + `resvg::render`, both already dependencies) at
  natural 31×31; then `rasterize(svg-produced bytes)` through the full DTO
  pipeline; assert the RGBA buffers are byte-identical. If byte equality fails
  on anti-aliasing rounding, stop and escalate per playbook §3.1/`AGENTS.md` §10
  rather than silently loosening to a tolerance.
- Existing helpers (`red_rect_cbor`, `natural_cbor`, hand-assembled malformed
  payloads) updated to the new schema; rejection cases extended for stroke
  fields.

### 5.4 Deno layer (`tests/conformance.test.ts`)

- Read committed `tests/fixtures/straightlines-sample.cbor` via `Deno.readFile`
  → `usvg2rgba` → natural 31×31, `pixels.length === 31*31*4`, `alphaMode`
  "straight", non-zero alpha present (white X and gray band visible).
- `svg2usvg(fixtureSvgString)` is **byte-identical to the committed `.cbor`** —
  pins canonical-CBOR output stability as a cross-machine regression net.
- `usvg2rgba(cborFromDisk)` equals `usvg2rgba(await svg2usvg(svg))`
  pixel-for-pixel.

### 5.5 Wasm layers

Add the fixture to `SVGS` in `scripts/test-wasm.ts` and
`scripts/test-wasm-browser.ts`: the existing wasm-vs-native byte-identity
harnesses then exercise the new DTO through both Wasm artifacts in both Deno and
headless Chrome, with no harness logic changes.

---

## 6. Fixture generation

```
cargo run --quiet --release -p svg2usvg --example dump_core \
  < tests/fixtures/straightlines-sample.svg \
  > tests/fixtures/straightlines-sample.cbor
```

Native core output is byte-identical to Wasm output (already asserted by the
harnesses), so generating via the example binary is equivalent to generating via
the package API. Proposed convenience task in `deno.json`:

```json
"fixtures:regen":
  "cargo run --quiet --release -p svg2usvg --example dump_core < tests/fixtures/straightlines-sample.svg > tests/fixtures/straightlines-sample.cbor"
```

---

## 7. Build and size impact

`deno task build` regenerates both Wasm binaries and both TS wrappers; the
committed artifacts and glue are updated in the same change set (playbook §7.1).
Expected size impact: growth in **both** artifacts — new DTO fields, enums,
validation, and SVG-emission strings; estimated tens of KB uncompressed per
artifact (well under 5% of the current 642K / 1.1M). Actual before/after sizes
are measured (`ls -l assets/`) and reported.

---

## 8. Commit strategy (one logical change per commit, `AGENTS.md` §7)

Proposed splits — messages subject to human approval at each step:

1. `feat(intermediate): add groups and strokes to the v1 DTO` — lib.rs +
   intermediate tests.
2. `feat(wasm): preserve layers and strokes end-to-end` — from_tree/to_tree
   fallout in producer/consumer tests, fixture generation + committed `.cbor`,
   golden pixel-equality test.
3. `test: cover the straightlines fixture across all layers` — Deno tests + both
   Wasm harness `SVGS` additions.
4. `chore(build): regenerate wasm artifacts and ts glue` — assets/, src/,
   recorded size delta.
5. `chore(task): mark §13 complete` + CHANGELOG entry.

Each commit passes `cargo fmt --check`, `deno fmt --check`, `deno task lint`,
`deno task check`; the final state passes `deno task test` (all four layers) and
`scripts/check-cdn-free.ts` via the build.

Nothing is pushed or published; JSR publishing remains the human's action.
