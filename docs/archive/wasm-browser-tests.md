# Plan: Browser-verified Wasm tests via Astral (CDP, headless Chrome)

Task: `task.md` §8, step **8.1 — Plan only**. This document satisfies
`engineering-playbook.md` §4 step 2 and `task.md` §8.1. No code is created in
this step; approval is required before §8.2.

---

## 1. Authorization — quoted

### 1.1 `engineering-playbook.md` §3.3 (authorizing section)

> ### 3.3 Rust Wasm tests (`wasm-pack test` or equivalent)
>
> Run via `deno task test:wasm`. These exercise the Wasm binaries in a headless
> browser, ensuring the JS boundary works as expected.
>
> Required test cases for each Wasm:
>
> - The TS entry point returns a `Promise<Uint8Array>` / `Promise<RgbaResult>`
>   as appropriate.
> - The output is identical to the output of the Rust-native `core` function for
>   the same input.

The parenthetical **"or equivalent"** in the section title is the hook this plan
relies on. `wasm-pack test` is one way to satisfy the "headless browser"
requirement; Astral-driven CDP is adopted as that _equivalent_.

### 1.2 `engineering-playbook.md` §1 (orchestration expectation)

> | `deno task test:wasm` | Run the Wasm tests in a headless browser. |

### 1.3 `AGENTS.md` §5 (dependency discipline) and `project-constitution.md` §3.6

> `npm:` specifiers are not allowed in the package code. They are a
> consumer-side concern, not a package concern. (`AGENTS.md` §5.1)
>
> Do not target Node.js. The package is for Deno and the browser.
> (`project-constitution.md` §3.6)

§3.6 scopes package code, not dev tooling, but the plan honors its spirit for
test tooling as well — no npm packages anywhere.

### 1.4 `engineering-playbook.md` §3.4 — why a second Wasm layer is still needed

§3.4's Deno-layer tests and §7's `scripts/test-wasm.ts` run entirely inside the
Deno runtime. That verifies output _correctness_ against the Rust-native core
but never exercises the **browser module-loading and Wasm-instantiation path**
for the shipped entry points (`src/usvg.ts`, `src/rgba.ts`). §3.3 requires that
path to be exercised in a real headless browser. This plan closes that literal
gap.

---

## 2. Decision — Astral + CDP as the §3.3 equivalent (do not re-litigate)

**Adopted:** `jsr:@astral/astral` driving headless Chrome via **CDP** (Chrome
DevTools Protocol), no chromedriver, no WebDriver bridge, no npm.

- **Canonical import path (verified 2026-08-22):** `jsr:@astral/astral` at
  pinned version **`0.5.6`** (`deno info jsr:@astral/astral`,
  `SUPPORTED_VERSIONS = { chrome: "125.0.6400.0", firefox: "116.0" }`).
  `deno info` shows **184 unique dependencies, 2.57 MB total, zero `npm:`
  specifiers**.
- **Why Astral / CDP:**
  - Real headless browser execution — satisfies §3.3 literally, unlike the
    current Deno-only `scripts/test-wasm.ts`.
  - CDP talks to Chrome directly; no `chromedriver` binary and no
    Chrome/chromedriver version-skew to manage.
  - JSR-native (tier-1 per `AGENTS.md` §5.1), no npm (Puppeteer, Playwright)
    needed.
  - Built-in pinned Chrome fetcher — reproducible across machines and CI.
- **Evidence:** A spike outside the repo (`/tmp/opencode/astral-spike.ts`,
  `launch({ headless: true, product: "chrome", cache:
  "/tmp/opencode/astral-cache" })` +
  `page.evaluate("1+1")`) downloaded `chrome@125.0.6400.0` and returned `2` on
  this WSL2 host without `sudo`, `apt`, or any npm install.

This decision was made before this plan per `task.md` §8 prologue; the plan
records it.

#### Tradeoff — older Chrome pin accepted

Astral `0.5.6`'s pin (`125.0.6400.0`, May 2024) is older than current stable
Chrome (`151.x` on this WSL2 host and on the `ubuntu-latest` runner image as of
2026-08-22). The older, stable, hermetically-fetched build is accepted in
exchange for staying entirely within the Deno/JSR ecosystem with no `npm:`
dependency. Deno-first outweighs having the newest possible Chrome for this test
layer. `npm:playwright` (or any other `npm:` package) is not added in this task
even though Deno 2's npm compatibility layer would allow it.

#### Fallback — if the pinned build becomes unavailable (doc only, not implemented)

