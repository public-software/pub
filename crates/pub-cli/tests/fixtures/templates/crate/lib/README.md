# {{CRATE}}

The `{{COMPONENT}}` library of [{{REPO}}](https://github.com/{{ORG}}/{{REPO}}), part of {{ORG_DISPLAY_NAME}}. Kind: `lib`.

_One paragraph: what it does, the specification it implements (listed in the repository's `PROVENANCE.md`), what it does not do yet._

```sh
cargo nextest run -p {{CRATE}}
```

Its entry in the repository's `CATALOG.toml`:

```toml
[[component]]
crate     = "{{CRATE}}"
kind      = "lib"
ledger    = "<ledger entry name>"
readiness = "none"
effort    = 3
specs     = []
provides  = []
requires  = []
```
