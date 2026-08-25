# Intermediate v1 extension: `shape-rendering` (root-level rendering hint)

> Status: DRAFT — not approved. No source or documentation files have been
> touched. Requires explicit human sign-off before implementation, per
> `docs/engineering-playbook.md` §4 step 3.
>
> Sequencing: this plan **must land before**
> `docs/plans/svg2rgba-pipeline-and-straightlines-rename.md`. That plan's Phase
> 2 formally freezes "whatever `intermediate` v1 already implements" as the
> permanent Straightlines subset boundary. `shape-rendering` is part of the
> Straightlines spec but is currently missing from both the test fixture and the
> DTO — freezing the subset before fixing this gap would lock in an incomplete
> definition and require re-opening the freeze later. This plan closes the gap
> first so Phase 2 documents the real, complete subset.

## 1. Problem statement

The Straightlines spec requires every document to declare rendering precision at
the `<svg>` root via the standard SVG presentation attribute:

```xml
<svg ... shape-rendering="geometricPrecision">
```

with exactly two permitted values: `geometricPrecision` or `crispEdges`. The
current `tests/fixtures/straightlines-sample.svg` (added under item 13) omits
this attribute entirely, and `intermediate` v1's DTO has no field for it — so
the CBOR payload in the committed fixture does not reflect a spec-compliant
input, and `svg2usvg` currently accepts SVG that the Straightlines spec would
consider incomplete. This plan fixes both.

## 2. Format and API decisions

- Add a single **root-level** field to the DTO — `shape-rendering` is declared
  once per document in Straightlines (unlike `stroke`, which is per-shape), so
  it belongs alongside the existing top-level `size` field in the payload map,
  not inside `Node`/`Group`/`Shape`.
- Represent it as a two-value enum (`GeometricPrecision` | `CrispEdges`) in the
  Rust DTO, encoded compactly in CBOR (implementer's choice of representation,
  per `project-constitution.md` §2.3/§4 — field encoding is
  implementation-defined).
- `svg2usvg` **requires** the attribute on the SVG root and rejects input where
  it is absent or set to any value other than the two permitted ones (including
  the SVG-spec defaults `auto`/`optimizeSpeed`). This extends the existing
  reject-don't-guess posture already used for `<text>`/`<image>`
  (`project-constitution.md` §3.1/§3.3) rather than silently defaulting.
- **Backward-decode question for the human (Phase 0 of this plan):** the fixture
  and any hypothetical external `0.1.0` payloads were encoded without this
  field. Two options: a) `usvg2rgba` treats a missing field as a decode error
  (strict — no format-version bump needed since it's a stricter read of the same
  v1 schema; existing `0.1.0` payloads become non-decodable), or b) the field is
  optional-with-default on decode for backward compatibility (which value is the
  default, if so?), while `svg2usvg` still always emits it (and always requires
  it in the source SVG) going forward. This plan defaults to **(a)** unless the
  human overrides in review, since `project-constitution.md` §5.3 already treats
  semantic validation as mandatory and no external `0.1.0` consumers have been
  confirmed to exist (see the version-policy decision recorded in the rename
  plan).

## 3. Architecture changes

- `intermediate`: add the `ShapeRendering` enum and the root-level DTO field;
  extend canonical-CBOR encode/decode and semantic validation (reject anything
  other than the two permitted values); extend the round-trip test.
- `svg2usvg` (`from_tree`): extract the SVG root's `shape-rendering` via
  `usvg`'s existing parse of the attribute (verify during implementation whether
  `usvg::Tree`/`usvg::Node` already exposes this per the upstream SVG spec
  support — if so, this is a mapping addition, not new parsing logic) and reject
  unsupported values or absence.
- `usvg2rgba` (`to_tree` / render call): verify whether the current `usvg::Tree`
  reconstruction path already threads the equivalent property through to
  `resvg`'s rendering, and, if not, wire the DTO value into the render call so
  `crispEdges` renders without anti-aliasing and `geometricPrecision` renders
  with `resvg`'s normal anti-aliased path. This determines whether the fix is
  confined to `intermediate`'s DTO/mapping layer or also touches the `usvg2rgba`
  render call — implementer confirms and records which during the plan's
  implementation step.

