# Cryptographic key purpose and lifecycle

RWLang distinguishes cryptographic key purpose and rotation state in the type system.

Examples:

```text
SigningKey<Webhook>
RetiringSigningKey<Webhook>
RetiredSigningKey<Webhook>
VerificationKey<Webhook>
EncryptionKey<UserData>
RetiringEncryptionKey<UserData>
RetiredEncryptionKey<UserData>
```

Key material remains classified as `Secret<T>` and cannot arrive from a web request boundary.

Webhook signing accepts only an active signing key. Signature verification may use active or retiring verification keys during rotation, but never retired keys. User-data encryption/decryption semantics are documented separately in `96-authenticated-encryption-key-lifecycle.md`.
