# Module namespaces

This example demonstrates RWLang's V1 module namespace model.

`main.rw` declares the complete source graph explicitly:

```rw
mod catalog;
mod catalog::queries;
mod catalog::pages;
```

The mapping is application-root-relative and canonical:

```text
catalog.rw          -> catalog
catalog/queries.rw  -> catalog::queries
catalog/pages.rw    -> catalog::pages
```

Loading a module does not inject its declarations into a global namespace. Cross-module references therefore stay qualified: `catalog::Product`, `catalog::queries::recent(...)`, and `catalog::pages::index`.

Within one module, its own declarations may still be referenced by their short local names. Routes remain application-global HTTP identifiers and are separate from code namespaces.

Check it with:

```bash
cargo run --locked -q -p rwlang-cli -- check examples/module-namespaces/main.rw
```
