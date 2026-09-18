// Deno tests for 0.3.0 cross-cutting behaviors: svg2usvg output shape,
// determinism, rejection, end-to-end flow, and init idempotency.

import { svg2rgba } from "../src/svg2rgba.ts";
import { svg2usvg } from "../src/svg2usvg.ts";

const SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">
  <path d="M 5 0 L 5 10" stroke="#ff0000" stroke-width="10" fill="none"/>
</svg>`;

const FRACTIONAL_SVG =
  `<svg xmlns="http://www.w3.org/2000/svg" width="27.9" height="31"><rect width="27.9" height="31" fill="#ff0000"/></svg>`;

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) {
    throw new Error(msg);
  }
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

function assertBytesEqual(a: Uint8Array, b: Uint8Array, msg: string): void {
  assert(a.length === b.length, `${msg}: length ${a.length} != ${b.length}`);
  for (let i = 0; i < a.length; i++) {
    assert(a[i] === b[i], `${msg}: byte mismatch at offset ${i}`);
  }
}

Deno.test("svg2usvg resolves to a non-empty Uint8Array of valid XML", async () => {
  const bytes = await svg2usvg(SVG);
  assert(bytes instanceof Uint8Array, "svg2usvg must return a Uint8Array");
  assert(bytes.length > 0, "svg2usvg must return non-empty bytes");
  const text = new TextDecoder().decode(bytes);
  assert(text.includes("<svg"), "svg2usvg output must contain <svg");
});

Deno.test("same SVG string produces identical usvg bytes (determinism)", async () => {
  const first = await svg2usvg(SVG);
  const second = await svg2usvg(SVG);
  assertBytesEqual(
    first,
    second,
    "repeated svg2usvg calls must produce identical bytes",
  );
});

Deno.test("malformed SVG rejects the svg2usvg promise", async () => {
  await assertRejects(
    svg2usvg("<svg>unclosed"),
    "malformed SVG must reject the svg2usvg promise",
  );
});

Deno.test("text / image SVG rejects svg2usvg", async () => {
  await assertRejects(
    svg2usvg("<svg><text>hi</text></svg>"),
    "<text> must reject",
  );
  await assertRejects(
    svg2usvg('<svg><image href="x.png"/></svg>'),
    "<image> must reject",
  );
});

Deno.test("svg2usvg output is re-parseable by svg2usvg and renders via svg2rgba", async () => {
  const usvgBytes = await svg2usvg(SVG);
  const usvgText = new TextDecoder().decode(usvgBytes);
  // Re-parse the normalized usvg XML through svg2rgba directly
  const viaUsvg = await svg2rgba(usvgText);
  const direct = await svg2rgba(SVG);
  assert(
    viaUsvg.width === direct.width && viaUsvg.height === direct.height,
    "usvg bytes re-parsed must produce same dimensions",
  );
  assertBytesEqual(
    viaUsvg.pixels,
    direct.pixels,
    "usvg bytes must render identically through svg2rgba",
  );
  for (
    const key of [
      "absBoundingBox",
      "absStrokeBoundingBox",
      "absLayerBoundingBox",
    ] as const
  ) {
    assert(
      Object.hasOwn(viaUsvg, key) && Object.hasOwn(direct, key),
      `${key} must be present on both render results`,
    );
    assert(
      viaUsvg[key] !== undefined && direct[key] !== undefined,
      `${key} must never be undefined`,
    );
  }
});

Deno.test("end-to-end svg2rgba sizing rules", async () => {
  const bytes = await svg2usvg(SVG);
  assert(bytes instanceof Uint8Array, "precondition: svg2usvg succeeded");

  const cases: Array<
    [{ width?: number; height?: number } | undefined, number, number]
  > = [
    [undefined, 10, 10],
    [{}, 10, 10],
    [{ width: 40 }, 40, 40],
    [{ height: 30 }, 30, 30],
    [{ width: 40, height: 30 }, 40, 30],
  ];
  for (const [options, w, h] of cases) {
    const result = await svg2rgba(SVG, options as never);
    assert(
      result.width === w && result.height === h,
      `sizing rule broken for options ${
        JSON.stringify(options) ?? "omitted"
      }: expected ${w}x${h}, got ${result.width}x${result.height}`,
    );
    assert(
      result.pixels.length === w * h * 4,
      `pixels.length must equal width*height*4 (${w}*${h}*4), got ${result.pixels.length}`,
    );
  }
});

Deno.test("fractional natural dimensions use the natural aspect ratio", async () => {
  const heightOnly = await svg2rgba(FRACTIONAL_SVG, { height: 256 });
  assert(
    heightOnly.width === 230 && heightOnly.height === 256,
    "height-only fractional dimensions must be 230x256",
  );
  assert(
    Math.abs(heightOnly.naturalWidth - 27.9) < 0.00001 &&
      heightOnly.naturalHeight === 31,
    "natural dimensions must remain fractional and unscaled",
  );

  const widthOnly = await svg2rgba(FRACTIONAL_SVG, { width: 279 });
  assert(
    widthOnly.width === 279 && widthOnly.height === 310,
    "width-only fractional dimensions must be 279x310",
  );
});

Deno.test("init is idempotent and the two Wasm modules do not cross-interfere", async () => {
  const first = await svg2usvg(SVG);
  const second = await svg2usvg(SVG);
  assertBytesEqual(
    first,
    second,
    "a repeated svg2usvg call must not re-initialize or corrupt output",
  );

  const renderA = await svg2rgba(SVG);
  const renderB = await svg2rgba(SVG);
  assertBytesEqual(
    renderA.pixels,
    renderB.pixels,
    "repeated svg2rgba calls must produce identical pixels",
  );

  const third = await svg2usvg(SVG);
  assertBytesEqual(
    first,
    third,
    "svg2usvg must stay correct after svg2rgba calls (no cross-init interference)",
  );

  const renderC = await svg2rgba(SVG, { width: 20, height: 10 });
  assert(
    renderC.width === 20 && renderC.height === 10,
    "svg2rgba must stay correct after further svg2usvg calls",
  );
});

Deno.test("malformed usvg input to svg2rgba rejects", async () => {
  await assertRejects(
    svg2rgba("<svg>unclosed"),
    "malformed SVG must reject svg2rgba",
  );
});
