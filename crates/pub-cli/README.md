# pub-cli

The `pub` binary: the Public Software organization CLI. Version 0 knows the catalog and stamps out crates.

```sh
pub catalog validate                 # catalog/catalog.toml, catalog.toml or config/catalog.toml, whichever exists
pub catalog render readme            # the ring tables the organization README embeds
pub catalog render json              # the repositories as a JSON array
pub catalog validate --catalog path/to/catalog.toml
pub new lib sched                    # crates/pub-<repo>-sched from the skeleton set, in the current repository
pub new service sched --templates ../templates   # a daemon pubd-sched, from a local templates checkout
```

The catalog's canonical home is `catalog/catalog.toml` in
[public-software/catalog](https://github.com/public-software/catalog); its rules are
`catalog/catalog.schema.json` there, and `validate` applies the same rules.

`pub new <kind> <component>` renders `crate/<kind>/` of the skeleton set (`lib`, `app`, `service`, `plugin`,
`spec`) into `crates/pub-<repo>-<component>/`, where `<repo>` is `[repo] name` of the `CATALOG.toml` in the
current directory or above it (`--dir` names the repository). The skeleton comes from `--templates <path>` (a
checkout of [public-software/templates](https://github.com/public-software/templates), or the bootstrap kit) or,
without it, from a shallow clone of that repository at `--ref` (default `main`). The placeholders are the ones the
templates repository's README lists; the rendering is byte-for-byte what its reference implementation produces:
text substituted and written with one trailing newline, images copied. An existing path, a component that is not
lowercase words joined by hyphens, and a placeholder the command does not know are refused before anything is
written. It prints the `[[component]]` entry to add to `CATALOG.toml`.
