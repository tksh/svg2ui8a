# Prompt: Revise `imple-by-grok/docs/plans/initial-implementation.md` against the updated governing documents

Use this prompt when you want an AI coding agent to **read the updated
four governing documents and revise the existing implementation plan
to match them**. The prompt is for a planning session, not an
implementation session. The agent's only deliverable is a revised plan
in `imple-by-grok/docs/plans/initial-implementation.md`, plus a short
summary.

---

## Prompt

You are an AI coding agent working on the JSR package
**`@tksh/svg2ui8a`**. The plan you wrote earlier at
`imple-by-grok/docs/plans/initial-implementation.md` was based on an
**earlier version of the four governing documents**. The four documents
have since been revised. Your task is to **read the revised documents,
reconcile the plan with them, and write a revised plan back to the
same path**.

You are in a **planning session**. Do not edit source files, do not
run `cargo build`, do not run `git apply`, do not commit anything.
Your only deliverable is the revised plan and a short summary.

---

## What changed in the governing documents

The four governing documents are:

1. `./AGENTS.md` (or its equivalent in the work repository)
2. `./docs/project-constitution.md`
3. `./docs/system-architecture.md`
4. `./docs/engineering-playbook.md`

Read all four, in this order, fully, before touching the plan.

**Read them in the work repository, not in any other location.** If
the work repository does not contain a `docs/` directory, escalate
immediately — the documents have not been vendored into the work
repo, and proceeding without them would mean re-doing the analysis
that already happened.

### The changes that matter for your plan

These are the substantive changes from the version you wrote
against. Your existing plan's Task 0 (the `usvg::Tree` ↔
`postcard` round-trip gate) **was correct at the time** but is **no
longer the right gate** under the updated documents. The change
pattern is:

1. **Serialization format: `postcard` → `CBOR` (RFC 8949).**
   See `docs/project-constitution.md` §3.7. `postcard` is no
   longer in scope. `CBOR` is the only allowed format. The
   constitution explicitly notes that a deterministic profile
   (e.g. `dCBOR`) is also acceptable; switching to it does not
   require a constitution amendment.

2. **The internal representation is an implementation choice, not
   a `usvg::Tree` direct serialization.** See
   `docs/system-architecture.md` §4. The constitution previously
   assumed `usvg::Tree` was `serde::Serialize`. That assumption
   was wrong: `usvg::Tree` does not implement `serde::Serialize`
   in the current upstream. The package's bytes are now defined
   to be a CBOR-encoded payload of **an internal representation
   that the implementation chooses**. The three options named in
   the architecture are:
   - direct serialization of `usvg::Tree` (only if a future
     upstream version adds serde support; verify at impl time),
   - a hand-designed DTO that the package maps `usvg::Tree` into
     (the recommended default; does not depend on upstream serde),
   - a hand-rolled SVG normalizer (most work, least dependency).
   The plan must pick one and record the rationale.

3. **The `usvg::Tree` ↔ `postcard` gate is replaced by a different
   gate.** The new gate is: "the chosen internal representation
   can be encoded and decoded back through the chosen CBOR codec
   without loss, deterministically, for a known SVG input." The
   plan records the exact form of this gate in the same Task 0
   slot.

4. **The agent has more implementation freedom than the previous
   documents suggested.** See `docs/engineering-playbook.md` §0.
   The governing documents describe the *boundary*; the
   implementer chooses:
   - the exact CBOR codec crate and its version,
   - the exact DTO shape (if any),
   - the exact `usvg` / `resvg` call sequence,
   - the exact Wasm import path,
   - the exact error type and message wording,
   - the exact un-premultiply arithmetic.
   The plan records these choices once the implementer makes them;
   it does not pre-specify them in the constitution.

5. **Sample code in the governing documents has been deliberately
   "demoted" to candidates.** The previous architecture showed
   specific Rust code, including `postcard::to_stdvec`,
   `tree.render(transform, &mut pixmap.as_mut())`, and a specific
   un-premultiply function. The updated architecture presents
   these as **candidate** calls, not as normative. The plan
   should not over-anchor on any of them.

6. **Version pins moved out of the governing documents into the
   plan.** The previous architecture pinned `usvg = "0.34"`,
   `resvg = "0.34"`, `postcard = "1.0"`. The updated architecture
   does not pin versions — it leaves the choice to the
   implementation, to be recorded in the plan with a rationale.
   When the plan picks versions, it must also surface the
   assumption that **`usvg` and `resvg` share a version number
   family** in upstream; they must be pinned together.

7. **The plan must include a "decisions" section.** Because the
   governing documents are now thinner than they were, the plan
   must carry the burden of recording what the implementation
   actually chose and why. Every previous "candidate"
   (CBOR codec, DTO shape, version pins, Wasm import path)
   becomes a row in a "Decisions" table in the plan, with a
   one-sentence rationale each.

### The things that did NOT change

