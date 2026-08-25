/**
 * CDN-free check (AGENTS.md §5.2, engineering-playbook §5.2, task.md §5).
 *
 * Bundles each of the package's four subpath entry points (root + three
 * leaves) and greps the
 * output for `https://` / `http://` references on non-comment lines. A match
 * means a runtime CDN import slipped into the browser bundle; the build must
 * abort.
 *
 * Comment lines are skipped: `deno bundle` injects provenance comments
 * (`// deno:https://jsr.io/...`) and keeps JSDoc URLs in bundled code, which
 * are not runtime imports and must not trip the check.
 *
 * Runs standalone (`deno run -A scripts/check-cdn-free.ts`) or in-process via
 * `runCheck()` from `scripts/build.ts`.
 */

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");

const RUNTIME_URL = /https?:\/\//;

function bundle(entry: string): string {
  const res = new Deno.Command("deno", {
    args: ["bundle", join(ROOT, entry)],
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
    "src/stln.ts",
    "src/stln-rgba.ts",
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
  }
  if (failed) {
    throw new Error("CDN-free check failed; build aborted.");
  }
}

if (import.meta.main) {
  runCheck();
  console.log("CDN-free check passed.");
}
