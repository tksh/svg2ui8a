/**
 * Wasm-loading tests (`docs/plans/wasm-file-delivery.md`).
 *
 * The package no longer embeds its Wasm binaries: every capability loads a
 * separate `.wasm` file. These tests cover that loading model:
 *
 *   1. the default location is the asset shipped next to the module,
 *   2. an explicit URL is honored, including over HTTP (the Deno server-side
 *      case, and the shape a browser deployment uses),
 *   3. initialization is cached — the binary is fetched exactly once, no
 *      matter how often the capability is used,
 *   4. a missing binary fails with a message that names the URL, and
 *   5. the URL cannot be changed after the module has been initialized.
 *
 * Each scenario needs its own module instance, because the initialization
 * state is cached per module. A cache-busting query on the import specifier
 * gives Deno a fresh instance of the same module.
 */

import type { RgbaResult, Svg2RgbaOptions } from "../src/svg2rgba.ts";

const ASSETS_DIR = new URL("../assets/", import.meta.url);

/** The public surface of the `svg2rgba` capability. */
interface RgbaModule {
  svg2rgba: (svg: string, options?: Svg2RgbaOptions) => Promise<RgbaResult>;
  initSvg2rgba: (options?: { wasmUrl?: string | URL }) => Promise<void>;
}

/** The public surface of the package root. */
interface RootModule {
  initSvg2ui8a: (options?: {
    svg2usvgWasmUrl?: string | URL;
    svg2rgbaWasmUrl?: string | URL;
  }) => Promise<void>;
  svg2usvg: (svg: string) => Promise<Uint8Array>;
  svg2rgba: (svg: string, options?: Svg2RgbaOptions) => Promise<RgbaResult>;
}

let instanceCounter = 0;

/** Import a fresh instance of a module of this package. */
function freshInstance<T>(path: string): Promise<T> {
  const url = new URL(`../src/${path}`, import.meta.url);
  url.searchParams.set("instance", String(instanceCounter++));
  return import(url.href) as Promise<T>;
}

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) {
    throw new Error(msg);
  }
}

const SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" fill="#00ff00"/></svg>';

function assertBytesEqual(a: Uint8Array, b: Uint8Array, label: string): void {
  assert(
    a.length === b.length,
    `${label}: length mismatch ${a.length} != ${b.length}`,
  );
  for (let i = 0; i < a.length; i++) {
    assert(a[i] === b[i], `${label}: byte mismatch at offset ${i}`);
  }
}

/**
 * Serve the package the way a registry serves it: the module under `src/`
 * and the Wasm binaries as siblings under `assets/`. The default location of
 * a binary is resolved relative to the module, so this reproduces the
 * published layout — and the way a browser deployment serves the two.
 */
