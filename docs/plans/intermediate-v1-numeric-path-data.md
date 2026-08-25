# Intermediate v1: numeric `path_data` encoding

> Status: DRAFT — not approved. No source or documentation files have been
> touched. Requires explicit human sign-off before implementation, per
> `docs/engineering-playbook.md` §4 step 3.
>
> Builds on top of the `0.2.0` rename (`svg2usvg`/`usvg2rgba` →
> `svg2stln`/`stln2rgba`, format identifier `"svg2ui8a/straightlines"`) and the
> shape-rendering extension, both shipped. This plan assumes that baseline and
> does not re-derive it.
>
> **Scope note:** this plan covers `path_data`'s encoding only. It does not
> cover documenting the meaning of the DTO's other integer-coded fields (`t`,
> `paint` variant tags, `linecap`/`linejoin`, `shape_rendering`, etc.). That is
> deliberately deferred to a separate future plan and is out of scope here — do
> not fold it into this change.

## 1. Problem statement

`intermediate` v1's `path_data` field is currently a CBOR byte string containing
the ASCII SVG mini-language text produced by formatting the parsed path back out
(e.g. `"M 15 0 L 15 31 "`, including a trailing space per segment). Decoding a
payload therefore requires re-parsing SVG path syntax on every `stln2rgba` call
— including cache-hit calls, which is exactly the case the two-stage pipeline
exists to make cheap (`docs/project-constitution.md` §2.2, §8 "optimizes for the
cache-hit case").

This round-trip through SVG syntax is unnecessary: `usvg::Tree` already holds
parsed, structured path segments (numeric move/line commands) before `svg2stln`
formats them back into a string for the DTO, and `stln2rgba` immediately
re-parses that string back into the same kind of structured segments to build
`tiny_skia`/`usvg` geometry. The string hop adds formatting cost on the producer
side and parsing cost on the consumer side for no benefit, and DTO field
encoding is implementation-defined (`docs/project-constitution.md` §2.3/§4), so
this is a supported bugfix- scale change, not a scope change.

## 2. Format and API decisions

The `0.2.0` rename already froze `svg2stln`'s accepted input to the
Straightlines subset: every `Shape` is exactly one straight, two-point line
segment (a single `moveTo` followed by a single `lineTo`; no curves, no
multi-segment paths, no closed subpaths). Given that guarantee, two encoding
shapes are possible for the replacement:

- **Option A — frozen 4-number endpoint form.** Replace `path_data` with a fixed
  numeric field carrying exactly the start and end coordinates (e.g.
  `[x0, y0, x1, y1]` or an equivalent 4-field struct). No command tags, no
  variable-length segment list — the shape is fully determined by the
  Straightlines-subset guarantee already in force.
- **Option B — general tagged-segment array.** Replace `path_data` with a
  variable-length array of `[cmd_tag, x, y]` entries (or similar), still
  numeric, but structurally able to represent more than one segment if the
  Straightlines subset is ever relaxed later.

**Recommended default: Option A.** It is smaller, simpler, and matches exactly
what `svg2stln` can ever legally produce today; Option B carries generality that
has no current use and would need its own validation logic to reject the shapes
it structurally allows but the subset forbids (e.g. a 3-segment array). If a
future subset relaxation needs more than one segment per shape, that is itself a
subset change requiring its own plan and its own freeze revision — this plan
should not pre-build for it. The implementer records the final choice and a
short rationale in the plan, per `docs/engineering-playbook.md` §0, and confirms
with the human before implementing if Option B is preferred instead.

Whichever option is chosen:

- Coordinates are encoded as numbers (the implementer chooses `f32` vs `f64`;
  the existing DTO already uses floating-point for `opacity`, `stroke.width`,
  etc., so match that precision choice unless a reason to diverge is recorded).
- `svg2stln` no longer formats a path string at all; it reads the two endpoint
  coordinates directly from the parsed `usvg::Tree` path segments.
- `stln2rgba` no longer parses any string for geometry; it builds the
  `tiny_skia`/`usvg` path directly from the numeric endpoints.

## 3. Architecture changes

- `intermediate`: change the `Shape` DTO's `path_data` field type from a
  byte-string/string to the chosen numeric form; update canonical-CBOR
  encode/decode and semantic validation accordingly (reject malformed or
  wrong-arity numeric data the same way the current code rejects malformed
  strings).
- `svg2stln` (`from_tree`): replace the path-to-string formatting step with
  direct extraction of the two endpoint coordinates from the already-parsed
  `usvg` path segments.
- `stln2rgba` (`to_tree` / render path): replace the string-to-path parsing step
  with direct construction of the `tiny_skia`/`usvg` path from the two numeric
  endpoints (`PathBuilder::move_to(x0, y0)` / `PathBuilder::line_to(x1, y1)` or
  equivalent).
- No change to `svg2rgba` (the general-purpose one-shot pipeline, if already
  added) — it has no CBOR intermediate and is unaffected by this DTO change.

## 4. Documentation changes

- `docs/system-architecture.md` §4: update the DTO field description for
  `path_data` (or its renamed field) to reflect the numeric form and the chosen
  Option (A or B).
- `docs/project-constitution.md`: no change expected — the envelope and the
  "package-owned DTO, not a `usvg::Tree` serialization" constraint (§2.3) are
  unaffected; only an internal field's encoding changes.
- `CHANGELOG.md`: entry under "Unreleased" (or directly under `0.2.1` at release
  time) describing the encoding change and its rationale (eliminates a
  string-format round trip on every `stln2rgba` call, including cache hits).

## 5. Test additions / updates

- `crates/intermediate`: update the round-trip test for the new numeric
  `path_data` representation; add a rejection test for malformed numeric data
  (wrong arity, non-finite values).
- `crates/svg2stln` (native): update existing tests that assert on `path_data`
  content to check numeric endpoints instead of a formatted string; confirm the
  existing "same SVG string always produces the same bytes" determinism test
  still passes under the new encoding.
- `crates/stln2rgba` (native): confirm the existing golden pixel-equality tests
  (against reference `resvg` renders, including the `crispEdges` case added by
  the shape-rendering plan) still pass byte-identically under the new decode
  path — this is the key regression check, since the visual output must be
  unaffected by an internal encoding change.
- Regenerate `tests/fixtures/straightlines-sample.cbor` via the existing
  `fixtures:regen` task; diff-review that the only change is the `path_data`
  representation (all coordinate values match the ASCII string values already
  verified against the source SVG).
- Deno layer: existing `.cbor` read-back and end-to-end tests should need no
  behavioral changes, only re-running against the regenerated fixture.

## 6. Verification

- `cargo test` across `intermediate`, `svg2stln`, `stln2rgba`;
  `deno task test:wasm`; `deno task test`.
- Confirm the regenerated fixture's decoded pixel output is byte-identical to
  the pre-change golden reference (this is the primary correctness bar — visual
  output must not move).
- Record the `path_data` size delta per shape (numeric encoding vs. the current
  ASCII string) and the resulting `assets/*.wasm` size delta, per
  `docs/engineering-playbook.md` §4 step "state the expected size impact."
- `cargo fmt --check`, `deno fmt --check`, `deno task lint`, `deno task check`.

## 7. Versioning

Target release: **`0.2.1`**, as decided by the human.

One point to surface, not to block on: `docs/project-constitution.md` §7 defines
a patch as preserving "output bytes for any input," and this change does alter
the CBOR byte layout of `path_data` for every input (the visual RGBA output is
unchanged, but the `.cbor` bytes themselves are not byte-identical to what
`0.2.0` would have produced for the same SVG). This is a stricter reading than a
pure patch under the constitution's own definition. Per the same reasoning
already applied to the `0.2.0` rename (no confirmed external consumer of
previously-cached `.cbor` payloads), this plan proceeds with `0.2.1` as
directed, but the human should confirm this reading is acceptable — if an
external consumer of cached `0.2.0` payloads is later confirmed to exist, this
would need to be revisited as a minor bump instead, per §7's own rule that an
incompatible schema change is not a patch.

`FORMAT_VERSION` stays `1`; only the internal encoding of one DTO field changes.

## 8. Open questions for the human

1. Option A (frozen 4-number endpoints) or Option B (general tagged-segment
   array)? Recommendation: Option A.
2. `f32` or `f64` for the coordinate values?
3. Confirm `0.2.1` (patch) is acceptable given the byte-layout caveat in §7, or
   should this ship as `0.2.1`-patch-in-name-only with the caveat simply
   recorded in `CHANGELOG.md`, or as a minor bump instead?

## 9. Assumptions

- No confirmed external consumer depends on the exact `.cbor` byte layout
  produced by `0.2.0` — consistent with the assumption already used for the
  `0.2.0` rename plan.
- The Straightlines subset (exactly one straight two-point segment per shape) is
  stable as of `0.2.0` and is not being revisited by this plan; if it changes
  later, this plan's Option A choice would need reconsidering at that time, not
  now.
- This plan does not add, remove, or reinterpret any other DTO field (`t`,
  `paint`, `linecap`, `linejoin`, `dasharray`, `shape_rendering`, etc.) and does
  not add documentation for their integer meanings — both are explicitly out of
  scope, per the scope note at the top of this document.
