// Deno tests for the runtime shape of usvg2rgba's public surface (§3.4 layer).
//
// These guard the §6 fix that made the public API function-only: they verify
// at runtime that `usvg2rgba` returns a plain data object (never a
// wasm-bindgen class instance), that results survive reuse of the Wasm
// instance, that options are plain object literals, and that no `__wasm_*`
// glue symbol is reachable from the public subpaths.

import { usvg2rgba } from "../src/rgba.ts";
import { svg2usvg } from "../src/usvg.ts";

const SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"
  shape-rendering="geometricPrecision">
  <rect width="10" height="10" fill="#ff0000"/>
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

Deno.test("usvg2rgba result is a plain object, not a wasm-bindgen class instance", async () => {
  const bytes = await svg2usvg(SVG);
  const result = await usvg2rgba(bytes);

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

Deno.test("usvg2rgba result stays valid after a second call reuses the wasm instance", async () => {
  const barSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"
      shape-rendering="geometricPrecision">
  <rect width="4" height="10" fill="#ff0000"/>
</svg>`;
  const bytes = await svg2usvg(barSvg);

  const first = await usvg2rgba(bytes, { width: 10, height: 10 });
  const firstPixels = first.pixels;
  const firstLen = firstPixels.length;
  const firstSum = checksum(firstPixels);

  const second = await usvg2rgba(bytes, { width: 20, height: 10 });
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

Deno.test("Usvg2RgbaOptions accepts a plain object literal, not a wasm class instance", async () => {
  const bytes = await svg2usvg(SVG);

  const full = await usvg2rgba(bytes, {
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

  const partial = await usvg2rgba(bytes, { width: 100 });
  assert(partial.width === 100, "width-only option must be honored");
  assert(
    partial.height === 10,
    "unset height must fall back to the natural SVG height",
  );
  assert(
    partial.alphaMode === "straight",
    "unset alphaMode must default to straight",
  );

  const empty = await usvg2rgba(bytes, {});
  const omitted = await usvg2rgba(bytes);
  assert(
    empty.width === omitted.width && empty.height === omitted.height &&
      empty.alphaMode === omitted.alphaMode &&
      checksum(empty.pixels) === checksum(omitted.pixels),
    "an empty options object must behave exactly like omitting options",
  );
});

Deno.test("public subpaths export nothing whose name starts with __wasm_", async () => {
  const rgba = await import("../src/rgba.ts");
  const mod = await import("../src/mod.ts");

  for (
    const [name, m] of [
      ["src/rgba.ts", rgba],
      ["src/mod.ts", mod],
    ] as const
  ) {
    for (const key of Object.keys(m)) {
      assert(
        !key.startsWith("__wasm_"),
        `${name} must not export the internal glue name ${key}`,
      );
    }
  }

  for (
    const [name, path] of [
      ["src/rgba.ts", "../src/rgba.ts"],
      ["src/mod.ts", "../src/mod.ts"],
    ] as const
  ) {
    const source = Deno.readTextFileSync(new URL(path, import.meta.url));
    for (const rawLine of source.split("\n")) {
      const line = rawLine.trim();
      if (line.startsWith("//")) {
        continue;
      }
      assert(
        !/^export\s+(class|function|const|let|var)\s+__wasm_/.test(line),
        `${name} exports an internal glue symbol on: ${line}`,
      );
      assert(
        !(/^export\s*\{/.test(line) && line.includes("__wasm_")),
        `${name} re-exports an internal glue symbol on: ${line}`,
      );
    }
  }
});
