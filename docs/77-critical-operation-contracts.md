# Critical operation contracts

Critical operations let an application name a security-sensitive business action once and let the compiler enforce its required controls.

```rw
permission BillingWrite {
    role Admin
    role BillingAdmin
}

critical Payment {
    permission BillingWrite
    mfa
    transaction
    audit
}
```

A handler opts into the contract with one short declaration:

```rw
action fn refund(ctx: ActionContext, db: Db, id: Int) -> Result<Json, PageError> critical Payment {
    transaction db {
        audit Payment id action refund;
    }
    return Ok(json(true));
}
```

The route may reuse the same intent instead of repeating permission and MFA details:

```rw
route refund POST "/billing/:id<Int>/refund"
    auth critical Payment
    => refund;
```

`auth critical Payment` compiles to the platform-owned authentication policy implied by the contract. A critical operation is always authenticated; permission and MFA requirements strengthen that baseline.

The compiler fails closed when a required transaction or audit record is missing. Audit values still obey the existing `Secret<T>` and `Sensitive<T>` leakage rules.

This keeps the normal path short: developers name the business risk, while RWLang owns the security mechanics.
