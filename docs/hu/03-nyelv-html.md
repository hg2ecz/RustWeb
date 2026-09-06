# 3. Nyelvi és HTML alapok

## Statementhatárok

Az RWLang explicit pontosvesszőt használ. Az egyszerű statementek `;` jellel záródnak, a sortörés csak whitespace. A blokk nélküli top-level deklarációk (`mod ...;`, `route ... => handler;`) szintén pontosvesszősök; a `{ ... }` blokkal záródó deklarációk és control-flow blokkok után nincs `;`.

```rw
mod products;
route product GET "/products/:id<Int>" => products::show;

let gross = price * quantity;
return Ok(json(gross));
```

Részletesen: [Statement terminátorok](55-statement-terminatorok.md).

## Kifejezések, aritmetika és logika

A nyelv támogatja az ellenőrzött `+`, `-`, `*`, `/`, `%` aritmetikát; `Int` esetén `<<`, `>>`, `&`, `^`, `|` operátorokat; valamint `Bool` értékekre `!`, `&&`, `||` logikát. A `&&` és `||` short-circuit módon működik. Matematikai builtin többek között az `ln`, `log10`, `log`, `exp`, `pow`, `round`, `floor` és `ceil`.

```rw
let bucket = id % 16;
let flags = mask | 4;
let visible = published && !deleted;
let rounded = round(score);
```

Részletesen: [Numerikus operátorok, F32 matematika és monoton időmérés](43-matematika-es-idomeres.md).

## String műveletek

A Unicode-tudatos String API tartalmazza a trimminget és case conversiont, keresést, `replace`/`split` műveleteket, substring/index műveleteket, karakterelérést és ismétlést.

```rw
let cleaned = trim(title);
let slugText = replace(lower(cleaned), " " , "-");
let prefix = substring(slugText, 0, 8);
let found = indexOf(slugText, "rw");
```

Részletesen: [String műveletek](45-string-muveletek.md) és [Reguláris kifejezések](48-regexp.md).

## Model

```rw
model Product {
    id: Int
    name: String
    price: Int
}
```

A model mezőtípusok nem korlátozódnak a korai `String`/`Int`/`Bool` készletre; a nyelv támogatott üzleti és domain típusait a kapcsolódó típusfejezetek dokumentálják.

## Page és action

```text
page fn product(ctx: PageContext, db: Db, id: Int) -> Result<Html, PageError> {
    ...
}
```

```text
action fn create(ctx: ActionContext, db: Db, name: String, price: Int)
    -> Result<Redirect, PageError> {
    ...
}
```

## HTML

```text
return Ok(html {
    <h1>{{ product.name }}</h1>
})
```

A `{{ ... }}` HTML-escaped. DB-ből érkező stringre is ugyanaz a szabály.

Lista:

```text
@for product in products {
    <li>{{ product.name }}</li>
}
```

Optional model:

```text
@if product {
    <h1>{{ product.name }}</h1>
}
```

Typed URL:

```text
<a @href(product, product.id)>View</a>
<form method="post" @action(delete, product.id)>
```

Ne használj dinamikus `href="{{ value }}"` mintát; URL csak typed route helperen menjen.
