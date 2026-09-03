# {{CRATE}}

The `{{COMPONENT}}` command of [{{REPO}}](https://github.com/{{ORG}}/{{REPO}}), part of {{ORG_DISPLAY_NAME}}. Kind: `app`; the binary is `{{COMPONENT}}`.

_One paragraph: what the command does, for whom, and what it does not do yet._

```sh
cargo run -p {{CRATE}} -- --version
cargo nextest run -p {{CRATE}}      # unit tests and tests/cli.rs, which runs the built binary
```

Its entry in the repository's `CATALOG.toml`:

```toml
[[component]]
crate     = "{{CRATE}}"
kind      = "app"
ledger    = "<ledger entry name>"
readiness = "none"
effort    = 3
specs     = []
provides  = []
requires  = []
```
