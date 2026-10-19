/**
 * CDN-free check (AGENTS.md §5.2, engineering-playbook §5.2, task.md §5).
 *
 * Bundles each of the package's entry points and greps the output for
 * `https://` / `http://` references on non-comment lines. A match means a
 * runtime CDN import slipped into the browser bundle; the build must abort.
 *
 * The same bundles are checked for an embedded copy of the Wasm binaries.
 * The package delivers its Wasm as separate `.wasm` files, so a bundle that
 * carries the bytes inline would silently restore the old delivery model.
 *
 * Comment lines are skipped: `deno bundle` injects provenance comments
 * (`// deno:https://jsr.io/...`) and keeps JSDoc URLs in bundled code, which
 * are not runtime imports and must not trip the check.
 *
 * Runs standalone (`deno run -A scripts/check-cdn-free.ts`) or in-process via
 * `runCheck()` from `scripts/build.ts`.
 */

const ROOT = new URL("../", import.meta.url).pathname;

const RUNTIME_URL = /https?:\/\//;

/**
 * Base64 fingerprints of the packaged Wasm binaries. 48 bytes encode to
 * exactly 64 base64 characters, with no padding.
 */
const WASM_FINGERPRINTS = ["svg2rgba_bg.wasm", "svg2usvg_bg.wasm"].map((
  name,
) => ({
  name,
  fingerprint: btoa(
    String.fromCharCode(
      ...Deno.readFileSync(new URL(`../assets/${name}`, import.meta.url))
        .subarray(0, 48),
    ),
  ),
}));

function bundle(entry: string): string {
  const res = new Deno.Command("deno", {
    args: ["bundle", new URL(entry, new URL("../", import.meta.url)).pathname],
    cwd: ROOT,
    stdout: "piped",
    stderr: "piped",
  }).outputSync();
  if (!res.success) {
    const stderr = new TextDecoder().decode(res.stderr);
    throw new Error(`deno bundle failed for ${entry}: ${stderr}`);
  }
  return new TextDecoder().decode(res.stdout);
}

/** True when a non-comment line contains a URL (a forbidden runtime import). */
function hasRuntimeUrl(bundleOutput: string): boolean {
  for (const rawLine of bundleOutput.split("\n")) {
    const line = rawLine.trim();
    if (
      line.startsWith("//") || line.startsWith("*") || line.startsWith("/*")
    ) {
      continue;
    }
    if (RUNTIME_URL.test(line)) {
      return true;
    }
  }
  return false;
}

export function runCheck(): void {
  const entries = [
    "src/mod.ts",
    "src/svg2rgba.ts",
    "src/svg2usvg.ts",
  ];
  let failed = false;
  for (const entry of entries) {
    const output = bundle(entry);
    if (hasRuntimeUrl(output)) {
      console.error(`CDN-free check FAILED for ${entry}: runtime URL found.`);
      failed = true;
    } else {
      console.log(`CDN-free check passed for ${entry}`);
    }
    for (const { name, fingerprint } of WASM_FINGERPRINTS) {
      if (output.includes(fingerprint)) {
        console.error(
          `CDN-free check FAILED for ${entry}: ${name} is embedded as base64.`,
        );
        failed = true;
      }
    }
  }
  if (failed) {
    throw new Error("CDN-free check failed; build aborted.");
  }
}

if (import.meta.main) {
  runCheck();
  console.log("CDN-free check passed.");
}
