# Add `svg2rgba` (general-purpose) and narrow the existing pipeline into

# `svg2stln` / `stln2rgba` (Straightlines-only)

> Status: APPROVED & IMPLEMENTED (Phases 2–4). Phase 0 answers recorded: (1)
> §3.9 amendment approved; (2) names as drafted; (3) identifier hard-cut to
> `"svg2ui8a/straightlines"` with no legacy decode; (4) `0.1.1` published
> manually first; this work targets `0.2.0`. Phase 1 (numeric `path_data`) was
> **dropped** during review — see CHANGELOG `0.2.0` for the recorded rationale.
>
> Builds on top of item 13 (`docs/plans/intermediate-v1-groups-strokes.md`),
> which is complete: the package is published as `0.1.0`, the hierarchical
> `Node`/`Group` DTO with `Stroke` is implemented, the
> `tests/fixtures/straightlines-sample.{svg,cbor}` fixture exists and passes a
> golden pixel-equality test against a reference `resvg` render, and the
> work-in-progress version target is `0.1.1`. This plan assumes that baseline
> and does not re-derive it.
>
> **Prerequisite:** `docs/plans/intermediate-v1-shape-rendering.md` must land
> first. It closes a spec-conformance gap (the Straightlines-mandated root-level
> `shape-rendering` attribute is currently missing from both the test fixture
> and the DTO) that this plan's Phase 2 needs fixed _before_ it formally freezes
> "whatever `intermediate` v1 already implements" as the permanent Straightlines
> subset — freezing an incomplete subset now would mean re-opening the freeze
> later. See §3 Phase 2 below for the exact ordering.

## 1. Problem statement

The package name `svg2ui8a` and the README's stated exclusion list (`<text>`,
`<image>`) set a third-party expectation of "arbitrary non-text/non-raster SVG
in, `Uint8Array` out." What is actually implemented and shipped in `0.1.0` is
narrower and, notably, already shaped like Straightlines:
`tests/fixtures/straightlines-sample.svg` — the fixture that exercises the real
pipeline — contains only straight two-point path segments, four sibling
(non-nested) `<g>` layers, stroke-only shapes (no fill), and group/stroke
opacity. That is not an accident of the fixture; it is representative of what
`intermediate` v1 can round-trip today.