For completeness, so you do not waste time re-debating them:

- The package name is still `@tksh/svg2ui8a`.
- The two-function shape (`svg2usvg`, `usvg2rgba`) is still the
  product.
- The two-subpath layout (`./usvg`, `./rgba`) is still the
  distribution.
- The two-Wasm-artifact constraint (constitution §3.9) is still
  in force.
- The no-fonts / no-BBox / no-PNG / no-native-fallback /
  no-Node.js / no-CDN constraints are still in force.
- The Deno-only runtime target is still in force.
- The cache-key prefixing pattern (constitution §6) is still in
  force; the only change is the encoder-version slot now reads
  `<cbor-encoder-version>` instead of `<postcard-version>`.
- The "package produces RGBA, not PNG" contract is still in
  force.
- The default alpha mode is still `straight`; the premultiplied
  option is still available; the field is still a string-literal
  enum, not a boolean.

---

## What to do, step by step

1. **Read the four governing documents.** Cite specific section
   numbers in the revised plan, not generalities.

2. **Read your existing plan.** It is at
   `imple-by-grok/docs/plans/initial-implementation.md`. Re-read
   it. Note which parts are now obsolete (Task 0 in its old
   form; the version pins; the postcard references; the
   sample-code expectations) and which parts are still valid
   (the task DAG; the two-crate split; the test layer split; the
   "no native fallback" rule).

3. **Draft a revised plan in the same file**, overwriting the
   previous content. The new plan must include:

   - The **purpose and scope**, in the same form as before.
   - The **updated task DAG**, with Task 0's gate rewritten
     against CBOR and the internal representation. Task 1 and
     onward should be largely unchanged in shape.
   - A new **"Decisions" section** that records:
     - chosen CBOR codec crate + version,
     - chosen internal representation (DTO / direct Tree / custom),
     - chosen `usvg` / `resvg` version (pinned together),
     - chosen Wasm import path (raw `.wasm` vs. wasm-pack glue),
     - chosen alpha-mode field name in the public API,
     - chosen error type and module,
     - any other place where the constitution said "implementation
       chooses" and the implementer chose.
     Each row has a one-sentence rationale.
   - The **dependency graph** updated to match the new choices.
   - The **test plan** updated to match the new gate (the
     round-trip test now targets the chosen internal
     representation, not `usvg::Tree`).
   - The **commit strategy** you previously proposed (DAG-staged
     PRs), unchanged.
   - The **open questions**, revised. Specifically:
     - Remove the now-obsolete questions about `postcard` and
       the `usvg::Tree` serde assumption.
     - Add questions about the choices you made in the
       "Decisions" section, if you are not confident in them
       and want human review.
     - If any new constitution-level question arises (e.g. the
       chosen CBOR crate is not on the allow-list), escalate per
       `AGENTS.md` §10.

4. **Cite the four governing documents by section number** at the
   top of each section of the plan, so a human reviewer can
   cross-reference. The previous plan already did this in part;
   expand it to cover the new sections.

5. **Surface any new contradiction** you find between the
   governing documents, between the governing documents and the
   plan, or between the plan and the actual repository state. Do
   not silently fix contradictions; list them under "Open
   questions" and ask the human.

6. **Do not invent tasks that the plan did not already have.**
   If a task seems necessary because the new documents changed
   the design (e.g. a "DTO design" task before Task 1), add it,
   but explain why. The default is to keep the task list as close
   to the previous one as the new design allows.

7. **Save the revised plan** to the same path,
   `imple-by-grok/docs/plans/initial-implementation.md`. Overwrite
   the previous content; do not append a "revision 2" section.

8. **Surface a short summary** (≤ 20 lines) in your final reply
   that points to the revised plan and lists the substantive
   changes you made.

---

## What NOT to do

- Do not edit any of the four governing documents.
- Do not edit any source code.
- Do not run `git` commands other than `status` (if you need to
  confirm the working state).
- Do not start implementation. This is a planning session.
- Do not re-litigate decisions the human already made (CBOR
  over postcard; `RgbaResult` over bare `Uint8Array`;
  `alphaMode` as a string-literal enum; independent scaling).
  These are settled.
- Do not re-litigate the no-fonts / no-BBox / no-PNG /
  no-native-fallback / no-Node.js / no-CDN constraints.
- Do not propose a single-Wasm-artifact approach.
- Do not propose runtime CDN imports in the browser bundle.
- Do not introduce a custom serialization format other than
  CBOR (or a deterministic profile of CBOR, e.g. `dCBOR`).

---

## Reporting back

When you finish:

1. Confirm that the four governing documents were read in full.
2. Summarize the substantive changes you made to the plan, in
   ≤ 20 lines.
3. List the open questions that the human should review.
4. State which decisions you made in the "Decisions" section
   that you are most uncertain about, so the human can prioritize
   review.

Keep the summary short. The plan is the artifact; the summary is
the narrative.
