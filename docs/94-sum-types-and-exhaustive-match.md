# Closed sum types and exhaustive match

RWLang's existing enum representation is a first-class closed sum type for control-flow purposes.

```rwlang
enum CommitState {
    Committed
    RolledBack
    CommitUnknown
}

match state {
    Committed => { ... }
    RolledBack => { ... }
    CommitUnknown => { ... }
}
```

Compiler invariants:

- only a sum/enum value can be matched;
- unknown variants are rejected;
- duplicate arms are rejected;
- every declared variant must be handled;
- no wildcard arm hides future variants;
- return contracts, effects, authorization, critical-operation checks and resource analysis traverse every arm.

Adding a new variant therefore turns old incomplete handling into a compile-time error rather than silently selecting a fallback branch.
