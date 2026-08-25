/**
 * Browser-verified Wasm tests — §8.5 full §3.3 required cases.
 *
 * Exercises both Wasm entry points inside headless Chrome via Astral/CDP
 * and verifies the two §3.3 guarantees: (1) correct Promise types and (2)
 * output byte-identical to the Rust-native `core` output for the same
 * input (via per-crate `dump_core` binaries). Complements
 * `scripts/test-wasm.ts` (Deno-side) — same fixtures and option sets,
 * different runtime.
 *
 * Run: `deno task test:wasm:browser` or `deno run -A scripts/test-wasm-browser.ts`
 * Requires pinned Chrome 125.0.6400.0 via Astral (see scripts/ensure-chrome.ts).
 */

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { launch, SUPPORTED_VERSIONS } from "@astral/astral";

const EXPECTED_CHROME = "125.0.6400.0";
const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

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

type RgbaOpts = { width?: number; height?: number; alphaMode?: string };

const RGBA_OPTION_SETS: Array<[string, RgbaOpts]> = [
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

if (SUPPORTED_VERSIONS.chrome !== EXPECTED_CHROME) {
  console.error(
    `SUPPORTED_VERSIONS.chrome is ${
      JSON.stringify(SUPPORTED_VERSIONS.chrome)
    } but expected ${JSON.stringify(EXPECTED_CHROME)}`,
  );
  Deno.exit(1);
}

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
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

async function bundleTsToJs(tsPath: string): Promise<string> {
  const cmd = new Deno.Command("deno", {
    args: ["bundle", tsPath],
    stdout: "piped",
    stderr: "piped",
  });
  const out = await cmd.output();
  if (!out.success) {
    const err = new TextDecoder().decode(out.stderr);
    throw new Error(`deno bundle ${tsPath} failed:\n${err}`);
  }
  return new TextDecoder().decode(out.stdout);
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

async function main(): Promise<void> {
  console.log(`Astral: jsr:@astral/astral@0.5.6 Chrome ${EXPECTED_CHROME}`);
  const [svgUsvgJs, svgRgbaJs] = await Promise.all([
    bundleTsToJs("src/svg2usvg.ts"),
    bundleTsToJs("src/svg2rgba.ts"),
  ]);
  console.log(`bundle: src/svg2usvg.ts → ${svgUsvgJs.length} bytes JS`);
  console.log(`bundle: src/svg2rgba.ts → ${svgRgbaJs.length} bytes JS`);

  const html = `<!doctype html>
<html><head><meta charset="utf-8"><title>wasm-browser §8.5</title></head>
<body>
<script type="module">
import { svg2usvg } from "/svg2usvg.js";
import { svg2rgba } from "/svg2rgba.js";
window.__svg2usvg = svg2usvg;
window.__svg2rgba = svg2rgba;
window.__ready = true;
</script>
</body></html>`;

  let server: Deno.HttpServer | undefined;
  let port = 0;

  try {
    server = Deno.serve(
      { hostname: "127.0.0.1", port: 0, onListen: ({ port: p }) => port = p },
      (req) => {
        const url = new URL(req.url);
        const jsFiles: Record<string, string> = {
          "/svg2usvg.js": svgUsvgJs,
          "/svg2rgba.js": svgRgbaJs,
        };
        if (jsFiles[url.pathname]) {
          return new Response(jsFiles[url.pathname], {
            headers: {
              "content-type": "application/javascript; charset=utf-8",
            },
          });
        }
        if (url.pathname === "/" || url.pathname === "/index.html") {
          return new Response(html, {
            headers: { "content-type": "text/html; charset=utf-8" },
          });
        }
        return new Response("not found", { status: 404 });
      },
    );
    await new Promise((r) => setTimeout(r, 50));
    const base = `http://127.0.0.1:${port}/`;
    console.log(`server: ${base}`);

    let browser;
    try {
      browser = await launch({ headless: true, product: "chrome" });
      console.log(`launch: ok`);
      const page = await browser.newPage(base);
      console.log(`page: navigated to ${base}`);
      await page.waitForFunction(() =>
        (globalThis as unknown as { __ready?: boolean }).__ready === true
      );
      console.log(`ready: window.__ready true`);

      // deno-lint-ignore no-inner-declarations
      async function browserSvg2usvg(svg: string): Promise<Uint8Array> {
        const res = await page.evaluate(async (svgStr) => {
          const g = globalThis as unknown as {
            __svg2usvg: (s: string) => Promise<Uint8Array>;
          };
          const pending = g.__svg2usvg(svgStr);
          const isPromise = pending instanceof Promise;
          const bytes = await pending;
          const isU8 = bytes instanceof Uint8Array;
          return {
            isPromise,
            isU8,
            ctor: bytes.constructor.name,
            arr: Array.from(bytes),
          };
        }, { args: [svg] });

        assert(
          (res as { isPromise: boolean }).isPromise,
          "svg2usvg must return a Promise in browser",
        );
        assert(
          (res as { isU8: boolean }).isU8,
          `svg2usvg must resolve to Uint8Array in browser (got ${
            (res as { ctor: string }).ctor
          })`,
        );
        const arr = (res as { arr: number[] }).arr;
        assert(
          arr.length > 0,
          "svg2usvg must resolve to non-empty Uint8Array in browser",
        );
        return new Uint8Array(arr);
      }

      const encoder = new TextEncoder();
      for (const [name, svgStr] of SVGS) {
        const browserBytes = await browserSvg2usvg(svgStr);
        const native = await runNativeCore(
          "svg2usvg",
          [],
          encoder.encode(svgStr),
        );
        assertBytesEqual(
          browserBytes,
          native.stdout,
          `svg2usvg [${name}] browser vs native`,
        );
        console.log(
          `  ✓ svg2usvg [${name}] browser vs native (${browserBytes.length} bytes)`,
        );

        for (const [optLabel, opts] of RGBA_OPTION_SETS.slice(0, 3)) {
          const res = await page.evaluate(async (svgStr, o) => {
            const g = globalThis as unknown as {
              __svg2rgba: (
                s: string,
                opts?: { width?: number; height?: number; alphaMode?: string },
              ) => Promise<{
                width: number;
                height: number;
                alphaMode: string;
                pixels: Uint8Array;
              }>;
            };
            const pending = g.__svg2rgba(svgStr, o as RgbaOpts);
            const isPromise = pending instanceof Promise;
            const rgba = await pending;
            return {
              isPromise,
              width: rgba.width,
              height: rgba.height,
              alphaMode: rgba.alphaMode,
              pixelsIsU8: rgba.pixels instanceof Uint8Array,
              pixelsCtor: rgba.pixels.constructor.name,
              pixelsArr: Array.from(rgba.pixels),
            };
          }, { args: [svgStr, opts as unknown as RgbaOpts] });

          assert(
            (res as { isPromise: boolean }).isPromise &&
              (res as { pixelsIsU8: boolean }).pixelsIsU8,
            `svg2rgba [${name}] ${optLabel}: bad shape in browser (got ${
              (res as { pixelsCtor: string }).pixelsCtor
            })`,
          );
          const browserOne = {
            width: (res as { width: number }).width,
            height: (res as { height: number }).height,
            alphaMode: (res as { alphaMode: string }).alphaMode,
            pixels: new Uint8Array((res as { pixelsArr: number[] }).pixelsArr),
          };
          const nativeOne = await runNativeCore("svg2rgba", [
            String(opts.width ?? 0),
            String(opts.height ?? 0),
            opts.alphaMode ?? "straight",
          ], encoder.encode(svgStr));
          const [oneW, oneH, oneMode] = nativeOne.stderr.trim().split(" ");
          assert(
            browserOne.width === Number(oneW) &&
              browserOne.height === Number(oneH) &&
              browserOne.alphaMode === oneMode,
            `[${name}] ${optLabel}: svg2rgba metadata differs from native`,
          );
          assertBytesEqual(
            browserOne.pixels,
            nativeOne.stdout,
            `[${name}] ${optLabel}: svg2rgba browser vs native`,
          );
          console.log(
            `  ✓ svg2rgba [${name}] ${optLabel} browser vs native`,
          );
        }
      }

      await page.close();
    } finally {
      if (browser) {
        try {
          await browser.close();
        } catch { /* ignore */ }
        console.log(`close: ok`);
      }
    }
  } finally {
    if (server) {
      await server.shutdown();
      console.log(`server: shutdown`);
    }
  }

  console.log(`test-wasm-browser §8.5: PASS — all §3.3 cases browser-verified`);
}

if (import.meta.main) {
  try {
    await main();
  } catch (e) {
    console.error(`test-wasm-browser §8.5: FAIL\n${e}`);
    Deno.exit(1);
  }
}