If `125.0.6400.0` ever fails to launch or becomes unavailable from Chrome for
Testing storage, the documented recovery path for a **future** task is to keep
Astral and point it at a locally-available newer Chrome via its `path` launch
option (e.g. a separately obtained Chrome for Testing stable build), not to
switch to a different browser-automation library. This note is documentation
only; no fallback is implemented in §8.

---

## 3. Scope — files touched across the full §8 (8.2–8.8)

No file below is created or modified in §8.1. The list is the complete touch set
for the eventual §8.2–§8.8 implementation; it is the authority for "smallest
possible diff" review.

| File                                                                                                               | Action in full §8                                                                                                                                                               | crates / artifacts affected                                                                                                                                                                            |
| ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `docs/plans/wasm-browser-tests.md`                                                                                 | **NEW in §8.1** — this plan                                                                                                                                                     | none (working artifact, `AGENTS.md` §2 exempt)                                                                                                                                                         |
| `scripts/ensure-chrome.ts` _or_ `scripts/test-wasm-browser.ts` with a `--ensure-only` / `--print-chrome-path` mode | **NEW in §8.2** — fetches pinned Chrome via Astral, prints resolved binary path/version, exits 0; no harness yet                                                                | none in §8.2 (helper only)                                                                                                                                                                             |
| `scripts/test-wasm-browser.ts`                                                                                     | **NEW in §8.3–§8.5** — incremental harness: §8.3 one entry point (`src/usvg.ts` → `svg2usvg`), §8.4 second entry point (`src/rgba.ts` → `usvg2rgba`), §8.5 remaining §3.3 cases | exercises **both** Wasm artifacts (`assets/svg2usvg_bg.wasm`, `assets/usvg2rgba_bg.wasm`) via their shipped TS entry points `src/usvg.ts` and `src/rgba.ts`; does **not** modify any `crates/*` source |
| `deno.json`                                                                                                        | **EDIT in §8.6** — add task(s) for the browser layer and wire into `deno task test` pipeline                                                                                    | build-task only; no crate change                                                                                                                                                                       |
| `.gitignore`                                                                                                       | **EDIT (if needed) in §8.2/§8.6** — ignore the user-space Chrome cache dir if it lives under the repo; no edit if cache stays under `~/.cache` (see §7)                         | none                                                                                                                                                                                                   |
| `docs/engineering-playbook.md` §3.3                                                                                | **EDIT in §8.7** — state Astral/CDP as the fixed interpretation of "or equivalent"                                                                                              | docs only                                                                                                                                                                                              |
| `CHANGELOG.md`                                                                                                     | **EDIT in §8.7** — add `Unreleased` entry for the browser-verified layer                                                                                                        | docs only                                                                                                                                                                                              |
| `task.md` §8 checklist                                                                                             | **EDIT incrementally** — check off 8.1→8.8 as each step lands                                                                                                                   | none                                                                                                                                                                                                   |

**Not touched:** any file under `crates/intermediate`, `crates/svg2usvg`,
`crates/usvg2rgba` (no crate source change), `assets/*.wasm` (artifacts are
_exercised_, not rebuilt by this track), `vendor/` (Astral stays a JSR dev
dependency; it is not vendored as package code).

---

## 4. What the browser harness will test and why it is distinct

### 4.1 What `scripts/test-wasm.ts` already does (§7)

`scripts/test-wasm.ts` (committed in §7) runs inside Deno, imports
`src/usvg.ts`/`src/rgba.ts` directly, and asserts that each Wasm entry point
returns the correct `Promise` type and that its output is byte-identical to the
corresponding Rust-native `core` output via the per-crate `dump_core` binaries.
It is correct and stays.

### 4.2 What the browser harness adds (§8.3–§8.5)

The browser harness verifies the **actual browser module-loading and
Wasm-instantiation path** for the _shipped_ entry points — something a Deno-only
harness cannot exercise:

- Loads `src/usvg.ts` and `src/rgba.ts` **inside headless Chrome** via
  Astral/CDP (real browser `import`, real `WebAssembly.instantiate`, real
  `Promise` microtask queue).
- Calls `svg2usvg(svg: string)` and `usvg2rgba(usvg, options?)` through those
  loaded modules and asserts on the resolved values **in the browser context**
  (then marshaled back for the process exit code).
- Therefore exercises the glue that `scripts/build.ts` generates (base64
  inlining, `__wbg_init` call, no runtime fetch) and the per-module
  `initialized` flag as the browser sees it.

