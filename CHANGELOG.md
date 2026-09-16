# 0.3.6

- **Expose fractional natural SVG dimensions in `RgbaResult`.** Adds
  `naturalWidth` and `naturalHeight` to the JavaScript result so consumers can
  access the original SVG dimensions separately from the integer output pixel
  dimensions.

# 0.3.5

- **Preserve fractional natural SVG dimensions for aspect-ratio sizing.** The
  `svg2rgba` renderer now keeps the natural `usvg` dimensions as floating-point
  values while calculating one-sided output sizes, then rounds only the final
  pixel dimensions.

# 0.3.4

- **Free temporary Wasm `RgbaResult` immediately after copying pixels (no
  API/output change).** `svg2rgba()` now calls `result.free()` in `finally`
  after building the plain object, instead of relying on `FinalizationRegistry`
  timing. Fixes Wasm-memory accumulation over consecutive renders.

# 0.3.3

- **Publishing from GitHub Actions (no runtime or byte-output change).**
  Releases are now published by `.github/workflows/publish.yml` on `v*.*.*` tags
  via `deno publish --set-version`; the version is no longer stored in
  `deno.json`, and the release flow is documented in
  `docs/engineering-playbook.md` §6.

# 0.3.2

- Replace `node:` path and URL imports in development scripts with Deno-native
  URL resolution.

# 0.3.1

- **Documentation: add module and symbol docs to every entrypoint.** Adds the
  JSR-required `@module` docs to `./`, `./svg2rgba`, and `./svg2usvg`, and JSDoc
  for `svg2rgba`, `svg2usvg`, `Svg2RgbaOptions`, and `RgbaResult`. No runtime or
  byte-output change.

# 0.3.0

- **Pivot: remove Straightlines / CBOR intermediate, keep `svg2rgba`, add
  `svg2usvg` (`Uint8Array` of usvg bytes)
  (`0.3.0-pivot-decommission-intermediate-add-svg2usvg.md`).** The custom CBOR
  representation failed to make cache-hit renders cheaper: `usvg::Tree` can only
  be reconstituted by re-parsing SVG XML (`pub(crate)` fields, no public
  constructors, verified against `usvg 0.47.0`), so the intermediate's
  re-serialization round-trip could not bypass the re-parse it was designed to
  avoid. The human has decided to decommission the strategy entirely rather than
  paper over it with numeric encodings.
  - Removed: `crates/intermediate`, `crates/svg2stln`, `crates/stln2rgba`,
    `cbor-core` (`0.10.1`), Straightlines fixtures
    (`tests/fixtures/straightlines-sample.{svg,cbor}`), and their tests. Any
    cached `.cbor` bytes from `0.2.0` are no longer decodable (intentional; no
    external consumer confirmed, per `0.2.0` rationale).
  - Kept: `svg2rgba` with its current functionality (SVG → RGBA `Uint8Array`, no
    intermediate).
  - Added: `svg2usvg` (`crates/svg2usvg`, `src/svg2usvg.ts`,
    `assets/svg2usvg_bg.wasm`) — SVG string in, `Uint8Array` (UTF-8 bytes of the
    normalized usvg XML via default `XmlOptions`, no pretty-print) out. Every
    public payload remains `Uint8Array` (`svg2ui8a` naming); both exports are
    independent Wasm artifacts with independent subpath imports (`./svg2usvg`,
    `./svg2rgba`).
  - Docs: `docs/project-constitution.md`, `docs/system-architecture.md`,
    `docs/engineering-playbook.md`, `AGENTS.md` rewritten for two products and
    no proprietary serialization; fonts/BBox changed from permanently forbidden
    to **postponed** (may become a faithful `linebender/resvg` wrapper later; no
    chase of latest upstream, no package-unique features).
  - Archive: `docs/plans/cbor-file-intermediate.md`,
    `intermediate-v1-groups-strokes.md`, `intermediate-v1-shape-rendering.md`,
    `intermediate-v1-numeric-path-data.md`,
    `svg2rgba-pipeline-and-straightlines-rename.md` moved verbatim to
    `docs/archive/` as a historical record that the strategy did not work out.

# 0.2.0

- **Breaking rename (`svg2rgba-pipeline-and-straightlines-rename.md`).** The
  cacheable two-stage pipeline is now the Straightlines-only pair: `svg2usvg` →
  **`svg2stln`** (subpath `./svg2stln`, file `src/stln.ts`) and `usvg2rgba` →
  **`stln2rgba`** (subpath `./stln2rgba`, file `src/stln-rgba.ts`, options type
  `Stln2RgbaOptions`). Crates renamed to match; Wasm artifacts are
  `assets/svg2stln_bg.wasm` / `assets/stln2rgba_bg.wasm`.
