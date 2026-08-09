# Dependency-baseline documentation update

## Goal

Document the Rust dependency baseline for `@tksh/svg2ui8a` and make the
repository's formatting commands explicit by file type.

## Decisions

- Use Cargo package `cbor-core` 0.10.1 (`cbor_core` in Rust) for canonical CBOR
  encoding and strict canonical-CBOR decoding.
- Pin `usvg` and `resvg` to the currently published 0.47.0 release. Version
  0.48.1 remains a future upgrade target because it is not currently published.
- Format `.rs` files with `cargo fmt`; format `.ts` and `.md` files with
  `deno fmt`.

## Affected documents

- `docs/project-constitution.md`
- `docs/system-architecture.md`
- `docs/engineering-playbook.md`

## Acceptance checks

- The dependency names, versions, and canonical-CBOR behavior agree across the
  affected documents.
- The documentation contains no `ciborium`, `serde_cbor`, or
  implementation-defined CBOR codec references.
- Formatting instructions identify `cargo fmt` for Rust and `deno fmt` for
  TypeScript and Markdown.

## Follow-up

The architecture should later reconcile its requirement for a single
intermediate-schema location with its statement that the two crates share no
source code. The no-font requirement also needs an explicit `usvg`
feature-configuration decision before implementation.
