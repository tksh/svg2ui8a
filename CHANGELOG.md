# Unreleased

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
