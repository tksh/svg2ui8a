/**
 * `@tksh/svg2ui8a`: turns SVG strings into `Uint8Array` payloads.
 *
 * Re-exports the two public functions and their supporting types. Import the
 * subpaths `@tksh/svg2ui8a/svg2usvg` or `@tksh/svg2ui8a/svg2rgba` to load
 * only the capability you need.
 *
 * @module
 */
export { svg2rgba } from "./svg2rgba.ts";
export type { RgbaResult, Svg2RgbaOptions } from "./svg2rgba.ts";
export { svg2usvg } from "./svg2usvg.ts";
