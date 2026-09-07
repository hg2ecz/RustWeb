# Domain types and bounded collections

RWLang domain input types let an application name validation rules once and reuse them at web trust boundaries.

```rw
type Username = String {
    length 3 32;
    pattern "^[A-Za-z0-9_]+$";
}

type PageSize = Int {
    range 1 100;
}
```

A route can then use the domain name directly:

```rw
route profile GET "/profiles/:username<Username>"
    query size<PageSize>
    public => profile;
```

The compiler expands the domain constraints into the route contract. The runtime validates them before entering the handler, so handler parameters arrive with validation evidence. A raw `String` still receives RWLang's default `0..4096` input bound; a domain `length` constraint replaces that broad default with the application's narrower contract.

Domain types currently use `String` or `Int` as their base. `String` supports `length` and `pattern`; `Int` supports `range`. A string domain maximum cannot exceed 4096 characters. Constraint kinds cannot be duplicated. Semicolons and commas inside the constraint body are optional separators.

Domain names now keep nominal identity throughout compiler-visible data flow. `UserId` and `ArticleId` are different types even if both use `Int` as their runtime representation. The runtime erases that nominal wrapper only after compilation, so there is no wrapper allocation or serialization overhead.

## Bounded string-list construction

`splitBounded(text, delimiter, maxItems)` constructs a `List<String>` with an explicit fail-closed item bound:

```rw
let tags = splitBounded(raw, ",", 32);
```

`maxItems` must be an integer literal in `1..4096`. If the input would produce more elements, evaluation fails instead of silently truncating. Requiring a literal makes the resource bound visible to the compiler, reviewer, and reader at the call site.

The older `split` remains globally capped at 4096 items for compatibility, but security-sensitive code should prefer the narrower `splitBounded` form when the domain has a known cardinality limit.
