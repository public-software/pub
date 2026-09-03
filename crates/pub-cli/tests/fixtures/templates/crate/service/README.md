# {{CRATE}}

The `{{DAEMON_PREFIX}}-{{COMPONENT}}` service of [{{REPO}}](https://github.com/{{ORG}}/{{REPO}}), part of {{ORG_DISPLAY_NAME}}. Kind: `service`; the daemon binary is `{{DAEMON_PREFIX}}-{{COMPONENT}}`, the logic is the library `{{CRATE_IDENT}}`.

_One paragraph: what the service does, what it listens on, what it stores, and what it does not do yet._

```sh
cargo run -p {{CRATE}} -- --health   # ok | degraded: <detail> | failing: <detail>; exit 0, 1 or 2
cargo nextest run -p {{CRATE}}
```

The health check is the `health` interface of `{{WIT_NAMESPACE}}:core` (the `interfaces` repository), answered by `Service::health`; an orchestrator reads the state, a human reads the detail.

Its entry in the repository's `CATALOG.toml`:

```toml
[[component]]
crate     = "{{CRATE}}"
kind      = "service"
ledger    = "<ledger entry name>"
readiness = "none"
effort    = 3
specs     = []
provides  = ["{{WIT_NAMESPACE}}:core/health@0.1.0"]
requires  = []
```
