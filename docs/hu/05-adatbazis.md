# 5. SQLite, PostgreSQL, MariaDB és typed SQL

## Backend konfiguráció

SQLite:

```text
sqlite:///srv/myapp/dev.db
```

PostgreSQL:

```text
postgres://app_user:password@db.internal:5432/appdb?sslmode=require
```

MariaDB:

```text
mysql://app_user:password@db.internal:3306/appdb
```

Productionban:

```bash
--db-url-file /run/secrets/database-url
```

## Egy sor

```text
query fn loadProduct(db: Db, id: Int) -> Result<Product, DbError> sql {
    SELECT id, name, price
    FROM products
    WHERE id = :id
}
```

Cardinality:

```text
Product       = pontosan 1 sor
Product?      = 0 vagy 1 sor
List<Product> = 0..N sor
```

## Lista

```text
query fn listProducts(db: Db, limit: Int, offset: Int)
    -> Result<List<Product>, DbError> sql {
    SELECT id, name, price
    FROM products
    ORDER BY id
    LIMIT :limit OFFSET :offset
}
```

Mindig paginálj nagy listát.

## SQL injection

Csak typed bind:

```text
WHERE email = :email
```

Ne építs SQL fragmentet user inputból. A runtime backend placeholderre fordítja a bindet (`$1`/`?`) és SQLx `.bind(...)` paraméterként küldi.

## Row shape

A SELECT/RETURNING mezők neve és sorrendje egyezzen a modellel.

```text
model Product { id, name, price }
SELECT id, name, price ...    // jó
```

## Migráció

A server nem futtat automatikus production migrációt. Használd a külön migration CLI-t és külön credentialt: [Database migration workflow](10-database-migrations.md).