A prior discussion in this thread found that pushing the _existing_ pipeline
further toward "arbitrary SVG" (numeric fast-path rasterization bypassing
`usvg::Tree` reconstruction — "Option A/B") would force `usvg2rgba` to
re-implement slices of `resvg`'s rendering pipeline by hand, a commitment that
gets worse once `clip-path`/`filter`/`mask`/`pattern` are ever considered, and
conflicts with `docs/engineering-playbook.md` §7.5/7.6 ("upstream behavior — do
not patch, do not reimplement").

Resolution: stop trying to make the two-stage, cacheable pipeline the
general-purpose entry point. Split it in two:

- **`svg2rgba`** (new) — general-purpose, one-shot. Scope: whatever
  feature-disabled `usvg`/`resvg` support, minus `<text>` and `<image>` — the
  scope the package name and README already imply. No CBOR envelope, no cache
  boundary, no fast-path; it is allowed to be "just" `usvg::Tree` build +
  `resvg::render`.
- **`svg2stln` / `stln2rgba`** (renamed from the existing `svg2usvg` /
  `usvg2rgba`) — the pipeline that is _already implemented_, permanently named
  and scoped for what it already is: the Straightlines subset (straight line
  segments, non-nested groups, stroke and/or fill, group and stroke opacity —
  the exact surface `intermediate` v1 supports today, per item 13). Freezing the
  scope here is what makes future fast-path optimization (deferred to Phase 5
  below) safe to pursue without the scope-creep risk described above.

## 2. What this plan does NOT decide yet

Three items are human-owned and are surfaced in Phase 0 rather than assumed:

1. `docs/project-constitution.md` §3.9 ("two Wasm artifacts, not one") is a hard
   constraint. Adding `svg2rgba` as a third Wasm artifact requires this section
   to be **amended**, not worked around.
2. `docs/project-constitution.md` §1/§2.3 fixes the function names (`svg2usvg`,
   `usvg2rgba`), the subpath names (`./usvg`, `./rgba`), and the file-format
   identifier (`"svg2ui8a/usvg"`). Renaming any of these is a **breaking change
   to a package already published at `0.1.0`**. `project-constitution.md` §7
   defines strict semver by compatibility (majors = incompatible API changes)
   with no stated `0.x` carve-out; common Deno/npm convention instead treats
   `0.x` minor bumps as allowed-breaking. **Human decision (recorded, not
   open):** the human has chosen the second reading — the rename ships as
   `0.2.0`, a minor bump under the "`0.x` minor = allowed-breaking" convention,
   not a major version. This overrides a literal reading of §7 for this change;
   `docs/project-constitution.md` §7 itself is not being amended by this
   decision, only applied under the `0.x` convention for this pre-1.0 package.
   Record this explicitly in `CHANGELOG.md` when Phase 2 ships, so the reasoning
   is not silently assumed by future readers.
3. Whether format-version-1 payloads under the old identifier
   (`"svg2ui8a/usvg"`) must remain decodable under the new function names, or
   whether the identifier changes outright (`intermediate`'s `FORMAT_VERSION`
   stays `1`; only the string identifier and the crate/ subpath names change —
   these are independent axes and Phase 0 should answer both).
4. Whether the currently-in-flight `0.1.1` work (item 13's remaining
   follow-through, if any) should land first, or whether this plan's Phase 2
   renaming folds into the same `0.1.1`/next release instead of shipping twice.

## 3. Phased plan

### Phase 0 — Decision record (no files touched)

Get explicit human answers to items 1, 2, and 4 in §2. Item 3 (version number)
is **already decided**: the rename ships as `0.2.0`, per the human decision
recorded in §2 item 2 above.

**Exit criterion:** written answers to the remaining open items. Nothing in
Phase 2 or 3 starts until then. `docs/plans/intermediate-v1-shape-rendering.md`
does not depend on this phase and may proceed independently and in parallel (see
the prerequisite note at the top of this document).

### Phase 1 — Numeric `path_data` encoding (independent, low-risk, ship first)

Still applicable: the shipped `0.1.0` fixture confirms `path_data` is currently
encoded as an SVG mini-language string (`'M 15 0 L 15 31 '`) inside the CBOR
DTO, not as numeric segments. This is decoupled from the naming/scope question
in Phases 0–3 and does not require touching `project-constitution.md` (DTO field
encoding is implementation-defined per §2.3/§4). Recommend shipping this
independently, before or alongside Phase 2:

- Change `path_data` from a string to numeric segments (e.g.
  `[cmd_tag, x, y, ...]`) decoded straight into `tiny_skia::PathBuilder` calls,
  removing the SVG-syntax round-trip at the DTO boundary.
- Update the round-trip test (`docs/engineering-playbook.md` §3.1) and the
  hand-built payload helpers already added for item 13's stroke rejection cases.
- Regenerate `tests/fixtures/straightlines-sample.cbor` via the existing
  `fixtures:regen` task; re-run the golden pixel-equality test to confirm the
  numeric encoding renders byte-identical to the current string-based one.
- If Phase 0 has already been answered by this point, fold the resulting version
  bump into Phase 2's release instead of shipping it alone.

### Phase 2 — Rename and permanently freeze the scope of the existing pipeline

Gated on Phase 0 approval **and** on
`docs/plans/intermediate-v1-shape-rendering.md` having already shipped. Do not
start step 3 below (formalizing the Straightlines subset definition) until the
shape-rendering fixture, DTO field, and rejection tests from that plan are in
place — otherwise the subset this step documents omits a spec-mandated attribute
and would need revisiting immediately after.

1. Rename `crates/svg2usvg` → `crates/svg2stln`, `crates/usvg2rgba` →
   `crates/stln2rgba`. Update `Cargo.toml` package names, `jsr.json` exports,
   and regenerate `src/usvg.ts`/`src/rgba.ts` → `src/stln.ts`/`src/stln-rgba.ts`
   from template (agent never hand-edits these, per `engineering-playbook.md`
   §2.3).
2. Change the format identifier constant per the Phase 0 decision (either
   `"svg2ui8a/straightlines"` with `FORMAT_VERSION` restarting at `1`, or a
   dual-decode legacy path for `"svg2ui8a/usvg"` — whichever Phase 0 chose).
3. Formally document the **Straightlines subset** that `svg2stln` accepts, as a
   codification of what `intermediate` v1 already implements per item 13, not a
   new restriction:
   - Path data: straight two-point segments only (already the only shape
     `svg2usvg` emits correctly today — curves are untested and unspecified, not
     merely "rare").
   - Groups: flat, non-nested (`Layer0`–`Layer3` in the fixture are all
     siblings; nested-group behavior is currently unspecified).
   - Paint: fill and/or stroke, with fill/stroke opacity and group opacity — the
     fields the `Stroke` DTO and `fill_opacity` already carry.
   - Rendering precision: a required root-level `shape-rendering` value
     (`geometricPrecision` or `crispEdges`), per
     `docs/plans/intermediate-v1-shape-rendering.md` (prerequisite to this
     phase).
   - Explicitly out of scope, not "not yet implemented": gradients, patterns,
     clip-path, mask, filter, curves, arcs.
   - `svg2stln` rejects anything outside this list, extending the existing
     text/image rejection tests (`crates/svg2usvg/tests/`) with rejection cases
     for curves, nested groups, gradients/patterns/clip/mask/filter.
4. Rewrite `straightlines-vision.md` to describe this crate pair as the closest
   practical implementation of the Straightlines primitive subset, referencing
   the now-renamed identifier.
5. Update `project-constitution.md` §1/§2/§3/§4, `system-architecture.md`, and
   `engineering-playbook.md` §3.1 test lists for the new names, the new
   identifier, and the new rejection cases.
6. Update `scripts/test-wasm.ts` / `scripts/test-wasm-browser.ts`'s `SVGS`
   entries and any hardcoded old subpath/crate names.

**Tests added:** rejection tests for each newly-excluded primitive; updated
envelope test for the new identifier; existing round-trip, determinism, and
golden pixel-equality tests carried over unchanged (they already exercise
exactly this subset via the `straightlines-sample` fixture).

### Phase 3 — Add `svg2rgba` (general-purpose, one-shot)

Gated on Phase 0 approval (§3.9 amendment).

1. New crate `crates/svg2rgba/`, new Wasm artifact `assets/svg2rgba_bg.wasm`,
   new subpath `./svg2rgba`.
2. Scope: whatever feature-disabled `usvg` supports, minus `<text>` and
   `<image>` — matching the README's current exclusion list exactly, now that
   `svg2stln` no longer tries to cover this ground.
3. Implementation: parse with `usvg`, render with `resvg`, return RGBA directly.
   No CBOR envelope in or out — there is no round trip to make byte-stable, so
   `crates/intermediate` is not involved and none of the versioning/hashing
   machinery built for the cacheable pipeline applies here.
4. Amend `project-constitution.md` §3.9 per the Phase 0 decision (e.g. "each
   independently-loadable capability is its own Wasm artifact" replacing the
   hardcoded "two").
5. Update `system-architecture.md` §2 (subpath table) and §9 (dependency graph,
   now three branches), `engineering-playbook.md` §2 (three-track build
   pipeline) and §5.2 (CDN-free check now covers three entry points).
6. Add tests across all four layers per `engineering-playbook.md` §4.1 ("adding
   a new subpath export"). Determinism is asserted only as "same input → same
   pixels," not "same input → byte-identical CBOR," since `svg2rgba` makes no
   cacheable-bytes contract.

### Phase 4 — Documentation and identity pass

1. Rewrite `README.md` for three entry points: `svg2rgba` ("one SVG in, pixels
   out, no caching contract") vs. `svg2stln`/`stln2rgba` ("many
   Straightlines-subset SVGs, stable hashable intermediate, cache-friendly").
2. `CHANGELOG.md` entry under "Unreleased" documenting the breaking rename and
   the new subpath, at the version number Phase 0 settled on.
3. Flag (do not decide) whether the package name `svg2ui8a` itself should be
   reconsidered now that it hosts a general-purpose function alongside a narrow
   one — human decision, out of scope here.

### Phase 5 — (later, optional) fast-path optimization inside `svg2stln`/`stln2rgba`

Only after Phase 2 ships and the subset is contractually frozen. The existing
golden pixel-equality test (item 13) already demonstrates that
byte-identical-to-`resvg` output is achievable for the current
tree-reconstruction pipeline on this fixture — useful evidence for, but not a
substitute for, the separate parity-bar question a fast-path rewrite would raise
(a direct-rasterization path bypassing `usvg::Tree` is a different code path and
would need its own byte-parity verification, per the earlier Option A/B
discussion). Revisit that discussion's four open questions (A vs. B, parity bar,
v1/legacy compat, benchmark gate) once Phase 2 has made the input scope small
enough that the scope-creep risk no longer applies.

## 4. Verification checklist (per phase)

- Phase 1: round-trip and golden pixel-equality tests pass with numeric
  `path_data`; `fixtures:regen` output committed; no envelope/identifier change.
- Phase 2: rejection tests for every newly-excluded primitive; envelope test
  updated for the new identifier; existing determinism and golden tests carried
  over and passing; `deno task build` regenerates
  `src/stln.ts`/`src/stln-rgba.ts` cleanly from a clean checkout; `SVGS` fixture
  lists updated in both test-wasm scripts.
- Phase 3: new subpath's four test layers per `engineering-playbook.md`
  §3.1–3.4; CDN-free check covers three bundles; dependency-feature verification
  (§3.6) confirms `svg2rgba` still excludes text/system-font/ raster-image
  features.
- Phase 4: doc review — no stale references to `svg2usvg`/`usvg2rgba`/old
  identifier remain outside a clearly marked legacy note (if Phase 0 chose to
  keep the old identifier decodable).

## 5. Open questions carried into Phase 0

1. Approve the §3.9 amendment to permit three artifacts?
2. Final names — confirm or bikeshed `svg2stln`/`stln2rgba`, `./svg2rgba`.
3. Keep `"svg2ui8a/usvg"` decodable as a legacy identifier, or hard-cut to
   `"svg2ui8a/straightlines"`?
4. Does the in-flight `0.1.1` work (and, ahead of it,
   `docs/plans/intermediate-v1-shape-rendering.md`) need to land and ship before
   this plan's Phase 2, or can they merge into one `0.2.0` release?
   (Recommendation in the shape-rendering plan: ship it first, standalone, so
   Phase 2's subset freeze starts from a complete spec.)
5. Reconsider the package name itself (flagged in Phase 4, not decided here)?

Already decided (not open, see §2 item 2): version number for the rename is
`0.2.0`.

## 6. Assumptions

- Item 13 is complete as described in `task.md` §13: `0.1.0` published,
  `intermediate` v1 hierarchical DTO with `Stroke`/`fill_opacity` shipped,
  `tests/fixtures/straightlines-sample.{svg,cbor}` committed and covered by a
  golden pixel-equality test, `deno task build`/`deno task test` green.
- No implementation work from this plan starts before Phase 0 is answered,
  matching `engineering-playbook.md` §4 step 3.
- `docs/plans/intermediate-v1-shape-rendering.md` ships before this plan's Phase
  2 (see the prerequisite note at the top of this document); Phase 1 and Phase 0
  of this plan may proceed independently of that timing.
- `svg2rgba` does not need a CBOR envelope or `intermediate`-crate integration;
  it is out of scope for the versioned-file-format contract in
  `project-constitution.md` §2.3, which applies only to the cacheable pipeline.
