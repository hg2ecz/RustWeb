# Numerikus operátorok, F32 matematika és monoton időmérés

Az RWLang ellenőrzött numerikus operátorokat és budgetelt matematikai builtin készletet ad az alkalmazás- és compute kódhoz.

## Operátorok

```rw
let osszeg = 10 + 3;
let kulonbseg = 10 - 3;
let szorzat = 10 * 3;
let hanyados = 10 / 3;
let maradek = 10 % 3;

let balra = 3 << 2;
let jobbra = 24 >> 2;
let maszkolt = 12 & 10;
let valtott = 12 ^ 10;
let egyesitett = 12 | 3;

let engedelyezett = kesz && jogosult;
let hasznalhato = cachelt || friss;
let tiltott = !engedelyezett;
```

A `+`, `-`, `*`, `/` és `%` azonos típusú `Int`, `F32` és `Decimal` operandusokra használható. A `String + String` továbbra is konkatenáció. A shift és bitenkénti operátorok (`<<`, `>>`, `&`, `^`, `|`) csak `Int` értékekre érvényesek. A `&&`, `||` és `!` `Bool` típust vár; a `&&` és `||` short-circuit módon működik, tehát szükség esetén a jobb oldal ki sem értékelődik.

A precedencia szorosabbtól lazább felé: unáris `!`; `* / %`; `+ -`; shiftek; összehasonlítások; bitenkénti `&`, `^`, `|`; logikai `&&`; logikai `||`. Ha az olvashatóság javul, használj zárójelet.

Az aritmetika ellenőrzött marad. Integer overflow, nullával osztás/maradékképzés, nem véges F32 eredmény és hibás shift-szám fail-closed hibát ad, nem csendes wraparoundot.

## F32 matematikai builtinok

```rw
let x = 0.5f32;
let s = sin(x);
let c = cos(x);
let gyok = sqrt(4.0f32);
let abszolut = abs(-3.5f32);
let termeszetes = ln(2.0f32);
let tiz_alapu = log10(1000.0f32);
let kettes_alapu = log(8.0f32, 2.0f32);
let exponencialis = exp(1.0f32);
let hatvany = pow(2.0f32, 8.0f32);
let kerek = round(2.6f32);
let le = floor(2.6f32);
let fel = ceil(2.1f32);
```

Szignatúrák:

```text
sin(F32) -> F32
cos(F32) -> F32
sqrt(F32) -> F32
abs(Int) -> Int
abs(F32) -> F32
ln(F32) -> F32
log10(F32) -> F32
log(F32, F32) -> F32
exp(F32) -> F32
pow(F32, F32) -> F32
round(F32) -> F32
floor(F32) -> F32
ceil(F32) -> F32
toF32(Int) -> F32
```

Minden F32 eredménynek végesnek kell maradnia. A hibás tartományú műveletek, például `sqrt(-1.0f32)` vagy `ln(-1.0f32)`, hibával leállnak NaN vagy végtelen érték létrehozása helyett.

## Monoton időmérés

```rw
let started = monotonicNanos();
// mért munka
let elapsed = monotonicNanos() - started;
```

A `monotonicNanos()` processzen belüli monoton eredethez képest ad vissza nanoszekundumot `Int` típusként. Eltelt idő mérésére való, nem dátum/időbélyeg előállítására. A compiler nem enged public cache-t olyan oldalnál, amelynek kimenete ettől az órától függ.

A matematikai builtinok súlyozott instruction-budget költséget kapnak. A transzcendens műveletek drágábbak az egyszerű aritmetikánál, így a számításigényes kód is a konfigurált erőforráskorlátok alatt marad.

Futtatható példa: `examples/numeric-operators/main.rw`.

## Kapcsolódó nyelvi szabályok

A fenti `let`, `set` és `return` formák egyszerű statementek, ezért `;` zárja őket; egy kifejezésen belüli sortörés nem terminátor. Lásd: [Statement terminátorok](55-statement-terminatorok.md). A `String + String` konkatenáció mellett a teljes Unicode-tudatos String API: [String műveletek](45-string-muveletek.md).
