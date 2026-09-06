# 4. Routing, input és validáció

## Path

```text
route product GET "/products/:id<Int>" => product;
```

A handlerben ugyanaz a név/típus kell:

```text
page fn product(ctx: PageContext, db: Db, id: Int) ...
```

## Query

```text
route products GET "/products"
    query page<Int> pageSize<Int>
    validate page range 1 100000 pageSize range 1 100
    => products;
```

A schema closed-world: hiányzó, duplikált vagy ismeretlen mező `400`.

## Form

```text
route create POST "/products"
    form name<String> price<Int>
    validate name length 1 100 price range 0 100000000
    => create;
```

Jelenlegi validation szabályok:

```text
length MIN MAX   // String
range MIN MAX    // Int
```

State-changing route-ot `action fn` kezeljen. A kliens által küldött `price`, `role`, `tenantId` stb. csak input; authority-t mindig a szerver/DB ad.

## Keresőbarát path: Slug

```rwlang
route articleShow GET "/cikk/:slug<Slug>" => articleShow;
```

A `Slug` canonical, max. 160 byte, és nem tetszőleges String. Így `../`, slash, szóköz és nem canonical forma nem jut át typed path paraméterként.

Új slug készítése:

```rwlang
let articleSlug = slug(title);
```

Részletesen: [Projektstruktúra, modulok és keresőbarát URL-ek](21-modules-slugs-project-layout.md).
