# Typed security events

RWLang security events are declared once and tied to a model:

```rw
security event RoleGranted for User;
```

Inside a transaction, emit the event with a typed object identifier:

```rw
security RoleGranted user.id;
```

Optional public, non-sensitive state transitions may be recorded:

```rw
security RoleChanged user.id from oldRole to newRole;
```

`Secret<T>` and `Sensitive<T>` values remain forbidden in security event fields. Critical operations may require a specific event:

```rw
critical RoleChange {
    permission UserAdmin
    mfa
    transaction
    audit RoleGranted
}
```

A different business audit or a different security event does not satisfy that contract. This keeps the normal application code short while making security-sensitive state changes observable by construction.
