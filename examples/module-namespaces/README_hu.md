# Modulnévterek

Ez a példa az RWLang V1 modulnévtér-modelljét mutatja be.

A `main.rw` explicit módon deklarálja a teljes forrásgráfot:

```rw
mod catalog;
mod catalog::queries;
mod catalog::pages;
```

A feloldás application-root relatív és kanonikus:

```text
catalog.rw          -> catalog
catalog/queries.rw  -> catalog::queries
catalog/pages.rw    -> catalog::pages
```

Egy modul betöltése nem emeli a deklarációit globális névtérbe. Modulhatáron át ezért kvalifikált név kell: `catalog::Product`, `catalog::queries::recent(...)` és `catalog::pages::index`.

Egy modulon belül a saját deklarációk rövid lokális neve továbbra is használható. A route-ok alkalmazásszintű HTTP-azonosítók, és külön fogalmat alkotnak a kód névterétől.

Ellenőrzés:

```bash
cargo run --locked -q -p rwlang-cli -- check examples/module-namespaces/main.rw
```
