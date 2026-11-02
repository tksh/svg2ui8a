// Tests for the bytes-based Wasm initialization path (`wasmBytes`).
//
// Each scenario imports a fresh module instance because the initialization
// state, including a failed promise, is cached per capability.

import type { RgbaResult, Svg2RgbaOptions } from "../src/svg2rgba.ts";

interface InitOptions {
  wasmUrl?: string | URL;
  wasmBytes?: Uint8Array;
}

interface RgbaModule {
  svg2rgba: (svg: string, options?: Svg2RgbaOptions) => Promise<RgbaResult>;
  initSvg2rgba: (options?: InitOptions) => Promise<void>;
}

interface UsvgModule {
  svg2usvg: (svg: string) => Promise<Uint8Array>;
  initSvg2usvg: (options?: InitOptions) => Promise<void>;
}

const SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" fill="#00ff00"/></svg>';

const RGBA_WASM = new URL("../assets/svg2rgba_bg.wasm", import.meta.url);
const USVG_WASM = new URL("../assets/svg2usvg_bg.wasm", import.meta.url);
const RGBA_BYTES = await Deno.readFile(RGBA_WASM);
const USVG_BYTES = await Deno.readFile(USVG_WASM);

let instanceCounter = 0;

function freshInstance<T>(path: string): Promise<T> {
  const url = new URL(`../src/${path}`, import.meta.url);
  url.searchParams.set("bytes-instance", String(instanceCounter++));
  return import(url.href) as Promise<T>;
}

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) {
    throw new Error(msg);
  }
}

async function captureError(
  promise: Promise<unknown>,
): Promise<Error> {
  try {
    await promise;
  } catch (error) {
    assert(error instanceof Error, `expected an Error, got ${error}`);
    return error;
  }
  throw new Error("expected the promise to reject");
}

function assertBytesEqual(a: Uint8Array, b: Uint8Array, label: string): void {
  assert(a.length === b.length, `${label}: ${a.length} != ${b.length}`);
  for (let i = 0; i < a.length; i++) {
    assert(a[i] === b[i], `${label}: byte mismatch at offset ${i}`);
  }
}

function startCountingServer(): {
  base: string;
  requests: Map<string, number>;
  shutdown: () => Promise<void>;
} {
  const requests = new Map<string, number>();
  let port = 0;
  const server = Deno.serve(
    { hostname: "127.0.0.1", port: 0, onListen: ({ port: p }) => port = p },
    (req) => {
      const pathname = new URL(req.url).pathname;
      requests.set(pathname, (requests.get(pathname) ?? 0) + 1);
      return new Response("not found", { status: 404 });
    },
  );
  return {
    base: `http://127.0.0.1:${port}`,
    requests,
    shutdown: () => server.shutdown(),
  };
}

Deno.test("bytes and URL initialization render identically", async () => {
  const urlModule = await freshInstance<RgbaModule>("svg2rgba.ts");
  await urlModule.initSvg2rgba({ wasmUrl: RGBA_WASM });
  const viaUrl = await urlModule.svg2rgba(SVG);

  const bytesModule = await freshInstance<RgbaModule>("svg2rgba.ts");
  await bytesModule.initSvg2rgba({ wasmBytes: RGBA_BYTES });
  const viaBytes = await bytesModule.svg2rgba(SVG);

  assertBytesEqual(viaBytes.pixels, viaUrl.pixels, "rgba bytes vs URL");

  const usvgUrlModule = await freshInstance<UsvgModule>("svg2usvg.ts");
  await usvgUrlModule.initSvg2usvg({ wasmUrl: USVG_WASM });
  const usvgViaUrl = await usvgUrlModule.svg2usvg(SVG);

  const usvgBytesModule = await freshInstance<UsvgModule>("svg2usvg.ts");
  await usvgBytesModule.initSvg2usvg({ wasmBytes: USVG_BYTES });
  const usvgViaBytes = await usvgBytesModule.svg2usvg(SVG);

  assertBytesEqual(usvgViaBytes, usvgViaUrl, "usvg bytes vs URL");
});

Deno.test("bytes initialization performs no network fetches", async () => {
  const server = startCountingServer();
  try {
    const module = await freshInstance<RgbaModule>("svg2rgba.ts");
    await module.initSvg2rgba({ wasmBytes: RGBA_BYTES });
    await module.svg2rgba(SVG);
    assert(
      server.requests.size === 0,
      `bytes initialization must not fetch; got ${
        [...server.requests.keys()].join(", ")
      }`,
    );
  } finally {
    await server.shutdown();
  }
});

Deno.test("supplying both sources is rejected before initialization", async () => {
  const module = await freshInstance<RgbaModule>("svg2rgba.ts");
  const error = await captureError(
    module.initSvg2rgba({ wasmUrl: RGBA_WASM, wasmBytes: RGBA_BYTES }),
  );
  assert(
    error.message.includes("wasmUrl") && error.message.includes("wasmBytes"),
    `the conflict must name both sources (got: ${error.message})`,
  );
  assert(
    !error.message.includes("already initialized"),
    `the conflict must be reported before initialization (got: ${error.message})`,
  );
});

