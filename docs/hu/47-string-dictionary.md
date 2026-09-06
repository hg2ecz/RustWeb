# Típusos string dictionary

Az RWLang `Dict<String,String>` compute-local kollekciót ad kis, determinisztikus kulcs-érték táblákhoz, például metaadatokhoz, feldolgozott inputhoz és lookup táblákhoz.

```rw
let values = dict();
set values["name"] = "RWLang";
set values["mode"] = "production";

let count = len(values);
let exists = containsKey(values, "name");
let name = values["name"];

set values = removeKey(values, "mode");
```

A kulcs és az érték is `String`; nincs implicit konverzió más skalár típusból.

## Hiányzó kulcs

A `values["missing"]` szigorú: hibát ad, nem üres stringet. Ha a kulcs hiánya normális eset, előbb `containsKey(values, key)` használható.

## Módosítás

Beszúrás és felülírás:

```rw
set values[key] = value;
```

A `removeKey(dict, key)` új dictionary értéket ad vissza, ezért törléskor:

```rw
set values = removeKey(values, key);
```

## Limitek

Legfeljebb 4096 bejegyzés lehet. A kulcs nem lehet üres, és legfeljebb 1024 UTF-8 bájt lehet. A kulcsok, értékek és a kollekció overheadje beleszámít a runtime allocation budgetbe.

A `Dict<String,String>` jelenleg compute-local típus: nem model mező, DB skalár, route/form inputtípus és nem business-audit skalár.
