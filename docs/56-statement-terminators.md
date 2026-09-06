# Statement terminators

RWLang uses an explicit semicolon (`;`) to terminate every simple statement. A newline is whitespace only; there is no automatic semicolon insertion.

```rw
let total = price
    * quantity
    + shipping;
set retries = retries + 1;
authorize article owner authorUsername or role Publisher;
flash success "Saved";
return Ok(json(total));
```

Inside `transaction db { ... }`, standalone mutating query calls and `audit ...` statements also require `;`.

```rw
transaction db {
    updateArticle(tx, id, title)?;
    audit Article id action update from oldTitle to title;
}
```

Blocks are closed by `}` and do not take a trailing semicolon:

```rw
if ready {
    set state = 1;
}
while state < 3 {
    set state = state + 1;
}
```

The same principle applies to block declarations such as `model`, `page fn`, and `action fn`: no `;` follows the closing brace. `mod path;` and `route ... => handler;` are semicolon-terminated because they are non-block declarations. A route may span multiple lines, but only the final `;` terminates it.

## Non-block top-level declarations

`mod` and `route` declarations also use an explicit `;`:

```rw
mod catalog::pages;

route catalogIndex GET "/catalog"
    query page<Int>
    validate page range 1 1000
    => catalog::pages::index;
```

The route scanner considers the declaration complete only after the final `;`; neither a newline nor `=> handler` alone terminates a route.
