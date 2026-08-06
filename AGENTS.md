# AGENTS.md

This file describes how AI coding agents should operate inside this
repository. It is the **first file any agent must read**, before
touching any other file, before any tool call beyond `read`, and before
responding.

The repository may contain other documents under `./docs/`. They are
**task-specific** and must be read only when the task explicitly
references them. Do not pre-emptively read every markdown file in
`./docs/`.

---

## 1. Project at a glance

This is a Deno-first web application with the following moving parts:

- Runtime: **Deno** (no Node.js in production; npm packages are pulled
  in via `npm:` specifiers or `deno.lock` resolution).
- Build: **`deno bundle`** for the browser bundle; Deno Deploy for the
  serverless side.
- A planned dynamic OGP image feature, served from Deno Deploy.
- An in-progress SVG-to-raster rendering pipeline, built on top of
  `usvg` / `resvg`. See `./docs/project-constitution.md` for why and
  `./docs/system-architecture.md` for what.

The current task is the `svg2ui8a` package. Other tasks will follow.
This file's guidance applies to all of them; the docs in `./docs/`
are specific to their respective concerns.

---

## 2. The agent's role

You are a careful collaborator. You are not an autonomous operator. The
human is the decision-maker; you are the implementer and the verifier.

Concretely:

- **Plan before you act.** For any task that touches more than one
  file, or that touches a non-trivial function, produce a written plan
  first. Save it under `./docs/` and wait for the human to approve.
- **Smallest possible diff.** Do not refactor, rename, or "improve"
  code that is not directly in scope.
- **No new dependencies without justification.** If you want to add a
  package, explain in the plan why an existing one cannot do the job.
- **No silent scope expansion.** If the plan surfaces a need that was
  not in the original task, stop and ask.
- **Verify before reporting done.** Run the project's tests, the
  formatter, the linter, the type checker. Report exactly which
  commands you ran and their outcomes.
- **When in doubt, ask.** A focused question is cheaper than a wrong
  implementation.

---

## 3. How to use the documentation

### Always read first

- `./AGENTS.md` (this file) — **always, at the start of every task.**

### Read in this order, when the task is about `svg2ui8a`

1. `./AGENTS.md` (this file) — your role and the rules.
2. `./docs/project-constitution.md` — what `svg2ui8a` is and is not.
3. `./docs/system-architecture.md` — the package layout and data flow.
4. `./docs/engineering-playbook.md` — how to build, test, and ship.

Do not skip steps. Do not re-order them. Each layer assumes the prior
layer has been read.

### Read only when the task references it

- Any other file under `./docs/` — read only when the human explicitly
  references it, or when the task description names it.

### Never read

- Files unrelated to the current task. Do not browse the repository
  beyond what the plan requires.
- Build artifacts, `node_modules/`, `_build/`, `dist/`, lock files
  beyond what is needed.

---

## 4. Coding conventions

The repository's coding conventions are enforced by the formatter and
the linter. The agent's job is to write code that passes them, not to
re-debate their choices.

- **Deno-style imports.** Use `import { ... } from "..."` with static
  specifiers. No `require`. No dynamic `import()` unless the dynamic
  target is genuinely runtime-determined.
- **TypeScript strict mode.** All code must type-check under the
  project's `deno check` configuration.
- **Async functions return `Promise<T>`.** Sync functions return `T`.
  No `// @ts-ignore`, no `any` to escape type errors. If a type error
  is real, fix the types.
- **Formatting is non-negotiable.** Run `deno fmt` before declaring
  done. If `deno fmt` would change lines you did not edit, leave them
  alone — only run the formatter on lines you changed.
- **Comments explain *why*, not *what*.** The code shows *what*.
- **No new top-level files unless the plan calls for one.** The
  human-approved plan is the authority for what files exist.

---

## 5. Dependency policy

- Deno-native modules from `deno.land/x` and `esm.sh` are preferred.
- npm packages are allowed when no Deno-native equivalent exists, or
  when the user explicitly requests an npm package.
- A package may be added during planning only if the plan justifies it
  in the "Dependency changes" section. The human must approve.
- Wasm binaries that the Rust side produces must be vendored as static
  assets, not loaded from a CDN at runtime, unless the human explicitly
  approves a CDN load.

---

## 6. Testing

- The project's test command is `deno task test` (or whatever the
  current `deno.json` defines — check first; do not assume).
- For Rust code, run `cargo test` (and `cargo test --target
  wasm32-unknown-unknown` if Wasm is involved).
- Visual / pixel-equality tests, if any, must be reviewed by the
  human before being marked passing. Do not auto-approve pixel diffs.
- The PNG IHDR-only size helper from
  `./docs/engineering-playbook.md` is the prescribed approach for
  reading image dimensions. Do not introduce a PNG decoder dependency
  for this.

---

## 7. Git hygiene

- One logical change per commit.
- Commit messages reference the section of the plan they implement,
  e.g. `svg2ui8a: split Wasm build into usvg and renderer artifacts`.
- Do not force-push, do not rebase other people's work, do not amend
  commits that have been pushed.
- Do not commit `dist/`, `node_modules/`, `_build/`, `target/`, or any
  other generated artifact. Check `.gitignore` first if unsure.

---

## 8. Boundaries: what the agent must not do

These are hard limits that apply regardless of what the plan says.

- Do not modify any file under `./docs/` (this file, or any of the
  `docs/*.md` files). The human owns the documentation. If something
  is wrong, the human will edit it.
- Do not introduce a custom binary serialization format. The
  serialization is `postcard` and is fixed.
- Do not add fonts or font-handling code. The project has a hard
  constraint of no fonts.
- Do not add BBox-related code. The project does not use BBox.
- Do not add a PNG decoder package (`pngjs`, `sharp`,
  `@jsquash/png`, etc.) solely to read image dimensions. Use the
  IHDR helper.
- Do not add a native `resvg` fallback. The project is committed to
  Wasm.
- Do not add a Node.js-only runtime fallback. Deno is the only
  supported runtime.
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

Keep the report short. The code is the artifact; the report is the
narrative.

---

## 10. Escalation

Stop and ask the human if:

- The constitution / architecture / playbook contradict each other
  or the codebase.
- A real type error cannot be fixed without a design decision.
- A test fails and the cause is not obvious from the diff.
- The plan would require modifying `AGENTS.md` or any `./docs/*.md`.
- A new dependency seems necessary.
- The task expands beyond what the user originally asked for.
- A design question arises that the human has not yet answered (e.g.
  a target JS bundler, a specific Deno version, a specific Wasm
  target).

Do not guess. Do not push through. Ask.
