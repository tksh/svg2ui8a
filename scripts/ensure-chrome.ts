/**
 * Pinned Chrome acquisition for Wasm browser tests (task.md §8.2).
 *
 * Fetches the pinned Chrome build via Astral's hermetic fetcher, verifies
 * the pin, and prints the resolved cache directory, binary path, and
 * version. No `sudo`, no `apt`, no `npm:` import — entirely JSR/Deno.
 *
 * Run: `deno task ensure:chrome`  (or `deno run -A scripts/ensure-chrome.ts`)
 * The first run downloads ~100–150 MB to `~/.cache/astral`; subsequent runs
 * reuse the cached binary. Set `ASTRAL_BIN_PATH` to override the binary
 * (see `docs/plans/wasm-browser-tests.md` §7 fallback), not used in §8.2.
 */

import {
  getBinary,
  getDefaultCachePath,
  launch,
  SUPPORTED_VERSIONS,
} from "@astral/astral";

const EXPECTED_CHROME = "125.0.6400.0";

function fail(msg: string): never {
  console.error(`ensure-chrome: ${msg}`);
  Deno.exit(1);
}

if (SUPPORTED_VERSIONS.chrome !== EXPECTED_CHROME) {
  fail(
    `SUPPORTED_VERSIONS.chrome is ${
      JSON.stringify(SUPPORTED_VERSIONS.chrome)
    } ` +
      `but expected ${JSON.stringify(EXPECTED_CHROME)} — ` +
      `Astral pin drifted; update docs/plans/wasm-browser-tests.md and this script.`,
  );
}

const cache = getDefaultCachePath();
console.log(`Astral: jsr:@astral/astral@0.5.6`);
console.log(`SUPPORTED_VERSIONS: ${JSON.stringify(SUPPORTED_VERSIONS)}`);
console.log(`cache: ${cache}`);
console.log(`expected Chrome: ${EXPECTED_CHROME}`);

// Ensure the pinned binary is present (downloads on first run, no sudo/apt).
let binary: string;
try {
  binary = await getBinary("chrome");
} catch (e) {
  fail(`getBinary(chrome) failed: ${e}`);
}

console.log(`binary: ${binary}`);

try {
  const stat = await Deno.stat(binary);
  if (!stat.isFile) fail(`binary path is not a file: ${binary}`);
  console.log(`binary size: ${stat.size} bytes`);
} catch (e) {
  fail(`binary not found after fetch: ${e}`);
}

// Verify the binary actually launches headless and speaks CDP, then close.
// This is the "verify it runs" part of §8.2 — no test harness yet.
let browser;
try {
  browser = await launch({ headless: true, product: "chrome" });
  console.log(`launch: ok (ws endpoint acquired)`);
  // A trivial evaluate proves the browser is responsive; not a Wasm test.
  const page = await browser.newPage("about:blank");
  const v = await page.evaluate(() => navigator.userAgent);
  console.log(`userAgent: ${String(v).slice(0, 120)}`);
  await page.close();
} catch (e) {
  fail(`Chrome launch/verify failed (missing shared libs? see plan §12): ${e}`);
} finally {
  if (browser) {
    try {
      await browser.close();
    } catch {
      // ignore close errors
    }
    console.log(`close: ok`);
  }
}

console.log(`ensure-chrome: verified Chrome ${EXPECTED_CHROME} at ${binary}`);
