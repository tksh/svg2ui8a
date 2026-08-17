/**
 * Vendoring scaffold (AGENTS.md §5.1, engineering-playbook §5.1, task.md §5).
 *
 * Usage:
 *   deno run -A scripts/vendor.ts <jsr:@scope/name@version>
 *
 * Produces `vendor/<scope>/<name>/mod.js`, a self-contained copy of the
 * dependency and all of its transitive dependencies, plus a `VENDORED.md`
 * recording the pinned specifier and the reproduction command.
 *
 * Note: `deno vendor` was removed in Deno 2, so a single-file bundle produced
 * by `deno bundle` is used as the self-contained vendored copy. Import it via
 * a `deno.json` import-map entry pointing at the vendored `mod.js`. The
 * baseline dependency set vendors nothing, so this script exists for the
 * future case where a JSR dependency must be pinned.
 */

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");

const specifier = Deno.args[0];
if (!specifier) {
  console.error(
    "usage: deno run -A scripts/vendor.ts <jsr:@scope/name@version>",
  );
  Deno.exit(1);
}

const match = specifier.match(/^jsr:@([^/]+\/[^@]+)@(.+)$/);
if (!match) {
  console.error(
    `unsupported specifier (expected jsr:@scope/name@version): ${specifier}`,
  );
  Deno.exit(1);
}
const [, name, version] = match;

const dir = join(ROOT, "vendor", name);
Deno.mkdirSync(dir, { recursive: true });
const modPath = join(dir, "mod.js");

const bundle = new Deno.Command("deno", {
  args: ["bundle", specifier, "-o", modPath],
  cwd: ROOT,
  stdout: "piped",
  stderr: "piped",
}).outputSync();
if (!bundle.success) {
  const stderr = new TextDecoder().decode(bundle.stderr);
  console.error(`deno bundle failed for ${specifier}: ${stderr}`);
  Deno.exit(1);
}

const check = new Deno.Command("deno", {
  args: ["check", modPath],
  cwd: ROOT,
  stdout: "piped",
  stderr: "piped",
}).outputSync();
if (!check.success) {
  const stderr = new TextDecoder().decode(check.stderr);
  console.error(`vendored bundle failed verification: ${stderr}`);
  Deno.exit(1);
}

const vendoredMd = `# VENDORED — ${name}

- Source: \`${specifier}\`
- Pinned version: \`${version}\`
- Form: self-contained single-file bundle (\`mod.js\`) produced by
  \`deno bundle\`, including all transitive dependencies. \`deno vendor\` was
  removed in Deno 2; a bundle is the self-contained vendoring form.
- Reproduction: \`deno run -A scripts/vendor.ts ${specifier}\`
- Import via a \`deno.json\` import-map entry, e.g.:

\`\`\`json
{
  "imports": {
    "jsr:@${name}": "./vendor/${name}/mod.js"
  }
}
\`\`\`

AGENTS.md §7: \`vendor/\` is not committed except as part of a release
snapshot (engineering-playbook §5.1).
`;

Deno.writeTextFileSync(join(dir, "VENDORED.md"), vendoredMd);
console.log(`vendored ${specifier} → vendor/${name}/mod.js`);