The two layers are complementary, not redundant: `test-wasm.ts` proves
byte-identity against native core inside Deno; the browser harness proves the
same bytes are produced when the same shipped modules are loaded by a real
browser.

### 4.3 Execution model (sketch, not code)

1. Ensure pinned Chrome exists (see §7).
2. `Astral.launch({ product: "chrome", headless: true, cache: <pinned
   cache dir> })`
   — Astral resolves the binary, spawns Chrome, connects over CDP, returns a
   `Browser`/`Page` handle.
3. Page loads a minimal HTML fixture that `import`s the two shipped entry
   points.
4. `page.evaluate` drives the test cases (§5) and returns structured results;
   the Deno process asserts and sets the exit code.

No `chromedriver`, no `npm:` import.

---

## 5. Required test cases — §8.5 will cover `engineering-playbook.md` §3.3

Quoted from §3.3:

> Required test cases for each Wasm:
>
> - The TS entry point returns a `Promise<Uint8Array>` / `Promise<RgbaResult>`
>   as appropriate.
> - The output is identical to the output of the Rust-native `core` function for
>   the same input.

§8.5 will implement exactly those, **inside the browser harness**:

- **§3.3 case 1 — Promise type:** `svg2usvg(...) instanceof Promise` and
  resolves to `Uint8Array`; `usvg2rgba(...) instanceof Promise` and resolves to
  `RgbaResult` — asserted from within the browser context (then reported back).
- **§3.3 case 2 — Output identity vs native core:** For the same SVG / `usvg`
  inputs used by `scripts/test-wasm.ts`, the browser-resolved `Uint8Array`
  (producer) and `RgbaResult.pixels` / `width` / `height` / `alphaMode`
  (consumer) are byte-identical to the corresponding
  `crates/*/examples/dump_core` native output. Reuses the same fixture SVGs
  where practical so drift between the two `test:wasm` layers is visible.

`§3.4` Deno tests remain the place for the broader API-contract suite (`.cbor`
round-trip, `init` idempotency, malformed-payload rejection, plain-object
`RgbaResult`, etc.). §8 does not duplicate §3.4; it satisfies §3.3 literally in
a browser.

Incremental steps:

- §8.3 covers `svg2usvg` alone (non-empty `Uint8Array` + §3.3 case 1 for the
  producer).
- §8.4 adds `usvg2rgba` (shape: `pixels` is `Uint8Array`,
  `pixels.length === width*height*4`, plus §3.3 case 1 for the consumer).
- §8.5 fills in the remaining §3.3 case 2 (full byte-identity) for both entry
  points.

---

## 6. Dependency changes

| Dependency                 | Source tier (`AGENTS.md` §5.1)                                           | Justification                                                                                                                                                                                                                                                                                                                                                             | Vendoring                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| -------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `jsr:@astral/astral@0.5.6` | **1 — JSR** (preferred, versioned, content-addressable, subpath-capable) | Only way to satisfy `engineering-playbook.md` §3.3 literally ("headless browser") without introducing a WebDriver bridge or an npm package. The Deno-only harness cannot exercise browser module-loading/Wasm-instantiation. Astral is the minimal, maintained, JSR-native CDP driver; alternatives (Puppeteer, Playwright) require `npm:` and a separate `chromedriver`. | **Not vendored as package code.** Test-only dev dependency, imported as `jsr:@astral/astral@0.5.6` via a pinned specifier in the harness. Day-to-day work resolves from JSR per tier 1. If a release snapshot is later required, `scripts/vendor.ts` can reproduce the tree under `vendor/` (tier 2), but the package's ship artifacts (`src/*.ts`, `assets/*.wasm`) must not import it — enforced by `scripts/check-cdn-free.ts`. No `npm:` specifier is added anywhere. |

No other new dependency. No change to `[workspace.dependencies]` (`cbor-core`,
`usvg`, `resvg` pins stay at `0.10.1` / `0.47.0`).

---

## 7. Pinned Chrome — mechanism, version, cache location, no-sudo guarantee

- **Mechanism:** Astral's built-in fetcher (`src/cache.ts`, Chrome for Testing).
  `launch({ product: "chrome", cache: <dir> })` downloads the pinned build on
  first run and reuses it thereafter. No `puppeteer`, no `npm:` helper, no
  manual `curl | unzip` outside Astral.
