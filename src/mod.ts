/**
 * `@tksh/svg2ui8a`: turns SVG strings into `Uint8Array` payloads.
 *
 * Re-exports the two public functions and their supporting types. Import the
 * subpaths `@tksh/svg2ui8a/svg2usvg` or `@tksh/svg2ui8a/svg2rgba` to load
 * only the capability you need.
 *
 * @module
 */
import { initSvg2rgba } from "./svg2rgba.ts";
import { initSvg2usvg } from "./svg2usvg.ts";

export { initSvg2rgba } from "./svg2rgba.ts";
export { svg2rgba } from "./svg2rgba.ts";
export type {
  RectF,
  RgbaResult,
  Svg2rgbaInitOptions,
  Svg2RgbaOptions,
} from "./svg2rgba.ts";
export { initSvg2usvg } from "./svg2usvg.ts";
export { svg2usvg } from "./svg2usvg.ts";
export type { Svg2usvgInitOptions } from "./svg2usvg.ts";

/**
 * Initializes the Wasm modules of both capabilities.
 *
 * Each capability loads its own binary; a URL that is omitted falls back to
 * the copy shipped with the package. Both binaries are loaded by this call,
 * so applications that use only one capability should prefer the
 * capability-specific `initSvg2usvg` / `initSvg2rgba` functions.
 *
 * @param options Optional Wasm location overrides, per capability.
 * @returns A promise that resolves once both Wasm modules are ready.
 * @throws If a binary cannot be fetched, or if a URL is passed after the
 * corresponding module has already been initialized.
 *
 * @example Point both capabilities at self-hosted binaries
 * ```ts
 * import { initSvg2ui8a } from "@tksh/svg2ui8a";
 *
 * await initSvg2ui8a({
 *   svg2usvgWasmUrl: "/wasm/svg2usvg_bg.wasm",
 *   svg2rgbaWasmUrl: "/wasm/svg2rgba_bg.wasm",
 * });
 * ```
 */
export async function initSvg2ui8a(options?: {
  /** Location of the `svg2usvg_bg.wasm` binary. */
  svg2usvgWasmUrl?: string | URL;
  /** Location of the `svg2rgba_bg.wasm` binary. */
  svg2rgbaWasmUrl?: string | URL;
}): Promise<void> {
  await Promise.all([
    initSvg2usvg(
      options?.svg2usvgWasmUrl === undefined
        ? undefined
        : { wasmUrl: options.svg2usvgWasmUrl },
    ),
    initSvg2rgba(
      options?.svg2rgbaWasmUrl === undefined
        ? undefined
        : { wasmUrl: options.svg2rgbaWasmUrl },
    ),
  ]);
}
