# Plan: JSR publish readiness (`task.md` §12)

Task: `task.md` §12 — verify that `jsr:@tksh/svg2ui8a` is ready to publish,
without performing a real publish. This document satisfies
`engineering-playbook.md` §4 step 2; approval is required before running
`deno publish --dry-run`.

---

## 1. Authorization — quoted

### 1.1 `docs/project-constitution.md` §7 (versioning)

> The package follows **strict semver**:
>
> - Patches: Wasm runtime tweaks that preserve supported format versions and
>   their output bytes for any input.
> - Minors: additions to the API surface or support for a new,
>   backward-compatible format version; version 1 remains decodable.
> - Majors: incompatible changes to an existing format version or removals from
>   the API surface.
>
> The envelope and every supported format version are part of the public file
> format contract.

The current `deno.json` (now canonical for JSR metadata, see §3) version is
`0.1.0` (pre-1.0); semver permits API change before 1.0, which the README now
reflects. `jsr.json` was removed in §12 verification to avoid duplicate
authoritative metadata — `deno.json` is the single source.

### 1.2 `docs/system-architecture.md` §2 (JSR subpath exports)

> The package exposes the following subpaths: `@tksh/svg2ui8a` (re-exports both
> `./usvg` and `./rgba`), `@tksh/svg2ui8a/usvg` (`svg2usvg` only), and
> `@tksh/svg2ui8a/rgba` (`usvg2rgba` only).

These are the three exports declared in `deno.json` (now canonical; `jsr.json`
was removed) (`"."`, `"./usvg"`, `"./rgba"`).

### 1.3 `docs/engineering-playbook.md` §6 (releasing)

> Releases are the human's responsibility. The agent's responsibilities around
> releases are:
>
> - Add an entry to `CHANGELOG.md` under the **"Unreleased"** section as part of
>   every change.
> - Run `deno task build` and commit the regenerated artifacts before the
>   release.
> - Tag the release commit with `v<version>` (human does this).
>
> The agent must not push tags, publish to JSR, or merge release PRs. The human
> does those.

This plan therefore verifies readiness via `deno publish --dry-run` only — no
`deno publish`, no tag, no push.

### 1.4 `docs/engineering-playbook.md` §4 (plan required)

> 2. **Draft a plan** in `docs/plans/<feature>.md`. The plan must:
>    - Quote the section of the constitution that authorizes the change.
>    - List the files to be touched, including which of the two crates and which
>      of the two Wasm artifacts are affected.
>    - List the tests to be added or updated.
>    - State the expected size impact on the Wasm binary.
>    - Justify any new dependency under the source tiers in `./AGENTS.md` §5.

The sections below satisfy those five items.

---

## 2. Decision

Verify JSR publish readiness without publishing. No new runtime dependency, no
new subpath, no Wasm rebuild. The only file edits expected are the `task.md` §12
checklist (already added) and this plan itself. If `deno publish --dry-run`
reports errors (missing `README.md`, `LICENSE`, `exports`, or excluded files),
fix them in the smallest possible diff and re-run the dry-run.

---

## 3. Scope — files touched

No file below is created or modified in the plan step beyond this plan and
`task.md` §12.

| File                                                        | Action in §12                                                                                                                                                                                                                                                                                       | Crates / artifacts affected                                |
| ----------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `docs/plans/jsr-publish-readiness.md`                       | **NEW** — this plan                                                                                                                                                                                                                                                                                 | none (working artifact)                                    |
| `task.md` §12                                               | **EDIT** — checklist added (already done)                                                                                                                                                                                                                                                           | none                                                       |
| `deno.json`                                                 | **EDIT+VERIFY** — becomes the single canonical JSR manifest (`name` `@tksh/svg2ui8a`, `version` `0.1.0`, `exports` `"."`, `"./usvg"`, `"./rgba"`), adds `publish.exclude` to ship only consumer files, and `imports` centralizes `@astral/astral` pin to `jsr:@astral/astral@0.5.6` (single source) | none; `deno.json` is the only file edited for JSR metadata |
| `jsr.json`                                                  | **REMOVE** — deleted; `deno.json` is now the single source of truth (JSR accepts `deno.json` alone, verified by `deno publish --dry-run` succeeding without `--config jsr.json`)                                                                                                                    | none; deletion avoids drift between duplicate manifests    |
| `README.md`                                                 | **VERIFY** — real README present, `deno fmt` clean, covers 9-section structure, no internal-process references                                                                                                                                                                                      | none                                                       |
| `LICENSE`                                                   | **VERIFY** — MIT file present at repo root                                                                                                                                                                                                                                                          | none                                                       |
| `src/mod.ts`, `src/usvg.ts`, `src/rgba.ts`, `assets/*.wasm` | **VERIFY** — exports and artifacts exist and are built; dry-run will list included files                                                                                                                                                                                                            | both Wasm artifacts are _verified_, not rebuilt            |
| `CHANGELOG.md`                                              | **VERIFY** — `Unreleased` entry exists                                                                                                                                                                                                                                                              | docs only                                                  |

**Not touched:** any file under `crates/*`, no new dependency, no `vendor/`
change, no `deno.lock` change beyond what `deno publish --dry-run` may refresh
implicitly (if so, commit separately as `chore: update deno.lock`).

---

