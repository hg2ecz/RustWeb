# Domain value refinement

RWLang nominal domain types are not casts. A primitive value does not silently become a domain value even when both share the same runtime representation.

Use explicit, fail-closed refinement:

```rw
let userId = validate UserId(raw)?;
```

The compiler requires `raw` to have the same safe runtime representation as the domain base type. At runtime every constraint declared on `UserId` is checked before the value is bound to `userId`. Failure aborts the request with a bad-request result.

This produces a validation proof only. It does not produce authorization evidence. A validated `UserId` proves that the value satisfies the `UserId` contract; it does not prove that the current principal may access the corresponding `User` object.

Refinement preserves data sensitivity/disclosure metadata but intentionally drops mutation authorization evidence. This prevents a value from being re-labelled into another nominal ID domain while retaining authority derived from a different object.

There is no implicit `UserId(raw)` cast and no unchecked domain constructor in normal RWLang code.
