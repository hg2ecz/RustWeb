# Domain értékek finomítása

Az RWLang nominális domain típusai nem castok. Egy primitív érték akkor sem válik automatikusan domain értékké, ha futásidőben ugyanaz a reprezentációjuk.

Az explicit, fail-closed forma:

```rw
let userId = validate UserId(raw)?;
```

A compiler megköveteli, hogy a `raw` érték biztonságos futásidejű reprezentációja egyezzen a domain alaptípusával. Futásidőben a `UserId` minden deklarált constraintje lefut, mielőtt az érték `userId` néven elérhetővé válik. Sikertelen validáció megszakítja a requestet Bad Request hibával.

Ez kizárólag validációs proofot hoz létre, authorization proofot nem. Egy validált `UserId` azt bizonyítja, hogy az érték megfelel a `UserId` kontraktusnak; azt nem, hogy az aktuális felhasználó jogosult a hozzá tartozó `User` objektumhoz.

A refinement megtartja az adat sensitivity/disclosure metaadatait, de szándékosan eldobja a mutation authorization evidence-et. Így egy másik nominális ID domainre történő újravalidálás nem örökölheti egy idegen objektum jogosultsági bizonyítékát.

Normál RWLang kódban nincs implicit `UserId(raw)` cast és nincs ellenőrizetlen domain konstruktor.
