# Atomikus token-életciklus szerződések

Az RWLang a bearer tokenek életciklus-módosításait nem közönséges adatbázis-törlésként, hanem explicit compiler-szerződésként kezeli.

## Bemutatott token proof

Requestből érkező tokenből csak ezzel készülhet lifecycle lookup hash:

```rw
let lookupHash = presentedTokenHash(token);
```

Az eredmény validált, purpose-pontos hash és külön lifecycle bizonyítékot hordoz. Ez szándékosan más, mint a `tokenHash(...)`, amely kizárólag frissen kiadott secret token persistálás előtti hashelésére való.

## Egyszer használható password reset

```rw
query fn consumeReset(
    tx: Transaction,
    tokenHash: PasswordResetTokenHash
) -> Result<Changed, DbError>
    consumes ResetGrant by tokenHash before expiresAt
sql {
    DELETE FROM reset_grants
    WHERE token_hash = :tokenHash
      AND expires_at > CURRENT_TIMESTAMP
}
```

A compiler megköveteli a `DELETE` műveletet, a `Changed` visszatérést, a purpose-pontos secret hash mezőt, a hash-egyezést és a DB-időhöz kötött lejárati feltételt. A `Changed` pontosan egy módosított sort követel, ezért a reset grant atomikusan csak egyszer fogyasztható el.

## Session visszavonás

A `revokes Session by tokenHash` szerződés ugyanígy atomikus `DELETE + Changed` műveletet követel. A hívás csak a `presentedTokenHash(sessionToken)` változatlan eredményét fogadja el; egy azonos típusú, de más eredetű hash nem hordoz lifecycle proofot.
