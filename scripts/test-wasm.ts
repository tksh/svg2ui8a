/**
 * Wasm-layer tests (engineering-playbook §3.3, task.md §7).
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

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { usvg2rgba, type Usvg2RgbaOptions } from "../src/rgba.ts";
import { svg2usvg } from "../src/usvg.ts";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

/**
 * The Straightlines fixture (task.md §13). Exercised through every layer so
 * groups (`<g>` with compositing opacity) and full stroke styling are
 * verified against the native core, not just flat filled rects.
 */
const FIXTURE_SVG = await Deno.readTextFile(
  join(ROOT, "tests", "fixtures", "straightlines-sample.svg"),
);

/** Test inputs fed identically to the TS entry point and the native core. */
const SVGS: Array<[string, string]> = [
  [
    "full-canvas rect",
    '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"/></svg>',
  ],
  [
    "partial-width bar",
    '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="4" height="10" fill="#ff0000"/></svg>',
  ],
  ["straightlines fixture (layers + strokes)", FIXTURE_SVG],
];

const RGBA_OPTION_SETS: Array<[string, Usvg2RgbaOptions]> = [
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
    cwd: join(ROOT, "crates", crate),
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
  const wasmUsvg = await pending;
  assert(
    wasmUsvg instanceof Uint8Array,
    "svg2usvg must resolve to a Uint8Array",
  );

  const encoder = new TextEncoder();
  const native = await runNativeCore("svg2usvg", [], encoder.encode(svgStr));
  assertBytesEqual(wasmUsvg, native.stdout, "svg2usvg vs native core");
  return wasmUsvg;
}

async function checkUsvg2rgba(
  usvgBytes: Uint8Array,
  label: string,
  options: Usvg2RgbaOptions,
): Promise<void> {
  const pending = usvg2rgba(usvgBytes, options);
  assert(
    pending instanceof Promise,
    "usvg2rgba must return a Promise<RgbaResult>",
  );
  const result = await pending;
  assert(
    typeof result.width === "number" && typeof result.height === "number",
    "usvg2rgba must resolve to an RgbaResult with numeric dimensions",
  );
  assert(
    typeof result.alphaMode === "string",
    "usvg2rgba must resolve to an RgbaResult with an alphaMode string",
  );
  assert(
    result.pixels instanceof Uint8Array,
    "usvg2rgba must resolve to an RgbaResult with a Uint8Array pixels field",
  );

  const native = await runNativeCore(
    "usvg2rgba",
    [
      String(options.width ?? 0),
      String(options.height ?? 0),
      options.alphaMode ?? "straight",
    ],
    usvgBytes,
  );
  const [nativeW, nativeH, nativeMode] = native.stderr.trim().split(" ");
  assert(
    result.width === Number(nativeW) && result.height === Number(nativeH),
    `${label}: dimensions differ from native core (${result.width}x${result.height} vs ${nativeW}x${nativeH})`,
  );
  assert(
    result.alphaMode === nativeMode,
    `${label}: alphaMode differs from native core (${result.alphaMode} vs ${nativeMode})`,
  );
  assertBytesEqual(result.pixels, native.stdout, `${label}: pixels`);
}

async function main(): Promise<void> {
  for (const [name, svgStr] of SVGS) {
    const usvgBytes = await checkSvg2usvg(svgStr);
    console.log(
      `  ✓ svg2usvg [${name}]: Wasm output matches native core (${usvgBytes.length} bytes)`,
    );
    for (const [optionLabel, options] of RGBA_OPTION_SETS) {
      await checkUsvg2rgba(usvgBytes, `[${name}] ${optionLabel}`, options);
      console.log(
        `  ✓ usvg2rgba [${name}] ${optionLabel}: Wasm output matches native core`,
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
