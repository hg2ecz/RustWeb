# 12. Újrafelhasználható formok

Az M27 célja, hogy ugyanazt a field- és validation-sémát ne kelljen minden POST route-on újra leírni.

## Form schema

```text
form ProductForm {
    name<String>
    price<Int>
    published<Bool>
    validate name length 2 100 price range 0 1000000
}
```

A schema compiler-owned contract. A route csak a nevét használja:

```text
route create POST "/products" form ProductForm => create;
```

A handler paraméterei ugyanabban a sorrendben és ugyanazzal a típussal követik a form mezőit:

```text
action fn create(
    ctx: ActionContext,
    name: String,
    price: Int,
    published: Bool
) -> Result<Redirect, PageError> {
    ...
}
```

## Hibás beküldés

Named form esetén a mezőszintű típus- vagy validation-hiba nem általános `400`, hanem:

```http
422 Unprocessable Content
Cache-Control: no-store
```

A szerver egy biztonságosan generált form-hibalapot ad vissza:

- a korábban beküldött értékek megmaradnak;
- a mezőhöz tartozó hiba megjelenik;
- a CSRF token automatikusan újra bekerül;
- minden visszatöltött érték HTML-escape-elt;
- a form action ugyanaz a request path.

A jelenlegi hibakódok:

```text
required
invalid_type
length
range
```

## Bool mező

Named formnál egy hiányzó `Bool` mező `false` értékként értelmeződik. Ez kényelmesebb HTML formokhoz, ahol a checkbox/select gyakran így viselkedik.

## Biztonsági különbség

Az ismeretlen vagy duplikált mező továbbra is `400 Bad Request`, nem „felhasználói validation hiba”. Ez megőrzi a closed-world request sémát és megakadályozza a parameter-smuggling jellegű fellazítást.

## Inline form továbbra is működik

A régi forma változatlan:

```text
route create POST "/products"
    form name<String> price<Int>
    validate name length 1 100 price range 0 100000
    => create;
```

Named formot akkor használj, ha ugyanaz a contract több helyen vagy hosszabb távon karbantartandó.

## v0.1 korlát

A 422 hibalap jelenleg framework-generated. Egyedi layout/field component/error partial a következő component/layout réteg feladata lesz. Raw HTML error value nincs.
