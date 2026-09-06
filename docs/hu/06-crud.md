# 6. CRUD és tranzakciók

Mutáció csak `Transaction` capability-vel:

```text
query fn updateProduct(
    tx: Transaction,
    id: Int,
    name: String,
    price: Int
) -> Result<Void, DbError> sql {
    UPDATE products
    SET name = :name, price = :price
    WHERE id = :id
}
```

Action:

```text
action fn update(
    ctx: ActionContext,
    db: Db,
    id: Int,
    name: String,
    price: Int
) -> Result<Redirect, PageError> {
    transaction db {
        updateProduct(tx, id, name, price)?
    }
    return Ok(redirect("/products?page=1&pageSize=20"));
}
```

Siker → commit. Query/runtime hiba → rollback. A `Transaction` nem escape-elhet a blokkból.

Hordozható SQLite/PostgreSQL/MariaDB CRUD-nál a `Result<Void, DbError>` jó közös minimum; backend-specifikus `RETURNING` csak tudatosan.
