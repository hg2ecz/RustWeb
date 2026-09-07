# Explicit public projections

RWLang does not allow loaded model values to cross a JSON response boundary implicitly. Public API output is an explicit field projection:

```rw
return Ok(json(expose(user, id, displayName)));
```

`expose(value, field, ...)` accepts a model, optional model, or model list and requires at least one named model field. Unknown and duplicate fields are compile errors. The runtime emits only the selected fields; adding a new field to the underlying model therefore cannot silently expand an existing API response.

## Data classification

`Public` fields may be projected directly. `Sensitive<T>` fields require object-authorization proof on the loaded model before projection. `Secret<T>` fields are never projectable, even after authorization.

```rw
model User {
    id: UserId
    email: Sensitive<Email>
    passwordHash: Secret<String>
    owner: String
}

let user = loadUser(db, id)?;
authorize user owner owner;
return Ok(json(expose(user, id, email)));
```

Trying to expose `passwordHash` fails with `SEC-DATA-005`; exposing `email` without the preceding authorization fails with `SEC-A01-012`.

## Lists

The same syntax projects every item in a model list:

```rw
let products = listProducts(db)?;
return Ok(json(expose(products, id, name, price)));
```

This avoids whole-record serialization and makes the response contract visible in code review.

## Security invariant

Direct model serialization such as `json(user)` or `json(products)` is rejected with `SEC-DATA-004`. Public declassification is intentionally a response-boundary construct rather than a general expression, so internal code cannot erase sensitivity metadata by calling a convenience conversion.
