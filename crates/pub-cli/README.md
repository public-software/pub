# pub-cli

The `pub` binary: the Public Software organization CLI. Version 0 knows the catalog.

```sh
pub catalog validate                 # catalog/catalog.toml, catalog.toml or config/catalog.toml, whichever exists
pub catalog render readme            # the ring tables the organization README embeds
pub catalog render json              # the repositories as a JSON array
pub catalog validate --catalog path/to/catalog.toml
```

The catalog's canonical home is `catalog/catalog.toml` in
[public-software/catalog](https://github.com/public-software/catalog); its rules are
`catalog/catalog.schema.json` there, and `validate` applies the same rules.
