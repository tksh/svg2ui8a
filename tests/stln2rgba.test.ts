// Deno tests for the runtime shape of stln2rgba's public surface (§3.4 layer).
//
// These guard the §6 fix that made the public API function-only: they verify
// at runtime that `stln2rgba` returns a plain data object (never a
// wasm-bindgen class instance), that results survive reuse of the Wasm
// instance, that options are plain object literals, and that no `__wasm_*`
// glue symbol is reachable from the public subpaths.

import { stln2rgba } from "../src/stln-rgba.ts";
import { svg2stln } from "../src/stln.ts";

const SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"
  shape-rendering="geometricPrecision">
  <path d="M 5 0 L 5 10" stroke="#ff0000" stroke-width="10" fill="none"/>
</svg>`;

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) {
    throw new Error(msg);
  }
}

function checksum(bytes: Uint8Array): number {
  let sum = 0;
  for (let i = 0; i < bytes.length; i++) {
    sum = (sum + bytes[i]) >>> 0;
  }
  return sum >>> 0;
}

Deno.test("stln2rgba result is a plain object, not a wasm-bindgen class instance", async () => {
  const bytes = await svg2stln(SVG);
  const result = await stln2rgba(bytes);

  assert(
    Object.getPrototypeOf(result) === Object.prototype,
    "result must be a plain object literal, not a wasm-bindgen class instance",
  );
  assert(
    typeof (result as { free?: unknown }).free === "undefined",
    "result must not expose a wasm-bindgen free() method",
  );
  assert(
    typeof (result as { __wbg_ptr?: unknown }).__wbg_ptr === "undefined",
    "result must not carry the wasm-bindgen __wbg_ptr handle",
  );
  assert(result.pixels instanceof Uint8Array, "pixels must be a Uint8Array");
  assert(
    Object.getPrototypeOf(result.pixels) === Uint8Array.prototype,
    "pixels must be a plain Uint8Array, not a wasm-bindgen-wrapped subclass",
  );

  let cloned: unknown;
  try {
    cloned = structuredClone(result);
  } catch (e) {
    throw new Error(`structuredClone must succeed on plain data: ${e}`);
  }
  const c = cloned as {
    width: number;
    height: number;
    alphaMode: string;
    pixels: Uint8Array;
  };
  assert(
    c.width === result.width && c.height === result.height &&
      c.alphaMode === result.alphaMode,
    "structuredClone must preserve width/height/alphaMode",
  );
  assert(c.pixels instanceof Uint8Array, "cloned pixels must be a Uint8Array");

  let parsed: { width: number; height: number; alphaMode: string };
  try {
    parsed = JSON.parse(JSON.stringify(result)) as {
      width: number;
      height: number;
      alphaMode: string;
    };
  } catch (e) {
    throw new Error(`JSON.stringify/parse must succeed on plain data: ${e}`);
  }
  assert(
    parsed.width === result.width && parsed.height === result.height &&
      parsed.alphaMode === result.alphaMode,
    "JSON round-trip must preserve width/height/alphaMode",
  );
  assert(
    !("free" in parsed),
    "JSON output must not contain a free key",
  );
});

Deno.test("stln2rgba result stays valid after a second call reuses the wasm instance", async () => {
  const barSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"
      shape-rendering="geometricPrecision">
  <path d="M 2 0 L 2 10" stroke="#ff0000" stroke-width="4" fill="none"/>
</svg>`;
  const bytes = await svg2stln(barSvg);

  const first = await stln2rgba(bytes, { width: 10, height: 10 });
  const firstPixels = first.pixels;
  const firstLen = firstPixels.length;
  const firstSum = checksum(firstPixels);

  const second = await stln2rgba(bytes, { width: 20, height: 10 });
  assert(
    second.width === 20 && second.height === 10,
    "second call must apply its own distinct options",
  );
  assert(
    checksum(second.pixels) !== firstSum || second.pixels.length !== firstLen,
    "second result must genuinely differ from the first result",
  );
  assert(
    firstPixels.length === firstLen,
    "first result's pixels must retain their length after a second call",
  );
  assert(
    checksum(firstPixels) === firstSum,
    "first result's pixels must retain their contents after a second call",
  );
});

Deno.test("Stln2RgbaOptions accepts a plain object literal, not a wasm class instance", async () => {
  const bytes = await svg2stln(SVG);

  const full = await stln2rgba(bytes, {
    width: 100,
    height: 50,
    alphaMode: "premultiplied",
  });
  assert(
    full.width === 100 && full.height === 50,
    "width/height options must be honored exactly",
  );
  assert(
    full.alphaMode === "premultiplied",
    "alphaMode option must be honored",
  );

  const partial = await stln2rgba(bytes, { width: 100 });
  assert(partial.width === 100, "width-only option must be honored");
  assert(
    partial.height === 10,
    "unset height must fall back to the natural SVG height",
  );
  assert(
    partial.alphaMode === "straight",
    "unset alphaMode must default to straight",
  );

  const empty = await stln2rgba(bytes, {});
  const omitted = await stln2rgba(bytes);
  assert(
    empty.width === omitted.width && empty.height === omitted.height &&
      empty.alphaMode === omitted.alphaMode &&
      checksum(empty.pixels) === checksum(omitted.pixels),
    "an empty options object must behave exactly like omitting options",
  );
});

Deno.test("public subpaths export nothing whose name starts with __wasm_", async () => {
  const modules: Array<[string, Record<string, unknown>]> = [
    ["src/stln.ts", await import("../src/stln.ts")],
    ["src/stln-rgba.ts", await import("../src/stln-rgba.ts")],
    ["src/svg2rgba.ts", await import("../src/svg2rgba.ts")],
    ["src/mod.ts", await import("../src/mod.ts")],
  ];

  for (const [name, m] of modules) {
    for (const key of Object.keys(m)) {
      assert(
        !key.startsWith("__wasm_"),
        `${name} must not export the internal glue name ${key}`,
      );
    }
  }
});
