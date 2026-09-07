# Credential-purpose típusok

Az RWLang credential értékei első osztályú cél-típusidentitást kapnak. A beépített típusok:

- `Password`
- `PasswordHash`
- `ApiToken`
- `SessionToken`
- `PasswordResetToken`
- `CsrfToken`
- `CryptoKey`

Runtime oldalon String reprezentációt használnak, de fordításkor nem felcserélhetők. Egy `Secret<ApiToken>` nem adható át `Secret<PasswordHash>` helyére akkor sem, ha mindkettő stringként tárolódik.

A jelszó közvetlenül a web boundaryn kap purpose-típust:

```rw
route change POST "/account/password"
    form password<Password>
    auth user => change;
```

A `Password` dekódolása a handler indulása előtt kikényszeríti a platform jelszó-hossz policy-jét. Tetszőleges validált `String` szándékosan nem használható jelszó-kriptográfiához.

```rw
let hash = passwordHash(password); // password: Password
```

A `passwordHash(Password)` eredménye `Secret<PasswordHash>`. A `passwordVerify` pontosan `Secret<PasswordHash>` és `Password` párost vár:

```rw
let valid = passwordVerify(account.passwordHash, password);
```

Credential-purpose érték nem renderelhető, nem serializálható, nem tehető általános `expose(...)` projectionbe és nem írható business audit mezőbe. A credentialek kiengedéséhez vagy felhasználásához dedikált, szűk protokoll-primitív szükséges.

A purpose wrapper runtimeban erased, tehát nincs plusz allokáció. A token-generálás, lejárat, single-use reset, session kiadás és CSRF-specifikus disclosure külön lifecycle-réteg; ezeket dedikált primitíveknek kell kezelniük a generic credential boundary fellazítása helyett.
