# Task List: Deliver Wasm as `.wasm` files

Source plan: `svg2ui8a-repos/svg2ui8a-memo/2026-10-20/wasm-file-delivery-by-beam.md`

## Library tasks
- [ ] Update build script to stop Base64 embedding
- [ ] Generate glue accepting URL/Response/bytes
- [ ] Default URL via `new URL("./assets/<file>.wasm", import.meta.url)`
- [ ] Add `initSvg2ui8a({ svg2usvgWasmUrl, svg2rgbaWasmUrl })`
- [ ] Cache initialization
- [ ] Keep lazy init
- [ ] Include assets in deno.json publish
- [ ] Update CDN-free check
- [ ] Update tests for URL loading
- [ ] Add Deno server-side test
- [ ] Update README and CHANGELOG

## pfpg migration tasks
- [ ] Copy `.wasm` to static output or configure emission
- [ ] Initialize with explicit URL if needed
- [ ] Deno Deploy smoke test of `/ogp/render`

## Validation
- [ ] Build produces assets, no WASM_BASE64
- [ ] Deno tests pass
- [ ] Browser tests pass
- [ ] Deno Deploy smoke test passes
- [ ] CDN-free check passes
