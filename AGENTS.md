# AGENTS.md

This file describes how AI coding agents should operate inside this repository.
It is the **first file any agent must read**, before touching any other file,
before any tool call beyond `read`, and before responding.

The repository may contain other documents under `./docs/`. They are
**task-specific** and must be read only when the task explicitly references
them. Do not pre-emptively read every markdown file in `./docs/`.

---

## 1. Project at a glance

This repository hosts the JSR package **`@tksh/svg2ui8a`**: a Wasm
SVG-processing toolkit whose inputs and outputs are all `Uint8Array`. The
package has two functions, in two subpath imports:

- `jsr:@tksh/svg2ui8a/usvg` → `svg2usvg(svg): Promise<Uint8Array>`
- `jsr:@tksh/svg2ui8a/rgba` → `usvg2rgba(usvg, options?): Promise<RgbaResult>`

There is no web application in this repository, no Deno Deploy wiring, and no
app-side routing. The package is the only thing this repository ships.

See:

- `./docs/project-constitution.md` for what the package is and is not.
- `./docs/system-architecture.md` for the package layout and data flow.
- `./docs/engineering-playbook.md` for how to build, test, and ship.

---

## 2. The agent's role

You are a careful collaborator. You are not an autonomous operator. The human is
the decision-maker; you are the implementer and the verifier.

Concretely:

- **Plan before you act.** For any task that touches more than one file, or that
  touches a non-trivial function, produce a written plan first. Save it under
  `./docs/plans/` and wait for the human to approve. Plans are exempt from the
  "no editing `./docs/`" rule (§8) — they are working artifacts, not governing
  documents.
- **Smallest possible diff.** Do not refactor, rename, or "improve" code that is
  not directly in scope.
- **No new dependencies without justification.** If you want to add a package,
  explain in the plan why an existing one cannot do the job.
- **No silent scope expansion.** If the plan surfaces a need that was not in the
  original task, stop and ask.
- **Implementer judgment is welcome.** When the governing documents describe a
  _shape_ but not a _detail_, the implementation detail is yours to choose. The
  constitution and architecture describe the boundary; the engineering playbook
  describes the workflow. They do not describe every function signature, every
  type mapping, or every import shape. If you find yourself writing code that
  the constitution does not contradict, you are within your mandate.
- **Verify before reporting done.** Run the project's tests, the formatter, the
  linter, the type checker. Report exactly which commands you ran and their
  outcomes.
- **When in doubt, ask.** A focused question is cheaper than a wrong
  implementation. "Should I use A or B?" is fine; "I picked A because reasons"
  is also fine; "I silently chose A and built on it" is not.

---

## 3. How to use the documentation

### Always read first

- `./AGENTS.md` (this file) — **always, at the start of every task.**

### Read in this order, when the task is about the package

1. `./AGENTS.md` (this file) — your role and the rules.
2. `./docs/project-constitution.md` — what the package is and is not.
3. `./docs/system-architecture.md` — the package layout and data flow.
4. `./docs/engineering-playbook.md` — how to build, test, and ship.

Do not skip steps. Do not re-order them. Each layer assumes the prior layer has
been read.

### Read only when the task references it

- Any other file under `./docs/` — read only when the human explicitly
  references it, or when the task description names it. This includes
  `docs/plans/<feature>.md` (only when the task is to execute a specific plan).

### Never read

- Files unrelated to the current task. Do not browse the repository beyond what
  the plan requires.
- Anything under `./notes/` — that is the human's personal design scratchpad,
  not project documentation.
- Build artifacts, `node_modules/`, `_build/`, `dist/`, lock files beyond what
  is needed.

---

## 4. Coding conventions

The repository's coding conventions are enforced by the formatter and the
linter. The agent's job is to write code that passes them, not to re-debate
their choices.

- **Deno-style imports.** Use `import { ... } from "..."` with static
  specifiers. No `require`. No dynamic `import()` unless the dynamic target is
  genuinely runtime-determined.
- **TypeScript strict mode.** All code must type-check under the project's
  `deno check` configuration.
- **Async functions return `Promise<T>`.** Sync functions return `T`. No
  `// @ts-ignore`, no `any` to escape type errors. If a type error is real, fix
  the types.
- **Formatting is non-negotiable.** The agent formats the files it has changed.
  If `deno fmt` would change lines the agent did not touch, those lines are left
  alone (revert any incidental changes before committing).
- **Comments explain _why_, not _what_.** The code shows _what_.
- **No new top-level files unless the plan calls for one.** The human-approved
  plan is the authority for what files exist.

---

## 5. Dependency policy

The package's value proposition includes **predictability of supply**: the
consumer must be able to build, test, and ship without depending on the uptime,
the bandwidth, or the terms of any third-party CDN.

### 5.1 Source of dependencies, in order of preference

1. **JSR (`jsr:` specifier).** Preferred. JSR is the canonical Deno package
   registry, versioned, content-addressable, and supports subpath imports.