## 4. What will be verified and why

### 4.1 `deno.json` (canonical JSR manifest) fields — why `deno.json`

`deno publish --dry-run --allow-dirty` without `--config` initially failed with
`Missing 'name' field in 'deno.json'` while `jsr.json` already declared it. JSR
accepts `deno.json` alone for publishing — verified by removing `jsr.json` and
re-running the dry-run, which then succeeded with `deno.json` as the sole
manifest. To avoid two files independently maintaining
`name`/`version`/`exports` (drift risk), `deno.json` was chosen as the single
canonical source and `jsr.json` was deleted. The dry-run now succeeds without
`--config`.

Check that `deno.json` `name` is `@tksh/svg2ui8a`, `version` matches `0.1.0` (or
the current file version at run time), and `exports` maps
`"." → "./src/mod.ts"`, `"./usvg" → "./src/usvg.ts"`,
`"./rgba" → "./src/rgba.ts"` — exactly the three subpaths documented in
`system-architecture.md` §2 and `README.md` Installation. `jsr.json` no longer
exists, so no duplicate to keep in sync.

### 4.2 `deno.json` imports map

Check that the Astral pin `0.5.6` appears **only** in `deno.json` `imports`
(`"@astral/astral": "jsr:@astral/astral@0.5.6"`), consistent with `Cargo.toml`
`[workspace.dependencies]` centralization on the Rust side. Verify
`grep -rn "jsr:@astral"` outside `deno.json` finds only non-import prose
(README, CHANGELOG, plan docs, `console.log` strings), not `from "jsr:..."`
specifiers.

### 4.3 `deno publish --dry-run` (with `publish.exclude`)

Run `deno publish --dry-run --allow-dirty` (not `deno publish`). It validates:
`deno.json` `exports` resolve, `README.md` and `LICENSE` are included, and the
Wasm assets are listed as included. The dry-run **must not** list internal
process docs — `AGENTS.md`, `task.md`, `docs/**`, `notes/**`, `crates/**`,
`scripts/**`, `tests/**`, `Cargo.toml`/`Cargo.lock`, `.gitignore`,
`.githooks/**`, `deno.lock`, `.git` — these are excluded via `deno.json`
`publish.exclude` which limits the published set to only consumer-needed files:
`README.md`, `LICENSE`, `CHANGELOG.md`, `deno.json`, `src/mod.ts`,
`src/usvg.ts`, `src/rgba.ts`, `assets/*.wasm`. The command must report **no
errors** and list **only** those 9 consumer files (plus `deno.json` itself). If
it reports missing `exports` or extra excluded files still appearing, fix
`deno.json` `publish.exclude` or file layout in the smallest diff and re-run.

### 4.4 Follow-up after real publish (deferred)

After `deno publish --dry-run` succeeds, **do not** re-run the README Quick
example snippet against the live `jsr:@tksh/svg2ui8a` import path — the package
is not yet published, so `jsr:` import would fail. Record this as a
**post-publish follow-up item**: after the human runs the real `deno publish`
and the package appears at `https://jsr.io/@tksh/svg2ui8a`, re-run the Quick
example via `jsr:@tksh/svg2ui8a` and confirm the commented expected outputs
(`10 10 "straight"`, `20 10`, `400`, `800`) match the live import. This
follow-up is deferred and is not executed in §12.

---

## 5. Tests to be added or updated

No new tests. Publish readiness is verified by:

- `deno publish --dry-run` (the primary check).
- Existing hygiene gates still pass: `cargo fmt --check`, `deno fmt --check`,
  `deno task lint` (0 problems), `deno task check`.

No test file is created or modified in §12.

---

## 6. Expected Wasm size impact

**Zero.** No `crates/*` source, `Cargo.toml`, or `assets/*.wasm` is changed in
§12. Publish readiness is a metadata / docs verification only.

---

## 7. Dependency changes

| Dependency | Source tier (`AGENTS.md` §5.1) | Justification                                                                                                    | Vendoring |
| ---------- | ------------------------------ | ---------------------------------------------------------------------------------------------------------------- | --------- |
| (none)     | —                              | No new dependency in §12. `jsr:@astral/astral@0.5.6` was added in §8 and remains pinned via `deno.json` imports. | —         |

No new dependency is justified or added in this plan.

---

## 8. Risks and mitigations

- **`deno publish --dry-run` reports missing files:** Fix `jsr.json` exports or
  include patterns; re-run. The plan's file list (§3) is the minimal set to
  adjust.
- **`jsr:` import path in README not yet live:** Explicitly deferred per §4.4 —
  not a failure of dry-run. The dry-run validates the local package layout; live
  `jsr:` resolution is a post-publish check.

---

## 9. Out of scope

- Running the real `deno publish` (human does this after approval).
- Re-running the Quick example against `jsr:@tksh/svg2ui8a` before publish
  (deferred).
- Changing `crates/*` source, Wasm artifacts, or `vendor/`.

---

## 10. Approval gate

This plan is §12 **plan only**. No `deno publish --dry-run` is executed in this
step.

**Requested action:** Human reviews this plan and replies "approved" (or
requests edits). Only after explicit approval may §12 verification
(`deno publish --dry-run` and the `jsr.json`/`deno.json` checks) begin, per
`engineering-playbook.md` §4 step 3.
