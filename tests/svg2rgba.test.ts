// Deno tests for the svg2rgba subpath (0.3.0).

import {
  type RgbaResult,
  svg2rgba,
  type Svg2RgbaOptions,
} from "../src/svg2rgba.ts";

const LINE_SVG =
  `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">
  <path d="M 5 0 L 5 10" stroke="#ff0000" stroke-width="10" fill="none"/>
</svg>`;

const FRACTIONAL_CANVAS_SVG =
  `<svg xmlns="http://www.w3.org/2000/svg" width="27.9" height="31"><rect width="27.9" height="31" fill="#ff0000"/></svg>`;

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

async function assertRejects(
  promise: Promise<unknown>,
  msg: string,
): Promise<void> {
  try {
    await promise;
  } catch {
    return;
  }
  throw new Error(msg);
}

Deno.test("svg2rgba resolves to a well-formed RgbaResult", async () => {
  const pending = svg2rgba(LINE_SVG);
  assert(pending instanceof Promise, "svg2rgba must return a Promise");
  const result = await pending;
  assert(
    result.width === 10 && result.height === 10,
    `natural size must be 10x10, got ${result.width}x${result.height}`,
  );
  assert(result.alphaMode === "straight", "default alphaMode must be straight");
  assert(result.pixels instanceof Uint8Array, "pixels must be Uint8Array");
  assert(
    result.pixels.length === result.width * result.height * 4,
    "pixels.length must equal width*height*4",
  );
});

Deno.test("svg2rgba honors sizing rules and alphaMode", async () => {
  const cases: Array<[Svg2RgbaOptions | undefined, number, number]> = [
    [undefined, 10, 10],
    [{}, 10, 10],
    [{ width: 40 }, 40, 40],
    [{ height: 30 }, 30, 30],
    [{ width: 40, height: 30 }, 40, 30],
  ];
  for (const [options, w, h] of cases) {
    const result: RgbaResult = await svg2rgba(LINE_SVG, options);
    assert(
      result.width === w && result.height === h,
      `sizing rule broken: expected ${w}x${h}, got ${result.width}x${result.height}`,
    );
  }
  const premultiplied = await svg2rgba(LINE_SVG, {
    width: 20,
    height: 20,
    alphaMode: "premultiplied",
  });
  assert(premultiplied.alphaMode === "premultiplied", "alphaMode reflected");
});

Deno.test("svg2rgba preserves fractional natural aspect ratios", async () => {
  const heightOnly = await svg2rgba(FRACTIONAL_CANVAS_SVG, { height: 256 });
  assert(
    heightOnly.width === 230 && heightOnly.height === 256,
    `height-only fractional sizing must be 230x256, got ${heightOnly.width}x${heightOnly.height}`,
  );

  const widthOnly = await svg2rgba(FRACTIONAL_CANVAS_SVG, { width: 279 });
  assert(
    widthOnly.width === 279 && widthOnly.height === 310,
    `width-only fractional sizing must be 279x310, got ${widthOnly.width}x${widthOnly.height}`,
  );

  const natural = await svg2rgba(FRACTIONAL_CANVAS_SVG);
  assert(
    natural.width === 28 && natural.height === 31,
    `fractional natural sizing must be 28x31, got ${natural.width}x${natural.height}`,
  );
});

Deno.test("svg2rgba renders general-purpose SVG (rects, curves, gradients)", async () => {
  const rect = await svg2rgba(
    `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#00ff00"/></svg>`,
  );
  assert(rect.pixels.length > 0, "rect renders");
  const curve = await svg2rgba(
    `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><path d="M 1 5 Q 5 0 9 5" stroke="#000" stroke-width="1" fill="none"/></svg>`,
  );
  assert(curve.pixels.length > 0, "curves render");
  const gradient = await svg2rgba(
    `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><defs><linearGradient id="g" x1="0" y1="0" x2="10" y2="0"><stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></linearGradient></defs><rect width="10" height="10" fill="url(#g)"/></svg>`,
  );
  assert(gradient.pixels.length > 0, "gradients render");
});

Deno.test("same SVG produces identical pixels (determinism)", async () => {
  const a = await svg2rgba(LINE_SVG);
  const b = await svg2rgba(LINE_SVG);
  assert(
    a.pixels.length === b.pixels.length &&
      a.pixels.every((byte, i) => byte === b.pixels[i]),
    "repeated calls must produce identical pixels",
  );
});

Deno.test("malformed / text / image SVG reject the svg2rgba promise", async () => {
  await assertRejects(svg2rgba("<svg>unclosed"), "malformed SVG must reject");
  await assertRejects(
    svg2rgba("<svg><text>hi</text></svg>"),
    "<text> content must reject",
  );
  await assertRejects(
    svg2rgba('<svg><image href="x.png"/></svg>'),
    "<image> content must reject",
  );
});

Deno.test("svg2rgba result is a plain object, not a wasm-bindgen class instance", async () => {
  const result = await svg2rgba(LINE_SVG);

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

Deno.test("svg2rgba result stays valid after a second call reuses the wasm instance", async () => {
  const barSvg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"/></svg>`;

  const first = await svg2rgba(barSvg, { width: 10, height: 10 });
  const firstPixels = first.pixels;
  const firstLen = firstPixels.length;
  const firstSum = checksum(firstPixels);

  const second = await svg2rgba(barSvg, { width: 20, height: 10 });
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

Deno.test("Svg2RgbaOptions accepts a plain object literal, not a wasm class instance", async () => {
  const full = await svg2rgba(LINE_SVG, {
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

  const partial = await svg2rgba(LINE_SVG, { width: 100 });
  assert(partial.width === 100, "width-only option must be honored");
  assert(
    partial.height === 100,
    "unset height must preserve the natural SVG aspect ratio",
  );
  assert(
    partial.alphaMode === "straight",
    "unset alphaMode must default to straight",
  );

  const empty = await svg2rgba(LINE_SVG, {});
  const omitted = await svg2rgba(LINE_SVG);
  assert(
    empty.width === omitted.width && empty.height === omitted.height &&
      empty.alphaMode === omitted.alphaMode &&
      checksum(empty.pixels) === checksum(omitted.pixels),
    "an empty options object must behave exactly like omitting options",
  );
});

Deno.test("public subpaths export nothing whose name starts with __wasm_", async () => {
  const modules: Array<[string, Record<string, unknown>]> = [
    ["src/svg2usvg.ts", await import("../src/svg2usvg.ts")],
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
