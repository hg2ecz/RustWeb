# String műveletek

Az RWLang típusos, Unicode-tudatos string-compute magot ad az alkalmazáslogikához. Ezek a műveletek a budgetelt bytecode runtime-ban futnak, tehát az instruction- és memóriaelszámolást nem kerülik meg.

## API

```rw
let cleaned = trim(text);
let left = trimStart(text);
let right = trimEnd(text);
let low = lower(cleaned);
let high = upper(cleaned);
let n = stringLen(cleaned);
let has = contains(low, "rust");
let prefix = startsWith(low, "rw");
let suffix = endsWith(low, "lang");
let rewritten = replace(low, " ", "-");
let part = substring(cleaned, 1, 3);
let first = indexOf(cleaned, "rw");
let last = lastIndexOf(cleaned, "a");
let ch = charAt("RWLang", 1);
let repeated = repeat("rw", 3);
let pieces = split(cleaned, ",");
```

Szignatúrák:

```text
stringLen(String) -> Int
trim(String) -> String
trimStart(String) -> String
trimEnd(String) -> String
lower(String) -> String
upper(String) -> String
contains(String, String) -> Bool
startsWith(String, String) -> Bool
endsWith(String, String) -> Bool
replace(String, String, String) -> String
split(String, String) -> List<String>
substring(String, Int) -> String
substring(String, Int, Int) -> String
indexOf(String, String) -> Int
lastIndexOf(String, String) -> Int
charAt(String, Int) -> String
repeat(String, Int) -> String
```

A `stringLen`, `substring`, `indexOf`, `lastIndexOf` és `charAt` Unicode skalárérték-pozíciókkal dolgozik, nem UTF-8 byte indexekkel. Az `indexOf` és `lastIndexOf` `-1` értéket ad, ha nincs találat. A `substring(text, start)` a `start` pozíciótól adja vissza a string végét; a háromparaméteres forma karakterhosszt kap, és a string végénél levágja a tartományt. Negatív vagy érvénytelen index fail-closed hibát ad.

A kis- és nagybetűsítés Unicode-tudatos. A `replace` nem enged üres keresőszöveget, a `split` pedig üres delimitert; egy split legfeljebb 4096 elemet eredményezhet. A `repeat` nem enged negatív ismétlésszámot, és az allokáció előtt bekerül a request memória-budget elszámolásába.

## Erőforrás-elszámolás

A String eredményt készítő builtinok az eredmény előállítása előtt lefoglalják a becsült/kiszámított kimeneti méretet a request és resource-scope allocation budgetből. Unicode case conversionnél konzervatív felső becslést használunk; `replace`, `split` és `repeat` esetén a kimeneti méretet előre számoljuk vagy konzervatívan becsüljük.

A műveletek külön instruction költséget is kapnak, tehát nem jelentenek budget nélküli kerülőutat.

Futtatható példák: `examples/string-operations/main.rw` és `examples/string-list/main.rw`.

## Statement szintaxis

A példák `let` sorai egyszerű statementek, ezért explicit `;` zárja őket; a sortörés önmagában nem terminátor. Lásd: [Statement terminátorok](55-statement-terminatorok.md). A `String + String` konkatenáció és az operátorprecedencia részletesen: [Numerikus operátorok, F32 matematika és monoton időmérés](43-matematika-es-idomeres.md).
