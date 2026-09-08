# Transaction outcome semantics

Critical database operations cannot safely collapse every failure into a generic database error.

Captured transactions use a closed outcome model:

- `Committed`: commit acknowledgement was received;
- `RolledBack`: non-commit/rollback is known;
- `CommitUnknown`: commit or rollback acknowledgement is unavailable and retry must not be assumed safe.

Example:

```rwlang
let outcome = transaction db {
    charge(tx)?;
};

match outcome {
    Committed => { return Ok(json(true)); }
    RolledBack => { fail conflict; }
    CommitUnknown => { fail conflict; }
}
```

For critical operations that require both transaction and idempotency, the compiler requires transaction outcome capture and exhaustive handling. This prevents an unknown commit state from being reduced to a normal retryable failure.
