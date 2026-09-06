# Compute `if`, explicit F32 konverzió és FFT4096

Az RWLang compute kód budgetelt feltételes blokkot támogat, implicit `else` ág nélkül:

```rw
let x = 3;
if x < 10 {
    set x = x + 1;
}
```

A feltétel típusa kötelezően `Bool`. A blokk utasításai ugyanazt az instruction- és allocation-budgetet fogyasztják, mint a `while` compute kód. A blokkon belül deklarált lokális változók compile-time szinten blokklokálisak; meglévő változó explicit `set` utasítással módosítható.

## Explicit Int -> F32 konverzió

A `toF32(Int)` az egész index/control értékek és a bináris lebegőpontos számítás közötti explicit átjáró:

```rw
let i = 64;
let phase = toF32(i) * 0.125f32;
```

Nincs implicit `Int`/`F32` konverzió. A `toF32` nagy egész számoknál szándékosan explicit, potenciálisan pontosságvesztő IEEE-754 konverzió; ahol pontos egész szemantika kell, ott `Int` maradjon.

## FFT4096 benchmark

Az `examples/fft4096/main.rw` teljes iteratív radix-2 Cooley-Tukey FFT-t valósít meg RWLangban, két `Array<F32>` tömbbel a valós és képzetes komponensekhez. Nem hív natív FFT könyvtárat.

A determinisztikus bemeneti jel egy egységnyi amplitúdójú 64-es és egy fél amplitúdójú 256-os binű szinuszt tartalmaz. Normalizálatlan transzformációnál a várt magnitúdók megközelítőleg 2048 és 1024. A példa kiírja:

- az FFT `monotonicNanos()` segítségével mért idejét;
- a 64-es bin magnitúdóját;
- a 256-os bin magnitúdóját;
- egy széles correctness tartomány eredményét.

Az időmérés a jelgenerálás után indul, és a bit-reversal valamint az összes FFT stage idejét tartalmazza. Compute-heavy példához megfelelően magas instruction budget szükséges. Teljesítménymérésnél ismételt kérést érdemes használni, mert az első kérés az expression-bytecode cache-t is feltöltheti.
