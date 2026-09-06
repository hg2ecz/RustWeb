# Modules and namespaces

RWLang modules are application-root-relative namespaces. A module declaration loads a source unit; it does not inject that unit's declarations into the current or global namespace.

## Canonical mapping

```text
main.rw                application root
models.rw              models
pages/article.rw       pages::article
admin/users/edit.rw    admin::users::edit
```

There is exactly one source mapping: `a::b` resolves to `<app-root>/a/b.rw`. RWLang does not use `mod.rw`.

```rwlang
mod models;
mod pages::article;
```

Module paths use `::`, are always rooted at the application directory, and may not use filesystem traversal. `../`, `./`, `self::`, `super::`, and `crate::` are rejected.

`mod` defines membership in the application's source graph, not a lexical import list. Once `main.rw` (or another loaded source unit) has loaded a module, any loaded source unit may refer to that module by its absolute namespace path. Applications should normally compose their top-level source graph from `main.rw`.

Module declarations are top-level source-graph declarations and must appear before ordinary declarations in that source unit. Directories are never scanned automatically: `mod catalog;` loads `catalog.rw`, but it does not implicitly load `catalog/queries.rw`; that requires an explicit `mod catalog::queries;`. V1 also has no `use`/wildcard import mechanism that injects another module's symbols into the current scope.

## Symbol identity

Declarations in a non-root module belong to that module namespace. For example, `pages/article.rw`:

```rwlang
page fn show(ctx: PageContext, slug: Slug) -> Result<Html, PageError> {
    return Ok(html {<h1>{{ slug }}</h1>});
}
```

has the symbol name `pages::article::show`.

Inside the defining module, a local declaration may be referenced by its short name and resolves in that module namespace. Crossing a module boundary requires the qualified namespace path. For example, an external reference is explicit:

```rwlang
route articleShow GET "/articles/:slug<Slug>" => pages::article::show;
```

`mod pages::article;` does **not** make `show` a global name.

This rule applies to enums, models, forms, queries, components, layouts, pages, and actions. Route names remain application-level HTTP identifiers and therefore remain globally unique rather than inheriting a module namespace.

## Cross-module example

```text
main.rw
models.rw
queries.rw
pages.rw
```

`main.rw`:

```rwlang
mod models;
mod queries;
mod pages;
```

`models.rw`:

```rwlang
model Article {
    id: Int
    title: String
}
```

`queries.rw`:

```rwlang
query fn byId(db: Db, id: Int) -> Result<models::Article, DbError> sql {
    SELECT id, title FROM articles WHERE id = :id
}
```

`pages.rw`:

```rwlang
page fn show(ctx: PageContext, db: Db, id: Int) -> Result<Html, PageError> {
    let article = queries::byId(db, id)?;
    return Ok(html {<h1>{{ article.title }}</h1>});
}

route articleShow GET "/articles/:id<Int>" => pages::show;
```

The explicit namespace is intentional: dependencies remain visible in source and name collisions between modules do not silently change meaning.

## Domain objects

Domain-object member notation composes with module namespaces. If `content/article.rw` declares `object Article { ... query fn bySlug ... }`, the external symbol is written as:

```rwlang
content::article::Article.bySlug(...)
```

The module separator is `::`; the domain-object member separator remains `.`.
