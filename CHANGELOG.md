# Unreleased

- Nothing yet. The `0.1.1` entry below ships in this checkout's `deno.json` and
  awaits the human's `deno publish`.

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
