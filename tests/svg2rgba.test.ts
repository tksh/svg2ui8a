// Deno tests for the svg2rgba one-shot subpath (rename plan Phase 3, §3.4).
//
// svg2rgba takes an SVG string directly and returns pixels: no CBOR payload,
// no cacheable-bytes contract. Determinism is asserted as "same input → same
// pixels" only.

import {
  type RgbaResult,
  svg2rgba,
  type Svg2RgbaOptions,
} from "../src/svg2rgba.ts";
import { svg2stln } from "../src/stln.ts";
import { stln2rgba } from "../src/stln-rgba.ts";

const LINE_SVG =
  `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">
  <path d="M 5 0 L 5 10" stroke="#ff0000" stroke-width="10" fill="none"/>
</svg>`;

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
    [{ width: 40 }, 40, 10],
    [{ height: 30 }, 10, 30],
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

Deno.test("svg2rgba renders general-purpose SVG outside the Straightlines subset", async () => {
  const rect = await svg2rgba(
    `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#00ff00"/></svg>`,
  );
  assert(rect.pixels.length > 0, "rect renders");
  const curve = await svg2rgba(
    `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><path d="M 1 5 Q 5 0 9 5" stroke="#000" stroke-width="1" fill="none"/></svg>`,
  );
  assert(curve.pixels.length > 0, "curves render");
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

Deno.test("svg2rgba and the stln pipeline agree on subset input at natural size", async () => {
  // Same rendering machinery underneath; sanity-check one shared input.
  const subsetSvg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" shape-rendering="geometricPrecision">
  <path d="M 5 0 L 5 10" stroke="#ff0000" stroke-width="10" fill="none"/>
</svg>`;
  const oneShot = await svg2rgba(subsetSvg);
  const piped = await stln2rgba(await svg2stln(subsetSvg));
  assert(
    oneShot.width === piped.width && oneShot.height === piped.height,
    "dimensions must match",
  );
  assert(
    oneShot.pixels.every((byte, i) => byte === piped.pixels[i]),
    "both paths must render identical pixels for identical input",
  );
});
