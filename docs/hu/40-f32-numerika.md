# F32 numerikus értékek

Az RWLang `F32` típusa IEEE-754 egyszeres pontosságú lebegőpontos számításokra szolgál. Nem azonos a `Decimal` típussal: a `Decimal` pontos decimális/üzleti értékekhez való, az `F32` pedig numerikus algoritmusokhoz.

Az F32 literál explicit `f32` suffixet igényel:

```rw
let x = 1.25f32;
let y = -0.5f32;
let z = x * y + 2.0f32;
```

A suffix nélküli törtszám fordítási hiba. Implicit kevert aritmetika sincs: az `1 + 0.5f32` hibás, így a pontosságváltás nem történhet észrevétlenül.

Az RWLang csak véges `F32` értéket enged. `NaN` és pozitív/negatív végtelen inputként elutasításra kerül, a nem véges eredményt előállító aritmetika pedig fail-closed runtime hibát ad. A nullával osztás szintén runtime hiba.

Az `F32` önállóan jelenik meg az AST-ban és a bytecode VM-ben; nem `Decimal` konverzión keresztül fut. A VM külön `PushF32` opkódot használ, és a `+`, `-`, `*`, `/`, `%` műveleteket egyszeres pontossággal végzi.

Typed HTTP/form/query inputban is használható `F32`. A DB kompatibilitási réteg jelenleg kanonikus szövegként tárolja, hogy a támogatott backendek között azonos parse-szemantika maradjon.

Ez a lépés még nem tartalmaz tömböt, indexelést, ciklust, trigonometrikus builtinokat vagy időmérést. Ezek következnek az FFT4096 benchmark előtt.
