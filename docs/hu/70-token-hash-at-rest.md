# Token hash tároláskor

Az RWLang bearer tokeneket purpose-specifikus secret credentialként adja ki, de a tartós ellenőrzési állapot külön, purpose-specifikus hash típust használ.

```rw
let token = newSessionToken();
let storedHash = tokenHash(token);
```

A statikus típusok:

```text
newSessionToken()        -> Secret<SessionToken>
tokenHash(sessionToken)  -> Secret<SessionTokenHash>
```

Ugyanez létezik password-reset és CSRF tokenhez:

- `SessionTokenHash`
- `PasswordResetTokenHash`
- `CsrfTokenHash`

A nyers token és a hash purpose nem felcserélhető.

## Tárolási szerződés

A hash kerüljön adatbázisba, ne maga a bearer token:

```rw
query fn storeSession(
    tx: Transaction,
    tokenHash: Secret<SessionTokenHash>
) -> Result<Void, DbError> sql {
    INSERT INTO sessions(token_hash) VALUES(:tokenHash)
}
```

A generált nyers token csak `tokenHash(...)` után adható ilyen sinknek. Requestből bemutatott tokenből pedig nem lehet ezzel a primitívvel tartós, trusted token-hash credentialt készíteni.

## Ellenőrzési szerződés

A `tokenMatches(storedHash, presented)` csak CSRF token hashhez használható egyezőségvizsgálat. Session és password-reset tokenhez a `tokenActive(storedHash, presented, expiresAt)` kötelező. Mindkét forma megköveteli:

1. első argumentumként a secret, purpose-specifikus token hash-t;
2. második argumentumként az ugyanahhoz a purpose-höz tartozó validált nyers tokent.

```rw
let valid = tokenActive(session.tokenHash, presentedSessionToken, session.expiresAt);
```

A runtime SHA-256-tal hasheli a bemutatott tokent, majd a fix méretű hex hash-eket constant-time összehasonlítással ellenőrzi. A generált bearer token 256 bit véletlen entrópiát tartalmaz, ezért a tartós hash nem hoz létre gyakorlatban kereshető alacsony entrópiájú credential-teret.

## Biztonsági határ

A hashing a nyelv normál felületén egyirányú. Nincs hashből token konverzió és nincs generic credential rendering API.

A `tokenHash(...)` csak kiadott secret tokent fogad. Így egy requestből érkező tetszőleges string hashelése nem használható trusted persistence credential előállítására.

## Lifecycle hatókör

Ez a réteg a biztonságos tartós tokenazonosságot adja. Még nem bizonyítja:

- a lejáratot;
- az egyszer használhatóságot;
- a session rotationt;
- a revocationt;
- a protokollszintű kézbesítést.

Ezek a következő lifecycle rétegben már úgy építhetők fel, hogy a bearer token nyers értéke nem kerül tartós tárolásba.
