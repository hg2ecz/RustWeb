# Idempotent critical operations

RWLang can require replay-safe execution for authenticated state-changing routes without exposing Redis or locking code to application authors.

```rw
critical Payment {
    transaction
    audit
    idempotency
}

action fn charge(ctx: ActionContext, db: Db) -> Result<Json, PageError> critical Payment {
    transaction db {
        // state change + audit
    }
    return Ok(json(true));
}

route charge POST "/charge"
    auth critical Payment
    idempotent
    => charge;
```

Clients send an `Idempotency-Key` header. Keys are 16..128 characters and may contain ASCII letters, digits, `-`, `_`, `.`, and `:`.

The platform scopes the key by domain, route, and authenticated principal. It also fingerprints method, target, content type, and request body. Redis `SET NX` provides the atomic claim. A completed non-5xx response is stored for 24 hours and replayed for the same key and request. Reusing the key for a different request is rejected. Concurrent duplicates receive a conflict while the first request is still pending.

Idempotent routes require Redis. There is intentionally no in-memory production fallback: distributed replay protection must remain correct across processes and restarts.

If execution ends in a 5xx state, the claim remains pending instead of permitting an automatic retry. This is fail-closed because the platform cannot prove that no external or database side effect occurred before the failure.

Security properties:

- POST only;
- authenticated routes only;
- critical contracts can require idempotency at compile time;
- principal-scoped keys prevent cross-account key collisions;
- request fingerprints prevent key reuse with changed payloads;
- Redis-backed atomic claims prevent concurrent duplicate execution;
- successful responses can be replayed without re-running the handler;
- uncertain 5xx outcomes do not silently re-run side effects.
