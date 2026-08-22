/**
 * Browser-verified Wasm tests — §8.3 minimal harness, one entry point.
 *
 * Loads `src/usvg.ts` inside headless Chrome via Astral/CDP and calls
 * `svg2usvg` once, asserting a non-empty `Uint8Array`. No second entry
 * point yet (deferred to §8.4), no full §3.3 cases yet (deferred to §8.5).
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
  const usvgJs = await bundleTsToJs("src/usvg.ts");
  console.log(`bundle: src/usvg.ts → ${usvgJs.length} bytes JS`);

  // Minimal HTTP server serving the bundled module and a fixture HTML.
  // Use 127.0.0.1 + random port (port: 0) so we never collide.
  const html = `<!doctype html>
<html><head><meta charset="utf-8"><title>wasm-browser §8.3</title></head>
<body>
<script type="module">
import { svg2usvg } from "/usvg.js";
window.__svg2usvg = svg2usvg;
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
        const svg2usvg = (globalThis as unknown as {
          __svg2usvg: (s: string) => Promise<Uint8Array>;
        }).__svg2usvg;
        if (typeof svg2usvg !== "function") {
          throw new Error("window.__svg2usvg not found");
        }
        const pending = svg2usvg(svg);
        const isPromise = pending instanceof Promise;
        const bytes = await pending;
        const isU8 = bytes instanceof Uint8Array;
        return {
          isPromise,
          isU8,
          length: bytes.length,
          ctorName: bytes.constructor.name,
          // Return first bytes for diagnostic; keep small.
          head: Array.from(bytes.slice(0, 8)),
        };
      }, { args: [SVG] });

      console.log(
        `browser result: isPromise=${result.isPromise} isU8=${result.isU8} length=${result.length} ctor=${result.ctorName} head=${
          JSON.stringify(result.head)
        }`,
      );

      if (!result.isPromise) {
        throw new Error("svg2usvg did not return a Promise in browser");
      }
      if (!result.isU8) {
        throw new Error(
          `svg2usvg did not resolve to Uint8Array in browser (got ${result.ctorName})`,
        );
      }
      if (result.length === 0) {
        throw new Error("svg2usvg resolved to empty Uint8Array in browser");
      }

      console.log(
        `✓ svg2usvg in headless Chrome → non-empty Uint8Array (${result.length} bytes)`,
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

  console.log(`test-wasm-browser §8.3: PASS`);
}

if (import.meta.main) {
  try {
    await main();
  } catch (e) {
    console.error(`test-wasm-browser §8.3: FAIL\n${e}`);
    Deno.exit(1);
  }
}
