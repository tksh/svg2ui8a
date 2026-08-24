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