Deno.test("source conflict takes precedence after initialization", async () => {
  const module = await freshInstance<RgbaModule>("svg2rgba.ts");
  await module.initSvg2rgba({ wasmBytes: RGBA_BYTES });
  const error = await captureError(
    module.initSvg2rgba({ wasmUrl: RGBA_WASM, wasmBytes: RGBA_BYTES }),
  );
  assert(
    error.message.includes("wasmUrl") && error.message.includes("wasmBytes"),
    `the conflict must name both sources (got: ${error.message})`,
  );
  assert(
    !error.message.includes("already initialized"),
    `the conflict must win over the initialized check (got: ${error.message})`,
  );
});

Deno.test("changing the source after initialization is rejected", async () => {
  const module = await freshInstance<RgbaModule>("svg2rgba.ts");
  await module.initSvg2rgba({ wasmBytes: RGBA_BYTES });
  const error = await captureError(
    module.initSvg2rgba({ wasmUrl: RGBA_WASM }),
  );
  assert(
    error.message.includes("already initialized"),
    `changing the source must be rejected (got: ${error.message})`,
  );

  await module.initSvg2rgba();
  await module.svg2rgba(SVG);
});

Deno.test("empty bytes fail explicitly and never fall back", async () => {
  const server = startCountingServer();
  try {
    const module = await freshInstance<RgbaModule>("svg2rgba.ts");
    const error = await captureError(
      module.initSvg2rgba({ wasmBytes: new Uint8Array(0) }),
    );
    assert(
      error.message.includes("svg2rgba") &&
        error.message.includes("empty") &&
        error.message.includes("0 bytes"),
      `empty bytes need a capability-specific diagnostic (got: ${error.message})`,
    );

    const retry = await captureError(module.initSvg2rgba());
    assert(
      retry.message.includes("empty"),
      `the failed promise must be reused, not reset (got: ${retry.message})`,
    );
    assert(
      server.requests.size === 0,
      "empty bytes must not fall back to the default URL",
    );
  } finally {
    await server.shutdown();
  }
});

Deno.test("invalid bytes expose the compilation failure and never fall back", async () => {
  const server = startCountingServer();
  try {
    const module = await freshInstance<RgbaModule>("svg2rgba.ts");
    const bytes = new Uint8Array([0, 1, 2, 3]);
    const error = await captureError(module.initSvg2rgba({ wasmBytes: bytes }));
    assert(
      error.message.includes("svg2rgba") &&
        error.message.includes("4 bytes"),
      `invalid bytes need a capability and length diagnostic (got: ${error.message})`,
    );
    assert(
      error.message.includes("WebAssembly") || error.message.includes("wasm"),
      `invalid bytes must expose the compilation failure (got: ${error.message})`,
    );
    assert(error.cause instanceof Error, "the compilation error is the cause");

    const render = await captureError(module.svg2rgba(SVG));
    assert(
      render.message.includes("svg2rgba"),
      `the cached failure must come from the same initialization (got: ${render.message})`,
    );
    assert(
      server.requests.size === 0,
      "invalid bytes must not trigger a URL fallback",
    );
  } finally {
    await server.shutdown();
  }
});

Deno.test("a failed URL load is cached without a default fallback", async () => {
  const server = startCountingServer();
  try {
    const module = await freshInstance<RgbaModule>("svg2rgba.ts");
    const missingUrl = `${server.base}/missing_bg.wasm`;
    const first = await captureError(
      module.initSvg2rgba({ wasmUrl: missingUrl }),
    );
    assert(
      first.message.includes(missingUrl) && first.message.includes("404"),
      `the URL diagnostic must retain URL and status (got: ${first.message})`,
    );

    const second = await captureError(module.initSvg2rgba());
    assert(
      second.message.includes(missingUrl),
      `the failed promise must not switch to the default URL (got: ${second.message})`,
    );
    assert(
      server.requests.get("/missing_bg.wasm") === 1,
      "the failed fetch must not be repeated",
    );
  } finally {
    await server.shutdown();
  }
});

Deno.test("repeated calls fetch and instantiate exactly once", async () => {
  const originalInstantiate = WebAssembly.instantiate;
  let instantiations = 0;
  WebAssembly.instantiate = ((...args: unknown[]) => {
    instantiations++;
    return (originalInstantiate as unknown as (...a: unknown[]) => unknown)(
      ...args,
    );
  }) as typeof WebAssembly.instantiate;

  try {
    const module = await freshInstance<RgbaModule>("svg2rgba.ts");
    await module.initSvg2rgba({ wasmBytes: RGBA_BYTES });
    await module.initSvg2rgba();
    await module.svg2rgba(SVG);
    await module.svg2rgba(SVG);
    assert(
      instantiations === 1,
      `the module must be instantiated once, got ${instantiations}`,
    );
  } finally {
    WebAssembly.instantiate = originalInstantiate;
  }
});

Deno.test("a Uint8Array view with a nonzero byteOffset is accepted", async () => {
  const offset = 7;
  const padded = new Uint8Array(RGBA_BYTES.length + offset + 3);
  padded.set(RGBA_BYTES, offset);
  const view = padded.subarray(offset, offset + RGBA_BYTES.length);

  const module = await freshInstance<RgbaModule>("svg2rgba.ts");
  await module.initSvg2rgba({ wasmBytes: view });
  const result = await module.svg2rgba(SVG);
  assert(
    result.pixels.length === result.width * result.height * 4,
    "a nonzero-offset view must render normally",
  );
});
