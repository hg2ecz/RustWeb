# Validált input adatfolyam

Az RWLang külön kezeli a belső/megbízható, a külső, illetve a route-szerződésen már ellenőrzött skalár értékeket.

```text
Trusted      belső/compiler/runtime tulajdonú adat
Validated    ellenőrzött route-határon átjutott külső adat
Untrusted    még nem validált külső adat
```

A fejlesztőnek normál handlerben nem kell `Untrusted<T>` vagy `Validated<T>` wrapper-eket kiírnia. A route maga a trust boundary.

A `String` query/form/json és most már path inputok is biztonságos alapértelmezett felső hosszkorlátot kapnak. Szűkebb domain-szabály egyszerűen megadható:

```rwlang
route create POST "/notes"
    form title<String>
    validate title length 1 160
    public => create;
```

Az olyan típusok, mint `Email`, `Url`, `Slug`, `Int`, `Bool`, `Uuid`, `Date`, `DateTime` és `Decimal` a saját dekóderük/normalizálójuk sikeres lefutása után kapnak validációs proofot. Hibás érték nem jut el a handlerig.

A tranzakciós query-k `SEC-DATA-003` hibával elutasítják a még `Untrusted` skalár argumentumokat. Ez különösen fontos például upload metaadatnál: a kliens által megadott fájlnév nem írható közvetlenül adatbázisba.

Az authorization proof ettől független: a validáció azt bizonyítja, hogy az adat megfelel a szerződésnek; az authorization azt, hogy az adott principal a konkrét objektumot módosíthatja.

Alapelv:

> A validáció proof, amelyet a boundary állít elő, nem egy application-code konvenció, amelyre emlékezni kell.
