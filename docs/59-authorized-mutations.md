# Authorized mutations

RWLang treats object identity as part of the authorization proof for `UPDATE` and `DELETE` queries.

A mutating query declares its protected model and key:

```rwlang
query fn updateArticle(
    tx: Transaction,
    id: Int,
    title: String
) -> Result<Void, DbError> mutates Article by id sql {
    UPDATE articles SET title = :title WHERE id = :id
}
```

`UPDATE` and `DELETE` without a mutation contract are rejected with `SEC-A01-006`.

The call must pass a key that comes from an authorized instance of the declared model:

```rwlang
let article = articleById(db, id)?;
authorize article owner authorUsername or role Editor;

transaction db {
    updateArticle(tx, article.id, title)?;
}
```

Passing the request parameter directly is rejected with `SEC-A01-005`:

```rwlang
updateArticle(tx, id, title)?; // rejected
```

Unchanged local aliases preserve the proof:

```rwlang
let articleId = article.id;
updateArticle(tx, articleId, title)?; // accepted
```

Transformations intentionally destroy the proof:

```rwlang
let otherId = article.id + 1;
updateArticle(tx, otherId, title)?; // rejected
```

A proof from another model is not interchangeable, even when the key type is identical.

## Shared objects

For application objects that are intentionally writable by every authenticated principal, use an explicit policy:

```rwlang
authorize product authenticated;
```

This still requires an authenticated route and produces object-specific proof without inventing a fake owner field.

## Fresh objects

A model returned from an `INSERT ... RETURNING` query is treated as fresh authority inside the same flow. This keeps create-then-update transaction code concise while preventing request-controlled identifiers from becoming mutation authority.
