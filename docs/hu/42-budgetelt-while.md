# Budgetelt `while` ciklus és explicit lokális értékadás

Az RWLang támogat erőforrás-számlált compute ciklust numerikus feladatokhoz. A `while` feltételének `Bool` típusúnak kell lennie; minden feltételvizsgálat és törzsutasítás fogyasztja a request instruction budgetet. A budget elfogyása a szokásos instruction-limit hibával állítja meg a futást, tehát nem maradhat korlátlanul futó worker ciklus.

```rw
let i = 0;
let samples = arrayF32(4096, 0.0f32);

while i < len(samples) {
    set samples[i] = 1.0f32;
    set i = i + 1;
}
```

A `set` explicit lokális újraértékadás. A változó statikus típusa nem változhat: `Int` lokálisba `F32` kifejezés nem írható. A tömbelem módosítása továbbra is `set array[index] = value`, futásidejű bounds checkkel.

Az összehasonlító operátorok: `==`, `!=`, `<`, `<=`, `>`, `>=`. Rendezési összehasonlítás jelenleg `Int`, `F32` és `Decimal` típusokra használható; egyenlőség `String` és `Bool` esetén is. Vegyes numerikus típusok implicit összehasonlítása szándékosan tiltott, például `1 < 2.0f32` fordítási hiba.

A `while` törzsében létrehozott `let` lokálisok compiler szempontból a compute blokkhoz tartoznak. A külső lokálisok `set` segítségével módosíthatók. Egymásba ágyazott `while` ciklusok is használhatók, ugyanazt az instruction budgetet fogyasztva.

A funkció determinisztikus, korlátozott alkalmazásoldali számításra készült, például numerikus transzformációkhoz. Nem kerüli meg a request timeoutot, az allocation limiteket vagy a named resource profile-okat.
