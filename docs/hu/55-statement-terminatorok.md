# Statement terminátorok

Az RWLang minden egyszerű statementet explicit pontosvesszővel (`;`) zár. A sortörés csak whitespace; nincs automatikus semicolon insertion.

```rw
let total = price
    * quantity
    + shipping;
set retries = retries + 1;
authorize article owner authorUsername or role Publisher;
flash success "Saved";
return Ok(json(total));
```

A `transaction db { ... }` blokkon belüli önálló mutating query-hívások és az `audit ...` statementek szintén `;` jelet igényelnek.

```rw
transaction db {
    updateArticle(tx, id, title)?;
    audit Article id action update from oldTitle to title;
}
```

A blokkokat a `}` zárja, utánuk nincs pontosvessző:

```rw
if ready {
    set state = 1;
}
while state < 3 {
    set state = state + 1;
}
```

Ugyanez érvényes a blokkos deklarációkra (`model`, `page fn`, `action fn`). A `mod path;` és a `route ... => handler;` pontosvesszős, mert blokk nélküli deklaráció. A route több sorba tördelhető, de csak a záró `;` terminálja.

## Top-level, blokk nélküli deklarációk

A `mod` és `route` deklaráció is explicit `;` jellel záródik:

```rw
mod catalog::pages;

route catalogIndex GET "/catalog"
    query page<Int>
    validate page range 1 1000
    => catalog::pages::index;
```

A route scanner csak a záró `;` után tekinti teljesnek a deklarációt; a sortörés és az `=> handler` önmagában nem terminátor.