- **Pinned version:** `125.0.6400.0` — the value of `SUPPORTED_VERSIONS.chrome`
  exported by `jsr:@astral/astral@0.5.6` (verified `deno info` + `deno run -A`
  print on 2026-08-22: `{"chrome":"125.0.6400.0","firefox":"116.0"}`). The
  harness pins the Astral specifier to `@0.5.6`; the Chrome version is therefore
  pinned transitively and reproducibly. If Astral is ever bumped, this plan
  requires the new `SUPPORTED_VERSIONS.chrome` value to be recorded in the
  commit message and in the harness's version assertion.
- **Cache location:** User-space, **outside the repo** by default: Astral's
  default `~/.cache/astral` (or an explicit `~/.cache/svg2ui8a-chrome` /
  `$XDG_CACHE_HOME/svg2ui8a` if the harness passes `cache:`). No file is written
  under the repo root, so no `.gitignore` entry is _required_ for the default.
  If the setup script instead chooses a repo-local cache (e.g.
  `.cache/chrome/`), the plan adds `.cache/` to `.gitignore` in the same commit
  — but the default is the user cache. CI may override via `cache:` or
  `ASTRAL_CACHE` env.
- **No `sudo`, no `apt`, no system package manager:** Confirmed. Astral
  downloads a hermetic Chrome binary via HTTPS; Chrome itself is not
  `apt-get`'d. The spike on this WSL2 host succeeded with no privilege
  escalation. The harness must not invoke `sudo`/`apt` and must fail fast with a
  diagnostic if Chrome cannot be fetched (e.g. offline CI).
- **Reproducibility note:** `deno.lock` pins the Astral content hash;
  `deno task test` without `--frozen=false` will refuse a silently changed
  Astral build. The Chrome binary itself is not hashed in `deno.lock` (it is a
  runtime download), but its version is pinned via the Astral version.

---

## 8. Expected Wasm size impact

**Zero.** No `crates/*` source or `Cargo.toml` is changed; no new Rust
dependency is added; `assets/*_bg.wasm` are exercised, not rebuilt. The only new
code is Deno-side test harnessing (Astral import). If a future step ever needed
to change a crate, this section would be updated and the size delta recorded;
for §8 as scoped, the delta is 0 bytes per artifact.

---

## 9. Tests to be added or updated

| Layer (`engineering-playbook.md` §3) | File                                                                                            | Cases                                                                                                                                                                                                                                        |
| ------------------------------------ | ----------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| §3.3 Wasm (browser) — **new**        | `scripts/test-wasm-browser.ts` (or `scripts/ensure-chrome.ts` + `scripts/test-wasm-browser.ts`) | §8.3: `svg2usvg` in Chrome → non-empty `Uint8Array`, Promise type. §8.4: `usvg2rgba` in Chrome → `RgbaResult` shape (`pixels.length === w*h*4`). §8.5: output byte-identical to `dump_core` native core for same inputs (both entry points). |
| §3.3 Wasm (Deno) — **unchanged**     | `scripts/test-wasm.ts`                                                                          | Stays; continues to run as `test:wasm`'s Deno-side complement.                                                                                                                                                                               |
| §3.4 Deno — **unchanged**            | `tests/*.test.ts`                                                                               | No change in §8.                                                                                                                                                                                                                             |
| §3.1 / §3.2 Rust — **unchanged**     | `crates/*/src`                                                                                  | No change.                                                                                                                                                                                                                                   |

All four layers must still pass after §8.6 wiring.

---

## 10. Wiring into `deno task test` (§8.6)

Current `deno.json` (`2026-08-22`):

```json
{
  "tasks": {
    "test": "deno task test:rust && deno task test:wasm && deno test -A",
    "test:wasm": "deno run -A scripts/test-wasm.ts"
  }
}
```

After §8.6, the intended order per `task.md` §8.6 is:

```
test:rust → test:wasm (Deno-side) → browser layer → deno test -A
```

Implementation choice (documented here, executed in §8.6 — not in §8.1):

```json
{
  "tasks": {
    "test:wasm": "deno run -A scripts/test-wasm.ts",
    "test:wasm:browser": "deno run -A scripts/test-wasm-browser.ts",
    "test": "deno task test:rust && deno task test:wasm && deno task test:wasm:browser && deno test -A"
  }
}
```

`test:wasm` keeps its current meaning (Deno-side byte-identity);
`test:wasm:browser` is the new browser layer; `test` runs both in order. An
alternative single-task alias is acceptable if the human prefers it — the plan
records the _order_, not the task name.

---

## 11. Documentation updates (§8.7)

