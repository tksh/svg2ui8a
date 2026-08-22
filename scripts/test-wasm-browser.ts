/**
 * Browser-verified Wasm tests — §8.4 second entry point.
 *
 * Extends the §8.3 harness to also load `src/rgba.ts` and call
 * `usvg2rgba`, asserting correct `RgbaResult` shape. Full §3.3
 * cases (byte-identity vs native core) are deferred to §8.5.
 *
 * Run: `deno task test:wasm:browser` or `deno run -A scripts/test-wasm-browser.ts`
 * Requires pinned Chrome 125.0.6400.0 via Astral (see scripts/ensure-chrome.ts).
 */

import { launch, SUPPORTED_VERSIONS } from "jsr:@astral/astral@0.5.6";

const EXPECTED_CHROME = "125.0.6400.0";
const SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"/></svg>';

if (SUPPORTED_VERSIONS.chrome !== EXPECTED_CHROME) {
  console.error(
    `SUPPORTED_VERSIONS.chrome is ${
      JSON.stringify(SUPPORTED_VERSIONS.chrome)
    } but expected ${JSON.stringify(EXPECTED_CHROME)}`,
  );
  Deno.exit(1);
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

async function main(): Promise<void> {
  console.log(`Astral: jsr:@astral/astral@0.5.6 Chrome ${EXPECTED_CHROME}`);
  const [usvgJs, rgbaJs] = await Promise.all([
    bundleTsToJs("src/usvg.ts"),
    bundleTsToJs("src/rgba.ts"),
  ]);
  console.log(`bundle: src/usvg.ts → ${usvgJs.length} bytes JS`);
  console.log(`bundle: src/rgba.ts → ${rgbaJs.length} bytes JS`);

  // Minimal HTTP server serving the bundled modules and a fixture HTML.
  // Use 127.0.0.1 + random port (port: 0) so we never collide.
  const html = `<!doctype html>
<html><head><meta charset="utf-8"><title>wasm-browser §8.4</title></head>
<body>
<script type="module">
import { svg2usvg } from "/usvg.js";
import { usvg2rgba } from "/rgba.js";
window.__svg2usvg = svg2usvg;
window.__usvg2rgba = usvg2rgba;
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
        if (url.pathname === "/usvg.js") {
          return new Response(usvgJs, {
            headers: {
              "content-type": "application/javascript; charset=utf-8",
            },
          });
        }
        if (url.pathname === "/rgba.js") {
          return new Response(rgbaJs, {
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
    // Wait a tick for onListen to fire; Deno.serve is async but onListen is sync.
    await new Promise((r) => setTimeout(r, 50));
    const base = `http://127.0.0.1:${port}/`;
    console.log(`server: ${base}`);

    let browser;
    try {
      browser = await launch({ headless: true, product: "chrome" });
      console.log(`launch: ok`);

      const page = await browser.newPage(base);
      console.log(`page: navigated to ${base}`);

      // Wait for window.__ready set by the module script.
      await page.waitForFunction(() =>
        (globalThis as unknown as { __ready?: boolean }).__ready === true
      );
      console.log(`ready: window.__ready true`);

      const result = await page.evaluate(async (svg) => {
        const g = globalThis as unknown as {
          __svg2usvg: (s: string) => Promise<Uint8Array>;
          __usvg2rgba: (
            usvg: Uint8Array,
            opts?: { width?: number; height?: number; alphaMode?: string },
          ) => Promise<{
            width: number;
            height: number;
            alphaMode: string;
            pixels: Uint8Array;
          }>;
        };
        const svg2usvg = g.__svg2usvg;
        const usvg2rgba = g.__usvg2rgba;
        if (typeof svg2usvg !== "function") {
          throw new Error("window.__svg2usvg not found");
        }
        if (typeof usvg2rgba !== "function") {
          throw new Error("window.__usvg2rgba not found");
        }

        // svg2usvg
        const pending = svg2usvg(svg);
        const isPromise = pending instanceof Promise;
        const bytes = await pending;
        const isU8 = bytes instanceof Uint8Array;

        // usvg2rgba — natural size, then check shape
        const pendingRgba = usvg2rgba(bytes);
        const isRgbaPromise = pendingRgba instanceof Promise;
        const rgba = await pendingRgba;
        const pixelsIsU8 = rgba.pixels instanceof Uint8Array;
        const expectedLen = rgba.width * rgba.height * 4;
        const pixelsLenOk = rgba.pixels.length === expectedLen;

        return {
          svg: {
            isPromise,
            isU8,
            length: bytes.length,
            ctorName: bytes.constructor.name,
            head: Array.from(bytes.slice(0, 8)),
          },
          rgba: {
            isPromise: isRgbaPromise,
            width: rgba.width,
            height: rgba.height,
            alphaMode: rgba.alphaMode,
            pixelsIsU8,
            pixelsLength: rgba.pixels.length,
            expectedLen,
            pixelsLenOk,
            pixelsCtor: rgba.pixels.constructor.name,
          },
          // Return bytes for potential cross-check (small SVG, cheap).
          usvgHead: Array.from(bytes.slice(0, 8)),
        };
      }, { args: [SVG] });

      console.log(
        `browser svg2usvg: isPromise=${result.svg.isPromise} isU8=${result.svg.isU8} length=${result.svg.length} ctor=${result.svg.ctorName} head=${
          JSON.stringify(result.svg.head)
        }`,
      );
      console.log(
        `browser usvg2rgba: isPromise=${result.rgba.isPromise} width=${result.rgba.width} height=${result.rgba.height} alphaMode=${result.rgba.alphaMode} pixelsIsU8=${result.rgba.pixelsIsU8} pixelsLength=${result.rgba.pixelsLength} expected=${result.rgba.expectedLen} ctor=${result.rgba.pixelsCtor}`,
      );

      if (!result.svg.isPromise) {
        throw new Error("svg2usvg did not return a Promise in browser");
      }
      if (!result.svg.isU8) {
        throw new Error(
          `svg2usvg did not resolve to Uint8Array in browser (got ${result.svg.ctorName})`,
        );
      }
      if (result.svg.length === 0) {
        throw new Error("svg2usvg resolved to empty Uint8Array in browser");
      }
      console.log(
        `✓ svg2usvg in headless Chrome → non-empty Uint8Array (${result.svg.length} bytes)`,
      );

      if (!result.rgba.isPromise) {
        throw new Error("usvg2rgba did not return a Promise in browser");
      }
      if (!result.rgba.pixelsIsU8) {
        throw new Error(
          `usvg2rgba pixels not Uint8Array in browser (got ${result.rgba.pixelsCtor})`,
        );
      }
      if (!result.rgba.pixelsLenOk) {
        throw new Error(
          `usvg2rgba pixels.length ${result.rgba.pixelsLength} !== width*height*4 ${result.rgba.expectedLen} (w=${result.rgba.width} h=${result.rgba.height})`,
        );
      }
      if (typeof result.rgba.alphaMode !== "string") {
        throw new Error("usvg2rgba alphaMode not string in browser");
      }
      console.log(
        `✓ usvg2rgba in headless Chrome → RgbaResult { width=${result.rgba.width} height=${result.rgba.height} alphaMode=${result.rgba.alphaMode} pixels=${result.rgba.pixelsLength} }`,
      );

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

  console.log(`test-wasm-browser §8.4: PASS`);
}

if (import.meta.main) {
  try {
    await main();
  } catch (e) {
    console.error(`test-wasm-browser §8.4: FAIL\n${e}`);
    Deno.exit(1);
  }
}
