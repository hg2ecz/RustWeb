# Nominal domain types

RWLang domain types are nominal, not aliases.

```rw
type UserId = Int { range 1 999999999; }
type ArticleId = Int { range 1 999999999; }
```

Even though both use `Int` at runtime, they are different compile-time types. This is rejected:

```rw
query fn loadUser(db: Db, id: UserId) -> Result<User?, DbError> sql {
    SELECT id FROM users WHERE id = :id
}

let user = loadUser(db, articleId)?;
```

The compiler reports `SEC-TYPE-001` and names both domain types. A raw `Int` literal is not silently promoted to `UserId` either.

Nominal identity is preserved across route parameters, handler parameters, model fields, query parameters, local aliases, typed redirects, and typed HTML route helpers. This prevents accidental cross-object identifier use and strengthens RWLang's authorization-proof model.

Representation-consuming safe operations remain ergonomic. A `Username = String { ... }` can be HTML-escaped, measured with `len`, split with `splitBounded`, or passed to other safe string operations. These operations consume its `String` representation and may return a primitive result; they do not implicitly manufacture a new validated `Username`.

String domain types also receive a fail-closed `length 0 4096` constraint when the declaration only provides another constraint such as `pattern`. Application-specific shorter bounds should still be declared when known.

At runtime, domain values use their base scalar representation. The nominal identity exists in the compiled program's type contract, not as a heap wrapper, so the stronger type safety does not add per-value allocation overhead.
