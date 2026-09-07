# Korlátos request-kollekciók

Az RWLang a külső kollekció elemszámát biztonsági határként kezeli. A `List<String>` request mezők alapból korlátosak, és a korlát explicit szűkíthető.

```rw
page fn search(ctx: PageContext, tags: List<String>) -> Result<Json, PageError> {
    return Ok(json(len(tags)));
}

route search GET "/search"
    query tags<List<String>>
    validate tags items 1 16
    public => search;
```

Ha nincs explicit `items` szabály, a biztonságos alapértelmezés `items 1 64`. Az explicit szabály ezt helyettesíti, nem egy második limitként rakódik rá.

## Bemeneti forma

- Query és form esetén ismételt mezők: `?tags=rust&tags=web`.
- JSON esetén valódi string tömb: `{ "tags": ["rust", "web"] }`.
- A normál request-kollekció nem CSV-string, ezért nem kell kézi `split()` boilerplate.

A JSON tömb már a deszerializáláskor hard cap-et kap, mielőtt a handler elindulna. A route ezután ennél szűkebb üzleti limitet írhat elő.

## Biztonsági tulajdonságok

- Duplikált skalár mező továbbra is fail-closed hiba.
- Ismételt mező csak `List<String>` esetén fogadható el.
- A request-lista csak bizonyított cardinality bound mellett válik `Validated<List<String>>` értékké.
- A platform abszolút plafonja 256 elem, az alap route-limit 64.
- Az üres JSON string tömb a jelenlegi wire formatban tiltott; ha a mező jelen van, legalább egy elemet kell tartalmaznia.

Az RWLangban jelenleg nincs általános, korlátlan user-data loop konstrukció. Ez az iteráció először a request boundaryt teszi bizonyíthatóan korlátossá; a későbbi collection transform/iteration funkcióknak a boundot meg kell őrizniük vagy szűkíteniük.
