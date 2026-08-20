# Unreleased

- Initial baseline implementation.
- TypeScript surface (§6): generated `usvg`/`rgba` wrappers with a single
  per-module `initialized` flag and a hand-written root re-export; the public
  surface is function-only (no class API, no `dispose`).
