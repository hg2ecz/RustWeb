# Nominális domain típusok

Az RWLang domain típusai valódi nominális típusok, nem egyszerű aliasok.

```rw
type UserId = Int { range 1 999999999; }
type ArticleId = Int { range 1 999999999; }
```

Bár mindkettő runtime reprezentációja `Int`, fordításkor külön típusok. Ez hibás:

```rw
query fn loadUser(db: Db, id: UserId) -> Result<User?, DbError> sql {
    SELECT id FROM users WHERE id = :id
}

let user = loadUser(db, articleId)?;
```

A compiler `SEC-TYPE-001` hibát ad, és az üzenetben a forrásbeli `UserId` illetve `ArticleId` neveket mutatja. Nyers `Int` literál sem alakul át csendben `UserId` értékké.

A nominális azonosság megmarad route paramétereken, handler paramétereken, modellmezőkön, query paramétereken, lokális aliasokon, typed redirecteken és typed HTML route helperen keresztül. Ez megakadályozza a különböző objektumazonosítók véletlen összekeverését, és erősíti az authorization-proof modellt.

A biztonságos, reprezentációt fogyasztó műveletek továbbra is kényelmesek. Egy `Username = String { ... }` használható HTML escapingben, `len`-nel, `splitBounded`-dal és más biztonságos string műveletekkel. Ezek a `String` reprezentációt fogyasztják, de nem gyártanak automatikusan új validált `Username` értéket.

Ha egy String domain csak például `pattern` constraintet deklarál, az RWLang automatikusan hozzáad egy fail-closed `length 0 4096` korlátot. Ha az üzleti domain ennél szűkebb, azt továbbra is érdemes explicit deklarálni.

Runtime oldalon a domain értékek az alap skalár reprezentációt használják. A nominális azonosság a lefordított program típuskontraktjában él, nem heap wrapperként, ezért az erősebb típusbiztonság nem ad értékenkénti allokációs költséget.
