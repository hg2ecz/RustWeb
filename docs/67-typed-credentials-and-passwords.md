# Typed credentials and password primitives

RWLang deliberately exposes high-level password operations instead of raw cryptographic algorithms.

```rw
let hash = passwordHash(password);
let valid = passwordVerify(account.passwordHash, password);
```

`passwordHash(Password)` accepts a validated purpose-typed password and produces statically classified `Secret<PasswordHash>` data. The runtime uses Argon2id with a fresh random salt and policy-owned parameters. The language does not expose algorithm, salt, or cost knobs in ordinary application code.

`passwordVerify(Secret<PasswordHash>, Password)` requires the exact password-hash purpose and a validated purpose-typed password input. Its result is a public `Bool`; the secret classification does not leak through the comparison result.

These operations do not create authorization evidence. Loading an account and checking or changing its password remains subject to the normal authentication/authorization contracts.

Raw password hashing primitives are intentionally absent from the normal RWLang builtin surface. Nominal domain types such as `PasswordResetToken`, `ApiToken`, and `SessionToken` should remain distinct even when they share a String representation.