2. **Local / vendored.** A dependency that is not on JSR must be vendored into
   the repository under `./vendor/` (or a similarly named directory) and
   imported via a relative path or a `deno.json` import map entry that points at
   the vendored copy.
3. **`deno.land/x` and `esm.sh`.** Permitted only as a fallback when neither (1)
   nor (2) is viable. When used, the dependency must still be vendored locally
   before any production build; the `deno.land/x` / `esm.sh` URL exists for
   discovery and initial bootstrap, not for runtime fetching.

`npm:` specifiers are not allowed in the package code. They are a consumer-side
concern, not a package concern.

### 5.2 Runtime CDN imports are forbidden

The browser bundle must not import dependencies from `https://deno.land/x/...`,
`https://esm.sh/...`, or any other runtime CDN at the moment the bundle is
loaded by a user. The bundle must contain every byte it needs to run.

Rationale: a third-party CDN going down, rotating their signing key, or changing
their caching policy must not be a cause of our users seeing a broken page. The
bundle is a self-contained artifact.

If a dependency cannot be vendored for a technical reason, that is a hard
blocker. Stop and escalate per §10.

### 5.3 Adding a dependency

A package may be added during planning only if the plan justifies it in the
"Dependency changes" section. The justification must name the source tier (1, 2,
or 3 above) and the vendoring strategy. The human must approve.

---

## 6. Testing

- The project's test command is `deno task test`, which orchestrates the
  project's test layers. See `./docs/engineering-playbook.md` §1 for the exact
  orchestration.
- The package produces RGBA pixels, not PNG. PNG encoding is the consumer's
  concern.
- Visual / pixel-equality tests on PNG output, if any, must be reviewed by the
  human before being marked passing. **Byte-level RGBA buffer tests** (asserting
  the contents of `RgbaResult.pixels` against a known-good output) are unit
  tests; they do not require human review.

---

## 7. Git hygiene

- One logical change per commit.
- Commit messages reference the section of the plan they implement, e.g.
  `svg2ui8a: add usvg2rgba subpath export`.
- Do not force-push, do not rebase other people's work, do not amend commits
  that have been pushed.
- Do not commit `dist/`, `node_modules/`, `_build/`, `target/`, `vendor/`, or
  any other generated or vendored artifact, **except** the Wasm binaries that
  the project explicitly vendors: the post-build `assets/*.wasm` files.
  Intermediate `crates/*/pkg/` is not committed. `vendor/` is not committed
  except as part of a release snapshot (see `./docs/engineering-playbook.md`
  §5.1). Check `.gitignore` first if unsure.

---

## 8. Boundaries: what the agent must not do

These are hard limits that apply regardless of what the plan says.

- Do not modify the **four governing documents**:
  - `AGENTS.md` (this file)
  - `docs/project-constitution.md`
  - `docs/system-architecture.md`
  - `docs/engineering-playbook.md`

  The human owns the documentation. If something is wrong, the human will edit
  it. Plans under `docs/plans/` are exempt from this rule — they are working
  artifacts.

- Do not read or modify anything under `./notes/`. That is the human's personal
  design scratchpad, not project documentation.
- Do not introduce a custom binary serialization format. The serialization is
  **CBOR** (RFC 8949) and is fixed. See `./docs/project-constitution.md` §3.7.
- Do not add fonts or font-handling code. The project has a hard constraint of
  no fonts.
- Do not add BBox-related code. The project does not use BBox.
- Do not add a PNG decoder package (`pngjs`, `sharp`, `@jsquash/png`, etc.) for
  any reason in this package. RGBA output is the package's contract; PNG / WebP
  / image-format encoding is the consumer's.
- Do not add a native `resvg` fallback. The project is committed to Wasm.
- Do not add a Node.js-only runtime fallback. Deno is the only supported
  runtime.
- Do not introduce a runtime CDN import in the browser bundle. See §5.2.
- Do not bundle `svg2usvg` and `usvg2rgba` into a single Wasm artifact. They are
  two separate builds, on purpose.
- Do not run `git push` without explicit human approval.
- Do not merge a PR you opened. The human merges.

---

## 9. Reporting back

When you finish a task:

1. State the exact commands you ran and their results.
2. List the files you changed, with one-line summaries.
3. State which checks passed and which were skipped.
4. Surface any deviation from the plan, with the reason.
5. Surface any open question or follow-up the human should know.

Keep the report short. The code is the artifact; the report is the narrative.

---

## 10. Escalation

Stop and ask the human if:

- The constitution / architecture / playbook contradict each other or the
  codebase.
- A real type error cannot be fixed without a design decision.
- A test fails and the cause is not obvious from the diff.
- The plan would require modifying any of the four governing documents.
- A new dependency seems necessary.
- The task expands beyond what the user originally asked for.
- A design question arises that the human has not yet answered (e.g. a target JS
  bundler, a specific Deno version, a specific Wasm target).
- A dependency cannot be vendored, blocking §5.2.
- The cross-Wasm test that the engineering playbook calls for fails for a reason
  the agent cannot diagnose in a few minutes.

Do not guess. Do not push through. Ask.
