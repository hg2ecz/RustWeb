# Típusos String listák

Az RWLang most már rendelkezik compute-local `List<String>` értékkel. Az első létrehozó művelet a `split(text, delimiter)`.

```rw
let parts = split("alpha,beta,gamma", ",");
let count = len(parts);
let first = parts[0];
```

A `split` típusa:

```text
split(String, String) -> List<String>
```

A szeparátor nem lehet üres. A `split` legfeljebb 4096 elemet hozhat létre, és az eredményül kapott stringek, valamint a kollekció overheadje beleszámít a runtime memóriafoglalási keretébe.

A `List<String>` egyelőre compute-local típus. Nem használható model mezőként, route/form skalárként, adatbázis skalárként vagy audit skalárként. Így a perzisztencia és a request schema explicit marad, amíg az általános collection réteg tovább épül.

Az indexelés futásidőben bounds-checkelt. Negatív vagy túl nagy index fail-closed hibát ad, nem implicit null értéket.

A collection expression réteg közös az `Array<F32>` típussal:

```rw
let samples = arrayF32(4096, 0.0f32);
let n = len(samples);
let x = samples[0];

let words = split("one two three", " ");
let m = len(words);
let word = words[0];
```

Ez a közös `len(collection)` és `collection[index]` reprezentáció lesz a későbbi típusos kollekciók és dictionary alapja.

Futtatható példa: `examples/string-list/main.rw`.