function startPackageServer(): {
  base: string;
  requests: Map<string, number>;
  shutdown: () => Promise<void>;
} {
  const requests = new Map<string, number>();
  let port = 0;
  const server = Deno.serve(
    { hostname: "127.0.0.1", port: 0, onListen: ({ port: p }) => port = p },
    (req) => {
      const url = new URL(req.url);
      requests.set(url.pathname, (requests.get(url.pathname) ?? 0) + 1);
      const moduleName = url.pathname.replace(/^\/src\//, "");
      if (url.pathname.startsWith("/src/") && moduleName.endsWith(".ts")) {
        const source = Deno.readTextFileSync(
          new URL(`../src/${moduleName}`, import.meta.url),
        );
        return new Response(source, {
          headers: { "content-type": "text/typescript; charset=utf-8" },
        });
      }
      const assetName = url.pathname.replace(/^\/assets\//, "");
      if (url.pathname.startsWith("/assets/") && !assetName.includes("/")) {
        let bytes: Uint8Array;
        try {
          bytes = Deno.readFileSync(new URL(assetName, ASSETS_DIR));
        } catch {
          return new Response("not found", { status: 404 });
        }
        return new Response(new Uint8Array(bytes), {
          headers: { "content-type": "application/wasm" },
        });
      }
      return new Response("not found", { status: 404 });
    },
  );
  return {
    base: `http://127.0.0.1:${port}`,
    requests,
    shutdown: () => server.shutdown(),
  };
}

async function main(): Promise<void> {
  // 1. The default location is the asset shipped next to the module.
  const defaultModule = await freshInstance<RgbaModule>("svg2rgba.ts");
  const defaultUrl = new URL("../assets/svg2rgba_bg.wasm", import.meta.url);
  const defaultResponse = await fetch(defaultUrl);
  assert(
    defaultResponse.ok,
    `default asset ${defaultUrl} must be readable (HTTP ${defaultResponse.status})`,
  );
  const defaultBytes = new Uint8Array(await defaultResponse.arrayBuffer());
  const defaultModule2 = await WebAssembly.compile(defaultBytes);
  assert(
    defaultModule2 instanceof WebAssembly.Module,
    "default asset must be a Wasm module",
  );
  const defaultResult = await defaultModule.svg2rgba(SVG);
  assert(
    defaultResult.pixels.length === defaultResult.width *
        defaultResult.height * 4,
    "default loading must produce a well-formed result",
  );
  console.log(
    `  ✓ default loading: ${defaultUrl.pathname} fetched and rendered (${defaultResult.width}×${defaultResult.height})`,
  );

  const server = startPackageServer();
  try {
    // 2. The published layout: a module served next to its assets resolves
    //    its own binary, with no configuration from the consumer.
    const published = await import(
      `${server.base}/src/svg2usvg.ts`
    ) as { svg2usvg: (svg: string) => Promise<Uint8Array> };
    const publishedBytes = await published.svg2usvg(SVG);
    assert(
      new TextDecoder().decode(publishedBytes).includes("<svg"),
      "the served module must normalize SVGs",
    );
    assert(
      (server.requests.get("/assets/svg2usvg_bg.wasm") ?? 0) === 1,
      "the module must fetch its own sibling asset exactly once",
    );
    console.log(
      "  ✓ published layout: the module resolved and fetched its sibling asset",
    );

    // 3. An explicit URL is honored.
    const explicit = await freshInstance<RgbaModule>("svg2rgba.ts");
    const wasmUrl = `${server.base}/assets/svg2rgba_bg.wasm`;
    await explicit.initSvg2rgba({ wasmUrl });
    const explicitResult = await explicit.svg2rgba(SVG);
    assertBytesEqual(
      explicitResult.pixels,
      defaultResult.pixels,
      "explicit URL vs default loading",
    );
    assert(
      (server.requests.get("/assets/svg2rgba_bg.wasm") ?? 0) === 1,
      "the binary must be fetched exactly once",
    );
    console.log(`  ✓ explicit URL: ${wasmUrl} fetched and rendered`);

    // 4. Initialization is cached: further use must not refetch.
    await explicit.initSvg2rgba();
    await explicit.initSvg2rgba();
    await explicit.svg2rgba(SVG);
    assert(
      (server.requests.get("/assets/svg2rgba_bg.wasm") ?? 0) === 1,
      "repeated use must not refetch the binary",
    );
    console.log("  ✓ initialization is cached: the binary is fetched once");

    // 5. A missing binary fails with a message that names the URL.
    const missing = await freshInstance<RgbaModule>("svg2rgba.ts");
    const missingUrl = `${server.base}/assets/missing_bg.wasm`;
    let missingError: unknown;
    try {
      await missing.initSvg2rgba({ wasmUrl: missingUrl });
    } catch (e) {
      missingError = e;
    }
    assert(missingError instanceof Error, "a missing binary must reject");
    assert(
      missingError.message.includes(missingUrl),
      `the error must name the URL (got: ${missingError.message})`,
    );
    assert(
      missingError.message.includes("404"),
      `the error must carry the HTTP status (got: ${missingError.message})`,
    );
    console.log(`  ✓ missing binary: rejected with "${missingError.message}"`);

    // 6. The URL cannot be changed after initialization.
    const late = await freshInstance<RgbaModule>("svg2rgba.ts");
    await late.svg2rgba(SVG);
    let lateError: unknown;
    try {
      await late.initSvg2rgba({ wasmUrl });
    } catch (e) {
      lateError = e;
    }
    assert(lateError instanceof Error, "a late URL override must reject");
    assert(
      lateError.message.includes("already initialized"),
      `the error must say the module is already initialized (got: ${lateError.message})`,
    );
    console.log(`  ✓ late URL override: rejected with "${lateError.message}"`);

    // 7. The root entry point initializes both capabilities.
    const root = await freshInstance<RootModule>("mod.ts");
    await root.initSvg2ui8a({ svg2rgbaWasmUrl: wasmUrl });
    const rootResult = await root.svg2rgba(SVG);
    assertBytesEqual(
      rootResult.pixels,
      defaultResult.pixels,
      "root entry point vs default loading",
    );
    const usvgBytes = await root.svg2usvg(SVG);
    assert(
      new TextDecoder().decode(usvgBytes).includes("<svg"),
      "the root entry point must normalize SVGs too",
    );
    console.log(
      "  ✓ root entry point: initSvg2ui8a initializes both capabilities",
    );
  } finally {
    await server.shutdown();
  }

  console.log("Wasm-loading tests passed.");
}

if (import.meta.main) {
  try {
    await main();
  } catch (e) {
    console.error(`Wasm-loading test failed: ${e}`);
    Deno.exit(1);
  }
}
