# Atomic token lifecycle contracts

RWLang treats bearer-token lifecycle mutations as explicit compiler contracts rather than ordinary database deletes.

## Presented-token proof

A request token may be converted to a lookup hash only with:

```rw
let lookupHash = presentedTokenHash(token);
```

This produces a validated, purpose-matched token-hash value carrying lifecycle evidence. It is intentionally different from `tokenHash(...)`, which only accepts newly issued secret tokens before persistence.

## Consume-once password reset

```rw
model ResetGrant {
    id: Int
    tokenHash: Secret<PasswordResetTokenHash>
    expiresAt: DateTime
}

query fn consumeReset(
    tx: Transaction,
    tokenHash: PasswordResetTokenHash
) -> Result<Changed, DbError>
    consumes ResetGrant by tokenHash before expiresAt
sql {
    DELETE FROM reset_grants
    WHERE token_hash = :tokenHash
      AND expires_at > CURRENT_TIMESTAMP
}
```

The compiler requires `DELETE`, `Changed`, a purpose-matched secret hash field, hash equality, and an expiry predicate against database time. `Changed` enforces exactly one affected row, so a reset grant can be consumed once atomically.

## Session revocation

```rw
query fn revokeSession(
    tx: Transaction,
    tokenHash: SessionTokenHash
) -> Result<Changed, DbError>
    revokes Session by tokenHash
sql {
    DELETE FROM sessions WHERE token_hash = :tokenHash
}
```

The call must receive the unchanged result of `presentedTokenHash(sessionToken)`. A same-typed hash from another source does not carry lifecycle evidence.

These contracts deliberately avoid a separate verify-then-delete flow and therefore avoid the associated TOCTOU/replay window.
