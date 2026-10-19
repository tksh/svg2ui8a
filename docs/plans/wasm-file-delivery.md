# Task List: Deliver Wasm as `.wasm` files

Source plan:
`svg2ui8a-repos/svg2ui8a-memo/2026-10-20/wasm-file-delivery-by-beam.md`

## Library tasks

- [x] Update build script to stop Base64 embedding
- [x] Generate glue accepting URL/Response/bytes
- [x] Default URL via `new URL("./assets/<file>.wasm", import.meta.url)`
- [x] Add `initSvg2ui8a({ svg2usvgWasmUrl, svg2rgbaWasmUrl })`
- [x] Cache initialization
- [x] Keep lazy init
- [x] Include assets in deno.json publish
- [x] Update CDN-free check
- [x] Update tests for URL loading
- [x] Add Deno server-side test
- [x] Update README and CHANGELOG

## pfpg migration tasks

- [ ] Copy `.wasm` to static output or configure emission
- [ ] Initialize with explicit URL if needed
- [ ] Deno Deploy smoke test of `/ogp/render`

## Validation

- [x] Build produces assets, no WASM_BASE64
- [x] Deno tests pass
- [x] Browser tests pass
- [ ] Deno Deploy smoke test passes
- [x] CDN-free check passes

## Notes

### Deviations from the plan

- **Default URL depth.** The plan spells the default as
  `new URL("./assets/<file>.wasm", import.meta.url)`. The published package
  keeps the module in `src/` and the binaries in `assets/` at the package root
  (that layout is fixed by `docs/system-architecture.md`), so the relative path
  from the module to the binary is `../assets/<file>.wasm`. With the plan's
  spelling, the default would resolve to `src/assets/…` and 404 on JSR. The
  web-standard shape (`new URL(..., import.meta.url)`) is unchanged.
- **One init function per capability, plus the combined one.** The plan names
  `initSvg2ui8a({ svg2usvgWasmUrl, svg2rgbaWasmUrl })`. A consumer that only
  uses one capability imports only that capability's subpath, so the init
  function has to be reachable from there: `initSvg2usvg({ wasmUrl })` and
  `initSvg2rgba({ wasmUrl })` are exported from the two capability subpaths, and
  `initSvg2ui8a({ svg2usvgWasmUrl, svg2rgbaWasmUrl })` is exported from the root
  subpath and initializes both.
- **Bytes, not a stream.** The binary is fetched and passed to `__wbg_init` as
  bytes, not compiled from a `Response` stream:
  `WebAssembly.instantiateStreaming` requires the `application/wasm` content
  type, and the hosting server is outside this package's control.
- **No `deno.json` publish change.** `assets/*.wasm` was already part of the
  published package; verified with `deno publish --dry-run` (the binaries are
  listed among the published files). No configuration change was needed.

### Deno loading

The default location of a binary is a URL, so a Deno consumer of the published
package fetches it from the registry on first use and needs `--allow-net` (or
`--allow-net=jsr.io`). Deno's native static `.wasm` import was evaluated as the
alternative: it works only with the wasm-bindgen "bundler" layout (the binary
imports a sibling `_bg.js`), and `deno bundle` inlines a statically imported
`.wasm` into the browser bundle as base64 — which is exactly what this plan
removes. Serving the binary from a separate URL keeps one loading mechanism for
browsers and Deno.

### Not done here

The pfpg migration tasks and the Deno Deploy smoke test need a published release
of this package: pfpg consumes `jsr:@tksh/svg2ui8a@^0.5.0/svg2rgba`, and `0.5.0`
still embeds the binary. Publishing is done by the human
(`deno publish --set-version` on a release tag), so the tasks stay open until
the new version is on JSR.