- **Format identifier hard-cut.** Envelope identifier changed from
  `"svg2ui8a/usvg"` to `"svg2ui8a/straightlines"`; `FORMAT_VERSION` stays `1`
  under the new identifier. Old-identifier payloads are no longer decodable —
  recorded here per the human decision that no external `0.1.x` consumers exist.
- **Straightlines subset frozen (Phase 2).** `svg2stln` now rejects, with
  explicit errors, anything beyond the subset v1 actually implements:
  non-two-point or curved paths, closed shapes, nested groups (including usvg's
  element-`opacity` wrapper groups), gradient/pattern paints, and
  clip-path/mask/filter. Layer opacity and stroke/fill opacity remain supported.
  This freeze is what makes the scope safe to optimize later without re-opening
  the contract.
- **New general-purpose one-shot subpath (Phase 3): `svg2rgba(svg, options?)`**
  (`crates/svg2rgba`, `assets/svg2rgba_bg.wasm`, `src/svg2rgba.ts`). SVG string
  in, RGBA pixels out; supports everything feature-disabled `usvg`/`resvg` do
  except `<text>`/`<image>`. No CBOR envelope and no cacheable-bytes contract;
  determinism is same-pixels only. Constitution §3.9 amended from "two Wasm
  artifacts" to one independent artifact per capability.
- **Versioning rationale (recorded per plan §2 item 2).** The rename is a
  breaking change shipped as a **minor** bump `0.1.1 → 0.2.0`: the package is
  pre-1.0 and the human has adopted the common `0.x` convention that minor
  releases may break. Constitution §7's literal "majors for breaking" reading
  applies post-1.0; it was applied under the `0.x` convention here, not amended.
- Phase 1 of the rename plan (numeric `path_data` encoding) was **dropped** by
  decision during Phase 0 review: usvg exposes no programmatic tree
  construction, so reconstruction must format numbers into SVG text anyway;
  numeric storage would move float-formatting onto the consumer hot path the
  constitution optimizes for. Rationale recorded here so the idea is not
  silently re-proposed.

# 0.1.0

- Initial baseline implementation.
- TypeScript surface (§6): generated `usvg`/`rgba` wrappers with a single
  per-module `initialized` flag and a hand-written root re-export; the public
  surface is function-only (no class API, no `dispose`).
- Browser-verified Wasm tests (§8): Astral/CDP headless Chrome `125.0.6400.0`
  via `jsr:@astral/astral@0.5.6` — pinned, hermetically fetched to
  `~/.cache/astral` with no `sudo`/`apt`/`npm`; two-layer verification
  (`test:wasm` Deno + `test:wasm:browser` browser) for the §3.3 guarantees
  (Promise types and byte-identity vs Rust-native `dump_core`) on the shipped
  `src/usvg.ts`/`src/rgba.ts`.
- JSR publish readiness (§12): `deno.json` is the single manifest
  (`name`/`version`/`exports`) with a `publish.exclude` limiting shipped files
  to consumer-facing ones.

# 0.1.1

- **Root-level `shape-rendering` support (§14).** The version-1 DTO gains a
  required root-level `shape_rendering` field (`geometricPrecision` |
  `crispEdges`). `svg2usvg` now rejects SVG documents that do not declare
  `shape-rendering` at the root with one of the two permitted values (`auto` is
  accepted as `geometricPrecision`; per-element overrides and pathless documents
  are rejected). `usvg2rgba` honors the value: `crispEdges` renders without
  anti-aliasing, byte-identical to a direct resvg render of the same input.
  Payloads encoded before this change are rejected by the new decoder.
  `tests/fixtures/straightlines-sample.svg` now declares
  `shape-rendering="geometricPrecision"`; regenerate its `.cbor` via
  `deno task fixtures:regen`. `FORMAT_VERSION` remains `1`; ships in `0.1.1`
  (human-approved).
- **Intermediate v1 extension (§13): layers (`<g>`), strokes, and stroke
  styling.** The version-1 DTO now preserves group nesting with per-group
  compositing opacity, stroke color (`stroke`), width, opacity, line cap, line
  join, miter limit, dash array, and dash offset — all resolved values from
  usvg. Stroke-only paths are no longer dropped: an SVG consisting of stroked
  lines renders exactly as before conversion, verified byte-for-byte against a
  direct render of the original SVG for
  `tests/fixtures/straightlines-sample.svg` (now committed as
  `straightlines-sample.cbor`, regenerable via `deno task fixtures:regen`).
  `Shape.opacity` was renamed to `fill_opacity`. Old pre-extension v1 payloads
  are rejected by the new decoder. `FORMAT_VERSION` remains `1`; package version
  bumped to `0.1.1` (major-zero development; human-approved).
