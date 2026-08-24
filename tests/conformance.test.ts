// Deno tests for the §3.4 required behaviors not exercised by
// tests/usvg2rgba.test.ts: producer output shape, JS-side determinism,
// rejection of malformed inputs, the end-to-end flow, the `.cbor` file
// round trip, and init idempotency across the two Wasm modules.

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { usvg2rgba, type Usvg2RgbaOptions } from "../src/rgba.ts";
import { svg2usvg } from "../src/usvg.ts";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const FIXTURE_SVG_PATH = join(ROOT, "tests/fixtures/straightlines-sample.svg");
const FIXTURE_CBOR_PATH = join(
  ROOT,
  "tests/fixtures/straightlines-sample.cbor",
);

const SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">
  <rect width="10" height="10" fill="#ff0000"/>
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

function assertBytesEqual(a: Uint8Array, b: Uint8Array, msg: string): void {
  assert(a.length === b.length, `${msg}: length ${a.length} != ${b.length}`);
  for (let i = 0; i < a.length; i++) {
    assert(a[i] === b[i], `${msg}: byte mismatch at offset ${i}`);
  }
}

Deno.test("svg2usvg resolves to a non-empty Uint8Array", async () => {
  const bytes = await svg2usvg(SVG);
  assert(bytes instanceof Uint8Array, "svg2usvg must return a Uint8Array");
  assert(bytes.length > 0, "svg2usvg must return non-empty bytes");
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

Deno.test("end-to-end flow produces correctly sized RGBA for every sizing rule", async () => {
  const bytes = await svg2usvg(SVG);

  const cases: Array<[Usvg2RgbaOptions | undefined, number, number]> = [
    [undefined, 10, 10],
    [{}, 10, 10],
    [{ width: 40 }, 40, 10],
    [{ height: 30 }, 10, 30],
    [{ width: 40, height: 30 }, 40, 30],
  ];
  for (const [options, w, h] of cases) {
    const result = await usvg2rgba(bytes, options);
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
    assert(
      result.alphaMode === "straight",
      "alphaMode must default to straight",
    );
  }

  const premultiplied = await usvg2rgba(bytes, {
    width: 40,
    height: 30,
    alphaMode: "premultiplied",
  });
  assert(
    premultiplied.alphaMode === "premultiplied",
    "alphaMode option must be reflected in the result",
  );
  assert(
    premultiplied.pixels.length === 40 * 30 * 4,
    "premultiplied pixels must have the same size as straight pixels",
  );
});

Deno.test(".cbor file round trip: written bytes rasterize identically", async () => {
  const bytes = await svg2usvg(SVG);
  const path = await Deno.makeTempFile({
    prefix: "svg2ui8a_",
    suffix: ".cbor",
  });
  try {
    await Deno.writeFile(path, bytes);

    const reread = await Deno.readFile(path);
    assert(
      reread instanceof Uint8Array,
      "Deno.readFile must yield a Uint8Array",
    );
    assertBytesEqual(
      reread,
      bytes,
      "file contents must match the svg2usvg output",
    );

    const viaFile = await usvg2rgba(reread, { width: 40, height: 30 });
    const inMemory = await usvg2rgba(bytes, { width: 40, height: 30 });
    assert(
      viaFile.width === inMemory.width && viaFile.height === inMemory.height &&
        viaFile.alphaMode === inMemory.alphaMode,
      "the .cbor file payload must produce the same RgbaResult metadata",
    );
    assertBytesEqual(
      viaFile.pixels,
      inMemory.pixels,
      "the .cbor file payload must produce identical pixels",
    );
  } finally {
    await Deno.remove(path);
  }
});

Deno.test("init is idempotent and the two Wasm modules do not cross-interfere", async () => {
  const first = await svg2usvg(SVG);
  const second = await svg2usvg(SVG);
  assertBytesEqual(
    first,
    second,
    "a repeated svg2usvg call must not re-initialize or corrupt output",
  );

  const renderA = await usvg2rgba(first);
  const renderB = await usvg2rgba(first);
  assert(
    renderB.width === 10 && renderB.height === 10,
    "a repeated usvg2rgba call must not re-initialize or change dimensions",
  );
  assertBytesEqual(
    renderA.pixels,
    renderB.pixels,
    "repeated usvg2rgba calls must produce identical pixels",
  );

  const third = await svg2usvg(SVG);
  assertBytesEqual(
    first,
    third,
    "svg2usvg must stay correct after usvg2rgba calls (no cross-init interference)",
  );

  const renderC = await usvg2rgba(first, { width: 20, height: 10 });
  assert(
    renderC.width === 20 && renderC.height === 10,
    "usvg2rgba must stay correct after further svg2usvg calls (no cross-init interference)",
  );
});

Deno.test("malformed usvg payloads reject the usvg2rgba promise", async () => {
  const notCbor = new TextEncoder().encode("this is not a usvg payload");
  await assertRejects(
    usvg2rgba(notCbor),
    "a garbage payload must reject the usvg2rgba promise",
  );
  await assertRejects(
    usvg2rgba(new Uint8Array()),
    "an empty payload must reject the usvg2rgba promise",
  );

  const bytes = await svg2usvg(SVG);
  const truncated = bytes.slice(0, Math.floor(bytes.length / 2));
  await assertRejects(
    usvg2rgba(truncated),
    "a truncated payload must reject the usvg2rgba promise",
  );
});

// ---------------------------------------------------------------------------
// Straightlines fixture (task.md §13): the committed .cbor must stay a
// faithful, standalone representation of the committed .svg.
// ---------------------------------------------------------------------------

const FIXTURE_SVG = await Deno.readTextFile(FIXTURE_SVG_PATH);
const FIXTURE_CBOR = await Deno.readFile(FIXTURE_CBOR_PATH);

Deno.test("fixture: svg2usvg output is byte-identical to the committed .cbor", async () => {
  const fresh = await svg2usvg(FIXTURE_SVG);
  assertBytesEqual(
    fresh,
    FIXTURE_CBOR,
    "svg2usvg(straightlines-sample.svg) must reproduce straightlines-sample.cbor byte-for-byte; run `deno task fixtures:regen` if the format intentionally changed",
  );
});

Deno.test("fixture: committed .cbor rasterizes at its natural size as a standalone file", async () => {
  const result = await usvg2rgba(FIXTURE_CBOR);
  assert(
    result.width === 31 && result.height === 31,
    `expected natural size 31x31, got ${result.width}x${result.height}`,
  );
  assert(
    result.pixels.length === 31 * 31 * 4,
    `pixels.length must be ${31 * 31 * 4}, got ${result.pixels.length}`,
  );
  assert(result.alphaMode === "straight", "default alphaMode must be straight");
  assert(
    result.pixels.some((byte, i) => i % 4 === 3 && byte !== 0),
    "the fixture render must contain visible (non-transparent) pixels",
  );
});

Deno.test("fixture: .cbor from disk and fresh svg2usvg output rasterize identically", async () => {
  const fromDisk = await usvg2rgba(FIXTURE_CBOR);
  const inMemory = await usvg2rgba(await svg2usvg(FIXTURE_SVG));
  assert(
    fromDisk.width === inMemory.width && fromDisk.height === inMemory.height &&
      fromDisk.alphaMode === inMemory.alphaMode,
    "fixture payloads must produce identical RgbaResult metadata",
  );
  assertBytesEqual(
    fromDisk.pixels,
    inMemory.pixels,
    "fixture payloads must produce identical pixels",
  );
});
