/**
 * Wasm-layer tests (engineering-playbook §3.3, task.md §16).
 *
 * For each Wasm artifact, exercised through the shipped TS entry point:
 *   1. the entry point returns the correct Promise type, and
 *   2. the output is byte-identical to the Rust-native `core` output for
 *      the same input.
 *
 * The native expectations are computed fresh on every run via each crate's
 * `dump_core` example binary, so they always track the current crate source
 * instead of stale checked-in goldens.
 */

import { svg2rgba, type Svg2RgbaOptions } from "../src/svg2rgba.ts";
import { svg2usvg } from "../src/svg2usvg.ts";

/** Test inputs fed identically to the TS entry point and the native core. */
const SVGS: Array<[string, string]> = [
  [
    "tiny rect",
    '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"/></svg>',
  ],
  [
    "stroked diagonal",
    '<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 32 32"><path d="M 2 3 L 29 22" stroke="#000000" stroke-width="1" fill="none"/></svg>',
  ],
  [
    "group with opacity",
    '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><g opacity="0.8"><rect width="10" height="10" fill="#00ff00"/></g></svg>',
  ],
  [
    "gradient fill",
    '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><defs><linearGradient id="g" x1="0" y1="0" x2="10" y2="0"><stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></linearGradient></defs><rect width="10" height="10" fill="url(#g)"/></svg>',
  ],
];

const RGBA_OPTION_SETS: Array<[string, Svg2RgbaOptions]> = [
  ["natural size", {}],
  ["width only", { width: 20 }],
  ["height only", { height: 5 }],
  ["both set", { width: 20, height: 10 }],
  ["both set, premultiplied", {
    width: 20,
    height: 10,
    alphaMode: "premultiplied",
  }],
];

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) {
    throw new Error(msg);
  }
}

function assertBytesEqual(a: Uint8Array, b: Uint8Array, label: string): void {
  assert(
    a.length === b.length,
    `${label}: length mismatch ${a.length} != ${b.length}`,
  );
  for (let i = 0; i < a.length; i++) {
    assert(a[i] === b[i], `${label}: byte mismatch at offset ${i}`);
  }
}

async function runNativeCore(
  crate: string,
  args: string[],
  stdin: Uint8Array,
): Promise<{ stdout: Uint8Array; stderr: string }> {
  const command = new Deno.Command("cargo", {
    args: ["run", "--quiet", "--example", "dump_core", ...args],
    cwd: new URL(`../crates/${crate}/`, import.meta.url).pathname,
    stdin: "piped",
    stdout: "piped",
    stderr: "piped",
  });
  const child = command.spawn();
  const writer = child.stdin.getWriter();
  await writer.write(stdin);
  writer.releaseLock();
  await child.stdin.close();
  const output = await child.output();
  if (!output.success) {
    throw new Error(
      `native core run failed (crates/${crate} example dump_core): ${
        new TextDecoder().decode(output.stderr)
      }`,
    );
  }
  return {
    stdout: output.stdout,
    stderr: new TextDecoder().decode(output.stderr),
  };
}

async function checkSvg2usvg(svgStr: string): Promise<Uint8Array> {
  const pending = svg2usvg(svgStr);
  assert(
    pending instanceof Promise,
    "svg2usvg must return a Promise<Uint8Array>",
  );
  const wasmBytes = await pending;
  assert(
    wasmBytes instanceof Uint8Array,
    "svg2usvg must resolve to a Uint8Array",
  );

  const encoder = new TextEncoder();
  const native = await runNativeCore("svg2usvg", [], encoder.encode(svgStr));
  assertBytesEqual(wasmBytes, native.stdout, "svg2usvg vs native core");
  return wasmBytes;
}

async function checkSvg2rgba(
  svgStr: string,
  label: string,
  options: Svg2RgbaOptions,
): Promise<void> {
  const pending = svg2rgba(svgStr, options);
  assert(pending instanceof Promise, "svg2rgba must return a Promise");
  const result = await pending;
  assert(
    typeof result.width === "number" && typeof result.height === "number" &&
      typeof result.alphaMode === "string" &&
      result.pixels instanceof Uint8Array &&
      result.pixels.length === result.width * result.height * 4,
    `${label}: svg2rgba must resolve to a well-formed RgbaResult`,
  );

  const encoder = new TextEncoder();
  const native = await runNativeCore("svg2rgba", [
    String(options.width ?? 0),
    String(options.height ?? 0),
    options.alphaMode ?? "straight",
  ], encoder.encode(svgStr));
  const [nativeW, nativeH, nativeMode] = native.stderr.trim().split(" ");
  assert(
    result.width === Number(nativeW) && result.height === Number(nativeH),
    `${label}: dimensions differ from native core`,
  );
  assert(result.alphaMode === nativeMode, `${label}: alphaMode differs`);
  assertBytesEqual(result.pixels, native.stdout, `${label}: pixels`);
}

async function main(): Promise<void> {
  for (const [name, svgStr] of SVGS) {
    const usvgBytes = await checkSvg2usvg(svgStr);
    console.log(
      `  ✓ svg2usvg [${name}]: Wasm output matches native core (${usvgBytes.length} bytes)`,
    );
    // Verify usvg output is valid UTF-8 XML
    const decoded = new TextDecoder().decode(usvgBytes);
    assert(
      decoded.includes("<svg"),
      `svg2usvg [${name}]: output must contain <svg`,
    );
    for (const [optionLabel, options] of RGBA_OPTION_SETS.slice(0, 3)) {
      await checkSvg2rgba(
        svgStr,
        `[${name}] ${optionLabel}`,
        options as Svg2RgbaOptions,
      );
      console.log(
        `  ✓ svg2rgba [${name}] ${optionLabel}: Wasm output matches native core`,
      );
    }
  }
  console.log("Wasm-layer tests passed.");
}

if (import.meta.main) {
  try {
    await main();
  } catch (e) {
    console.error(`Wasm-layer test failed: ${e}`);
    Deno.exit(1);
  }
}
