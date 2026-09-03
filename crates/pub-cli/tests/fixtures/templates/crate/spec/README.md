# {{CRATE}}

The `{{COMPONENT}}` specification of [{{REPO}}](https://github.com/{{ORG}}/{{REPO}}), part of {{ORG_DISPLAY_NAME}}. Kind: `spec`: the normative text is [SPEC.md](SPEC.md) (also the crate's documentation) and the conformance cases are the typed table `{{CRATE_IDENT}}::cases()` that implementations test against.

```sh
cargo doc -p {{CRATE}} --open       # the specification
cargo nextest run -p {{CRATE}}      # the cases are well-formed: unique names, every one tied to a requirement
```

Its entry in the repository's `CATALOG.toml`:

```toml
[[component]]
crate     = "{{CRATE}}"
kind      = "spec"
ledger    = "<ledger entry name>"
readiness = "none"
effort    = 3
specs     = ["{{COMPONENT}}"]
provides  = []
requires  = []
```