- `docs/engineering-playbook.md` §3.3: add a sentence fixing the interpretation,
  e.g.:

  > The "or equivalent" in this section is fixed to **Astral-driven headless
  > Chrome via CDP** (`jsr:@astral/astral@0.5.6`, pinned Chrome `125.0.6400.0`);
  > no chromedriver, no WebDriver, no npm.

- `CHANGELOG.md` `Unreleased`: one-line entry for the browser-verified layer.

Both are docs-only, no crate change.

---

## 12. CI implication (§8.8)

- **Local WSL2 (verified 2026-08-22):** Astral downloads `125.0.6400.0` to
  `~/.cache/astral`, launches headless, CDP `page.evaluate` succeeds. No display
  server, no `sudo`/`apt`. `ldd` on the fetched `chrome-linux64/chrome` reports
  no missing shared libs; the host's `libnss3`, `libatk-bridge`, `libgbm`,
  `libasound2` etc. were already present.

- **CI — `ubuntu-latest` GitHub Actions (verified without standing up a run):**
  The stock `ubuntu-latest` runner image (checked `Ubuntu2404-Readme.md`,
  2026-08-22) **already ships** `Google Chrome 151.0.7922.137`, `ChromeDriver`,
  and `Chromium`, which implies its shared-library set is sufficient for
  headless Chrome. Astral's error-handling path for
  `error while loading shared libraries` (see `src/browser.ts`) — which suggests
  `apt-get install … google-chrome-stable fonts-… libxss1` — is the
  **minimal-Docker** fallback, not the `ubuntu-latest` path. Chrome for
  Testing's GitHub Actions examples and Astral's own docs do not prescribe extra
  packages for `ubuntu-latest`; outbound HTTPS to `googlechromelabs.github.io`
  on first run (or a cache restore) is the only requirement.

  **Therefore:** No `apt` step is expected on stock `ubuntu-latest`, and none is
  added by this task. The harness itself **never invokes `sudo`/`apt`** (same
  rule as local dev) and must surface Astral/Chrome stderr verbatim if launch
  fails.

- **Policy distinction made explicit:** The project's **local-dev** `no sudo` /
  `no apt` policy (task.md §8.2, this plan §7) is unchanged. CI is a throwaway,
  non-persistent environment owned by the runner; _if_ a future minimal
  container were used, any `apt` there would be a **runner-only, CI-workflow
  concern** — not a relaxation of the project's policy — and would need to be an
  exact, enumerated package list (e.g. the Puppeteer-troubleshooting list:
  `libnss3 libxss1
  libasound2 libatk-bridge2.0-0 libgbm1 …`), not a vague
  "documented deps". No such list is needed for `ubuntu-latest` as presently
  imaged.

- **Open-item handling (§8.8):** Because this conclusion is reached without
  standing up an actual CI run (out of scope for §8.1), §8.8 retains a one-line
  verification item: _"Confirm the browser layer passes on a clean
  `ubuntu-latest` run with no `apt` step; if launch fails, record the exact
  missing-lib list and add a runner-only install step — do not add it to the
  local-dev path."_ §8.1 defers the live proof to §8.8; the prose is no longer
  ambiguous about how much `apt` access is acceptable (answer: none on
  `ubuntu-latest` as shipped).

---

## 13. Risks and mitigations

- **Astral API churn:** Pinned to `@0.5.6`; `SUPPORTED_VERSIONS` guarded by an
  assertion in the harness. Bump only with a plan amendment.
- **Chrome download size / offline CI:** First run downloads ~100–150 MB;
  subsequent runs are cached. CI should cache `~/.cache/astral`.
- **Flaky CDP launch:** Harness must `await browser.close()` in `finally`, time
  out with a diagnostic, and exit non-zero on any unhandled rejection — never
  silently pass.

---

## 14. Out of scope

- Changing `crates/*` source, `Cargo.toml` workspace pins, `assets/*.wasm`, or
  the `src/usvg.ts`/`src/rgba.ts` generation template.
- Replacing `scripts/test-wasm.ts` — it stays as the Deno-side complement.
- Any `npm:` specifier, `chromedriver`, `Puppeteer`, or `Playwright`.

---

## 15. Approval gate

This plan is **§8.1 only**. No file listed in §3 beyond this plan is created or
modified in this session.

**Requested action:** Human reviews this plan and replies "approved" (or
requests edits). Only after explicit approval may §8.2 (pinned Chrome
acquisition script) begin. Per `engineering-playbook.md` §4 step 3, no
implementation before approval.
