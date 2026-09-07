# Atomikus session rotáció

Az RWLangban a session rotáció credential-életciklus állapotátmenet, nem közönséges update query.
A nyelv egyszerre követeli meg a régi, bemutatott token bizonyítékát és az új, frissen kiadott token bizonyítékát.

```rw
model Session {
    id: Int
    tokenHash: Secret<SessionTokenHash>
    expiresAt: DateTime
}

query fn rotateSession(
    tx: Transaction,
    tokenHash: SessionTokenHash,
    newTokenHash: Secret<SessionTokenHash>,
    expiresAt: DateTime
) -> Result<Changed, DbError>
    rotates Session by tokenHash to newTokenHash until expiresAt
sql {
    UPDATE sessions
    SET token_hash = :newTokenHash,
        expires_at = :expiresAt
    WHERE token_hash = :tokenHash
      AND expires_at > CURRENT_TIMESTAMP
}
```

A hívási oldalon a régi hash csak validált, bemutatott bearer tokenből származhat:

```rw
let oldHash = presentedTokenHash(token);
```

Az új hash pedig kizárólag frissen kiadott tokenből:

```rw
let newToken = newSessionToken();
let newHash = tokenHash(newToken);
```

Az állapotátmenet ezután egy tranzakción belül atomikus:

```rw
transaction db {
    let changed = rotateSession(tx, oldHash, newHash, expiresAt)?;
}
```

A compiler elutasítja a rotációt, ha:

- a query nem `UPDATE`;
- a visszatérés nem `Changed`;
- a régi hash nem hordoz presented-token proofot;
- az új hash nem hordoz friss issued-token-hash proofot;
- az SQL nem cseréli le egyszerre a token hasht és az expiry-t;
- a `WHERE` nem a régi token hashre illeszt;
- a régi expiry nincs `CURRENT_TIMESTAMP` ellen védve.

A `tokenHash(...)` maga is `IssuedToken` proofot követel. Tárolóból visszaolvasott `Secret<SessionToken>` nem nevezhető át friss replacement tokenné egyszerű újrahasheléssel.

## Miért külön lépés a cookie delivery?

A beépített RWLang szerver már birtokolja az autentikációs cookie-t (`__Host-rw_session`). Egy második alkalmazásszintű session cookie két versengő session-authorityt hozna létre. A biztonságos böngészős deliveryt ezért a meglévő auth/session alrendszerrel kell integrálni, nem párhuzamos cookie-modellt bevezetni.
