# Projektstruktúra, modulok és névterek

Az RWLang modulok application-root relatív névterek. A `mod` betölt egy source unitot, de annak deklarációit nem emeli be sem az aktuális, sem a globális névtérbe.

## Kanonikus leképezés

```text
main.rw                alkalmazás-root
models.rw              models
pages/article.rw       pages::article
admin/users/edit.rw    admin::users::edit
```

Pontosan egy forrásleképezés van: `a::b` → `<app-root>/a/b.rw`. Nincs `mod.rw` alternatíva.

```rwlang
mod models;
mod pages::article;
```

A modulútvonal mindig application-root relatív. A `../`, `./`, `self::`, `super::` és `crate::` nem támogatott.

A `mod` az alkalmazás source graph tagságát deklarálja, nem lexikális importlista. Ha a `main.rw` vagy egy másik már betöltött source unit betöltött egy modult, bármely betöltött source unit hivatkozhat rá az abszolút namespace útvonalával. A top-level modulgráfot célszerű a `main.rw`-ben összeállítani.

A `mod` top-level source-graph deklaráció, ezért az adott source unit normál deklarációi előtt kell szerepelnie. A könyvtárakat a compiler nem járja be automatikusan: a `mod catalog;` csak a `catalog.rw` fájlt tölti be, a `catalog/queries.rw` fájlhoz külön `mod catalog::queries;` szükséges. V1-ben nincs olyan `use` vagy wildcard import sem, amely egy másik modul neveit az aktuális scope-ba emelné.

## Szimbólumazonosság

A nem root modulban deklarált enum, model, form, query, component, layout, page és action a modul névterébe kerül.

`pages/article.rw`:

```rwlang
page fn show(ctx: PageContext, slug: Slug) -> Result<Html, PageError> {
    return Ok(html {<h1>{{ slug }}</h1>});
}
```

A teljes neve:

```text
pages::article::show
```

A saját modulon belül a lokális deklaráció rövid névvel is hivatkozható, és az aktuális modul névterében oldódik fel. Modulhatár átlépésekor kötelező a teljes namespace. Külső hivatkozás:

```rwlang
route articleShow GET "/articles/:slug<Slug>" => pages::article::show;
```

A `mod pages::article;` tehát nem hoz létre globális `show` nevet.

A route neve továbbra is alkalmazásszintű HTTP-azonosító, ezért globálisan egyedi marad és nem kap automatikusan modulprefixet.

## Modulok közötti használat

```rwlang
// queries.rw
query fn byId(db: Db, id: Int) -> Result<models::Article, DbError> sql {
    SELECT id, title FROM articles WHERE id = :id
}
```

```rwlang
// pages.rw
page fn show(ctx: PageContext, db: Db, id: Int) -> Result<Html, PageError> {
    let article = queries::byId(db, id)?;
    return Ok(html {<h1>{{ article.title }}</h1>});
}

route articleShow GET "/articles/:id<Int>" => pages::show;
```

A teljes név szándékosan explicit: a dependency látható marad, és két modul azonos helyi neve nem ütközik.

## Domain objectek

A modulnévtér és a domain-object member jelölés együtt használható:

```rwlang
content::article::Article.bySlug(...)
```

A modul separator `::`, a domain-object member separator továbbra is `.`.