## 4. Documentation changes

- `tests/fixtures/straightlines-sample.svg`: add
  `shape-rendering="geometricPrecision"` to the root `<svg>` element.
- Regenerate `tests/fixtures/straightlines-sample.cbor` via the existing
  `fixtures:regen` task.
- Update `docs/system-architecture.md` §4 (DTO field list) and
  `docs/project-constitution.md` §2.3 (DTO field summary, if it enumerates
  fields there) to mention the new root-level field.
- Update `docs/engineering-playbook.md` §3.1 required-test list to include the
  new rejection cases (missing attribute, unsupported value).
- `docs/straightlines-vision.md`: no change expected — this is a
  spec-conformance fix, not a scope change to the vision document.

## 5. Test additions

- `crates/svg2usvg` (native): rejects SVG missing `shape-rendering`; rejects SVG
  with `shape-rendering="auto"` (or any value outside the two permitted);
  accepts and round-trips both `geometricPrecision` and `crispEdges`.
- `crates/intermediate`: round-trip test for both enum values; envelope/
  validation test for the new field's presence requirement.
- `crates/usvg2rgba` (native): golden pixel-equality test comparing `crispEdges`
  output against a reference `resvg` render at `shape-rendering="crispEdges"`,
  alongside the existing `geometricPrecision` golden test (which the regenerated
  fixture already exercises).
- Deno layer: `.cbor` read-back test using the regenerated fixture, asserting
  the decoded `RgbaResult` matches the `crispEdges` vs `geometricPrecision`
  golden references as applicable.
- Update `SVGS` entries in `scripts/test-wasm.ts` and
  `scripts/test-wasm-browser.ts` if they enumerate expected fixture properties.

## 6. Verification

- `cargo test` across `intermediate`, `svg2usvg`, `usvg2rgba`;
  `deno task
  test:wasm`; `deno task test`.
- Confirm the regenerated `straightlines-sample.cbor` differs from the committed
  one only by the addition of the new field (diff review).
- Confirm `deno task build` output artifacts change only as expected
  (`assets/*.wasm` size delta recorded, per `engineering-playbook.md` §4 step
  6).
- `cargo fmt --check`, `deno fmt --check`, `deno task lint`,
  `deno task
  check`.

## 7. Versioning

Additive DTO field, still under `intermediate::FORMAT_VERSION = 1`, matching the
pattern already used for item 13's `Stroke`/`fill_opacity` addition. Per
`project-constitution.md` §7 this is a **minor** addition to the API/format
surface if decode option (b) in §2 is chosen (old payloads still decode); it
becomes a **breaking** change requiring the same version-policy decision as the
rename plan if option (a) is chosen instead. Target release: the next
`0.1.x`/`0.2.0` slot, decided alongside the rename plan's Phase 0 once both
plans are approved — see that plan's updated version note.

## 8. Open questions for the human

1. Confirm option (a) or (b) in §2 (strict-reject vs. default-on-decode for the
   missing field on old payloads).
2. Confirm no external `0.1.0` consumer exists that would be broken by option
   (a) — same question already raised in the rename plan.
3. Should this ship as its own release before the rename plan's `0.2.0`, or be
   folded into the same release? (Recommendation: ship first, as a `0.1.1`/minor
   addition, so the rename plan's Phase 2 freeze documents a complete subset
   from the start — see that plan's updated sequencing.)

## 9. Assumptions

- `usvg` already parses the `shape-rendering` presentation attribute upstream
  (it is a standard SVG property), so this plan is a DTO/mapping addition plus
  stricter validation, not new SVG-parsing logic. The implementer confirms this
  during Phase-3-equivalent implementation and escalates per `AGENTS.md` §10 if
  the assumption does not hold.
- This is scoped strictly to the root-level `shape-rendering` attribute as used
  in the Straightlines spec (one value per document). Per-element
  `shape-rendering` overrides, if any exist in future SVG inputs, are out of
  scope and should be rejected the same way other unsupported attributes are,
  consistent with the existing reject-don't-guess posture.
