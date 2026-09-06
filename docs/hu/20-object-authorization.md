# Objektumszintű authorization

A route auth azt dönti el, hogy a kérés beléphet-e a handlerbe. Az objektumszintű guard azt, hogy a konkrét betöltött rekordhoz hozzáférhet-e az actor.

## Alapminta

```rwlang
model Article {
    id: Int
    authorUsername: String
    title: String
}

query fn loadArticle(db: Db, id: Int) -> Result<Article, DbError> sql {
    SELECT id, authorUsername, title FROM articles WHERE id = :id
}

page fn article(ctx: PageContext, db: Db, id: Int) -> Result<Html, PageError> {
    let article = loadArticle(db, id)?;
    authorize article owner authorUsername or role Publisher;
    return Ok(html {<h1>{{ article.title }}</h1>});
}

route article GET "/articles/:id<Int>" auth user => article;
```

A compiler ellenőrzi, hogy `article` valóban már betöltött, nem optional model record és `authorUsername` `String`. Optional modelt előbb explicit módon kell kezelni; authorization nem értelmez `Null` rekordot. A guardot használó handler route-ja nem lehet public, és public cache sem kombinálható object authorizationnel.

## Módosítás

```rwlang
action fn articleEdit(
    ctx: ActionContext,
    db: Db,
    id: Int,
    title: String
) -> Result<Redirect, PageError> {
    let article = loadArticle(db, id)?;
    authorize article owner authorUsername or role Publisher;

    transaction db {
        updateTitle(tx, id, title)?;
    }

    return Ok(redirect("/articles"));
}
```

Az authorization query read-only `Db` query. A módosítás továbbra is csak `Transaction` capabilityvel történhet.

### Ownership szabály

M35-ben az owner mező **stabil, nem módosítható authority attribútumként** használandó. Ne implementálj normál owner-guardos actionnel ownership transfert. Átadás/átvétel külön privileged (`auth role ...`, indokolt esetben MFA) workflow legyen.

Ez a megkötés fontos, mert általános mutable ACL esetén a betöltés és későbbi update között concurrency race jöhetne létre. Ha az alkalmazás mutable ACL-t igényel, arra külön későbbi policy/transaction mechanizmust használjunk.

## Role override

```rwlang
authorize article owner authorUsername or role Publisher or role Admin;
```

A szerepkör a session trusted auth backendből jön. Form, JSON, query vagy path paraméter soha nem szerepkör-authority.

## Tiltott minták

Public route:

```rwlang
route article GET "/articles/:id<Int>" => article;
```

ha a handler `authorize` guardot tartalmaz, compile error. Legalább `auth user`, `auth mfa` vagy `auth role ...` szükséges.

Public cache és object authorization szintén nem keverhető.

## Audit

Deny: HTTP `403`. Autentikált state-changing action deny auditálódik. Autentikált GET 403 is security activityként auditálódik. A rekord tartalma vagy owner mező értéke nincs az audit rekordban.
