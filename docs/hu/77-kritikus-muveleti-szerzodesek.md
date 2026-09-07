# Kritikus műveleti szerződések

A kritikus művelet egy névvel ellátott üzleti biztonsági szerződés. Egyszer deklaráljuk, milyen védelmek kötelezőek, a compiler pedig minden használatnál kikényszeríti őket.

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

A handler röviden csak a szándékot jelöli:

```rw
action fn refund(ctx: ActionContext, db: Db, id: Int) -> Result<Json, PageError> critical Payment {
    transaction db {
        audit Payment id action refund;
    }
    return Ok(json(true));
}
```

A route sem ismétli meg a permission/MFA részleteket:

```rw
route refund POST "/billing/:id<Int>/refund"
    auth critical Payment
    => refund;
```

Az `auth critical Payment` a szerződésből vezeti le a platform-owned auth policyt. Minden kritikus művelet legalább autentikációt követel; a permission és MFA ezt tovább szigorítja.

Ha a kötelező tranzakció vagy audit hiányzik, a program nem fordul le. Az audit továbbra sem fogad `Secret<T>` vagy nem megfelelően kezelt érzékeny adatot.
