# Típusos credentialek és jelszó-primitívek

Az RWLang szándékosan magas szintű jelszóműveleteket ad raw kriptográfiai algoritmusok helyett.

```rw
let hash = passwordHash(password);
let valid = passwordVerify(account.passwordHash, password);
```

A `passwordHash(Password)` validált, purpose-típusos jelszót fogad, és statikusan `Secret<PasswordHash>` adatot eredményez. A runtime Argon2id algoritmust, friss véletlen saltot és runtime-policy által birtokolt paramétereket használ. Az alkalmazáskód normál esetben nem választ algoritmust, saltot vagy cost paramétert.

A `passwordVerify(Secret<PasswordHash>, Password)` kizárólag a pontos password-hash purpose-t és validált purpose-típusos jelszót fogadja. Az eredmény publikus `Bool`; a secret klasszifikáció nem szivárog át az összehasonlítás eredményére.

Ezek a műveletek nem hoznak létre authorization proofot. Egy account betöltése és jelszavának ellenőrzése/módosítása továbbra is a normál auth szerződéseket követi.
