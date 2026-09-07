# Domain típusok és korlátos kollekciók

Az RWLang domain input típusai lehetővé teszik, hogy egy webes bemeneti invariánst egyszer nevezzünk el, majd minden trust boundaryn újra felhasználjunk.

```rw
type Username = String {
    length 3 32;
    pattern "^[A-Za-z0-9_]+$";
}

type PageSize = Int {
    range 1 100;
}
```

A route ezután közvetlenül használhatja a domain nevet:

```rw
route profile GET "/profiles/:username<Username>"
    query size<PageSize>
    public => profile;
```

A compiler a domain constraintjeit beemeli a route contractba. A runtime a handler meghívása előtt ellenőrzi őket, ezért a handler paramétere validation proof birtokában érkezik. A sima `String` továbbra is automatikus `0..4096` korlátot kap; a domain `length` szabály ezt egy szűkebb, alkalmazásspecifikus kontraktra cseréli.

Jelenleg domain típus alapja `String` vagy `Int` lehet. `String` esetén `length` és `pattern`, `Int` esetén `range` használható. String domain maximuma legfeljebb 4096 karakter. Azonos constraint fajta nem deklarálható kétszer. A constraint bodyban a pontosvessző és a vessző opcionális elválasztó.

A domain nevek most már megőrzik nominális típusazonosságukat a compiler által követett teljes adatfolyamban. A `UserId` és az `ArticleId` külön típus akkor is, ha mindkettő runtime reprezentációja `Int`. A runtime csak fordítás után erodálja a nominális wrappert, ezért nincs extra wrapper-allokáció vagy szerializációs költség.

## Korlátos String-lista

A `splitBounded(text, delimiter, maxItems)` explicit, fail-closed elemszám-korláttal készít `List<String>` értéket:

```rw
let tags = splitBounded(raw, ",", 32);
```

A `maxItems` kötelezően `1..4096` közötti egész literál. Ha a bemenet ennél több elemet eredményezne, a végrehajtás hibával leáll, nem csonkol csendben. A literál megkövetelése miatt az erőforráskorlát a compiler, a reviewer és az olvasó számára is látható.

A régi `split` kompatibilitásból továbbra is globálisan 4096 elemre korlátozott, de security-sensitive kódban a szűkebb `splitBounded` forma ajánlott, ha a domain ismert cardinality limittel rendelkezik.
