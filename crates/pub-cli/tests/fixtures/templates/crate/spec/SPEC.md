# {{COMPONENT}} — specification

- Status: draft
- Repository: [{{REPO}}](https://github.com/{{ORG}}/{{REPO}})
- Crate: `{{CRATE}}` (kind `spec`); the conformance cases are `{{CRATE_IDENT}}::cases()`

## Scope

_What this specification covers, and what it leaves to other specifications._

## Terms

_The words this text uses in a fixed sense._

## Requirements

Each requirement is numbered `{{COMPONENT}}-1`, `{{COMPONENT}}-2`, … and has at least one conformance case in `src/lib.rs` that names it.

- `{{COMPONENT}}-1` — _the first requirement._

## Conformance

An implementation conforms when, for every case in `{{CRATE_IDENT}}::cases()`, it produces `expected` from `input`. The encoding of both is fixed here, once, so that implementations in any language can run the same cases.
