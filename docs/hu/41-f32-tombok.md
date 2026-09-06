# F32 tömbök

Az RWLang numerikus számításokhoz korlátozott `Array<F32>` tömböt biztosít:

```rw
let samples = arrayF32(4096, 0.0f32);
set samples[0] = 1.0f32;
let first = samples[0];
let n = len(samples);
```

Az `Array<F32>` futásidejű numerikus konténer, adatbázis- vagy modellmezőként nem használható. A foglalás beleszámít a request memória-budgetbe. Egy tömb jelenlegi biztonsági maximuma 1 048 576 elem. Az indexelés bounds-checkelt; negatív vagy túl nagy index fail-closed hibát ad.

Az explicit `set` utasítás láthatóvá teszi a módosítást a compiler és a resource-budget számára. Ez lesz a ciklusok, matematikai builtinok, monotonic timer és az FFT4096 alapja.
