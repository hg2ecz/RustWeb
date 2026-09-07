# Expiry-aware token verification

Session and password-reset credentials have a time-bounded lifecycle. RWLang therefore does not allow equality-only verification for these token purposes.

Use `tokenActive(storedHash, presentedToken, expiresAt)`:

```rw
model Session {
    id: Int
    tokenHash: Secret<SessionTokenHash>
    expiresAt: DateTime
}

let valid = tokenActive(session.tokenHash, presentedSessionToken, session.expiresAt);
```

The compiler requires:

1. `Secret<SessionTokenHash>` with `SessionToken`, or `Secret<PasswordResetTokenHash>` with `PasswordResetToken`;
2. a validated presented token of the matching purpose;
3. an explicit `DateTime` expiry value.

`tokenMatches(...)` is intentionally reserved for CSRF token equality. Using it with session or password-reset hashes is a compile-time security error (`SEC-A07-004`), so applications cannot accidentally omit expiry checks for those credentials.

At runtime `tokenActive(...)` hashes the presented bearer token with SHA-256, compares the fixed-size hashes in constant time, and returns `false` when `expiresAt <= now`.

This primitive proves only two facts at runtime: the bearer value matches the persisted hash, and the credential has not expired. The returned `Bool` is deliberately not an authorization or consume-once proof. Password-reset consumption and session rotation/revocation are separate lifecycle transitions and must not be inferred from a boolean equality result.
