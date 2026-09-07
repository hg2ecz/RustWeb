# MFA elevation proofs

RWLang can require recent MFA elevation at the handler boundary without exposing proof wrappers or session plumbing to application code.

## MFA-only sensitive handlers

```rw
action fn disableMfa(
    ctx: ActionContext
) -> Result<Json, PageError> requires mfa {
    return Ok(json(true));
}

route disableMfa POST "/account/mfa/disable"
    auth mfa
    => disableMfa;
```

The compiler rejects a weaker `auth user` route. The server uses the platform-owned session and its `mfa_verified` state; applications do not implement a second MFA mechanism.

## Permission plus MFA

```rw
permission BillingWrite {
    role Admin
    role BillingAdmin
}

action fn refund(
    ctx: ActionContext,
    id: Int
) -> Result<Json, PageError> requires BillingWrite + mfa {
    return Ok(json(true));
}

route refund POST "/billing/:id<Int>/refund"
    auth permission BillingWrite mfa
    => refund;
```

Both conditions are mandatory at runtime: the session must hold a role granting `BillingWrite` and it must be MFA-verified.

## Fail-closed rule

`requires ... + mfa` is a handler security contract, not documentation. A route that grants the permission but omits MFA fails compilation with `SEC-A07-020`.

The secure path is intentionally short: application code declares intent and the compiler/server enforce the platform session policy.
