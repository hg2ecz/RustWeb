# Bounded request collections

RWLang treats external collection cardinality as a security boundary. `List<String>` request fields are bounded by default and may be narrowed explicitly.

```rw
page fn search(ctx: PageContext, tags: List<String>) -> Result<Json, PageError> {
    return Ok(json(len(tags)));
}

route search GET "/search"
    query tags<List<String>>
    validate tags items 1 16
    public => search;
```

The safe default is `items 1 64` when no explicit `items` rule is present. The explicit rule replaces that default rather than stacking another limit.

## Input representation

- Query and form inputs use repeated fields: `?tags=rust&tags=web`.
- JSON inputs use a real string array: `{ "tags": ["rust", "web"] }`.
- CSV-in-a-string is not the collection contract; no manual `split()` is required for normal request collections.

JSON arrays are hard-capped during deserialization before the handler runs. Route validation may then apply a smaller domain limit.

## Security properties

- Duplicate scalar fields still fail closed.
- Repeated values are accepted only for `List<String>` fields.
- Request collections become `Validated<List<String>>` only when their route contract proves a cardinality bound.
- The platform absolute collection ceiling is 256 items; the default route ceiling is 64.
- Empty JSON string arrays are rejected in the current wire format; use at least one item when the field is present.

RWLang currently has no general unbounded user-data loop construct. This iteration establishes the bounded request boundary first; later collection transforms/iteration must preserve or reduce the bound rather than erase it.
