# {{CRATE}}

The `{{COMPONENT}}` plugin of [{{REPO}}](https://github.com/{{ORG}}/{{REPO}}), part of {{ORG_DISPLAY_NAME}}. Kind: `plugin`: a WebAssembly component implementing the `component` world of `{{WIT_NAMESPACE}}:core` (the `interfaces` repository), hosted by `plugin-runtime`.

_One paragraph: what the plugin adds to its host, which interfaces it imports, and what it does not do yet._

```sh
cargo nextest run -p {{CRATE}}                          # host build: the manifest and the unit tests
cargo build -p {{CRATE}} --target wasm32-wasip2          # the component, once plugin-runtime's tooling is in place
```

Its entry in the repository's `CATALOG.toml`:

```toml
[[component]]
crate     = "{{CRATE}}"
kind      = "plugin"
ledger    = "<ledger entry name>"
readiness = "none"
effort    = 3
specs     = []
provides  = ["{{WIT_NAMESPACE}}:core/component@0.1.0"]
requires  = ["{{WIT_NAMESPACE}}:core/health@0.1.0", "{{WIT_NAMESPACE}}:core/logging@0.1.0"]
```
