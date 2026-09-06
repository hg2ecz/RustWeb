# Bytecode kifejezés-végrehajtás

Az RWLang futás közben már nem rekurzív AST-bejárással értékeli ki minden alkalommal az alkalmazás kifejezéseit. A compiler továbbra is típusos `Program`/AST reprezentációt készít, de a runtime kifejezés-végrehajtása egy kis stack-alapú bytecode VM-en történik.

A jelenlegi folyamat:

```text
.rw forrás
  -> lexer/parser/típusellenőrzés
  -> Program / AST
  -> kifejezés-bytecode cache
  -> stack VM
  -> runtime értékek
```

A VM jelenleg a meglévő kifejezésnyelvet fedi le: string, egész szám, bool, enum literal, változó, modellmező, `slug(...)`, valamint az ellenőrzött `+`, `-`, `*`, `/` műveletek. A bytecode ugyanazt az instruction-budget szemantikát tartja meg, mint a korábbi rekurzív kiértékelő: minden kibocsátott kifejezésművelet egy instruction egységet fogyaszt. Túlcsordulás, nullával osztás, hibás mezőelérés és érvénytelen slug továbbra is fail-closed hibát ad.

A lefordított kifejezés-bytecode strukturális kifejezésazonosság alapján cache-elődik. A cache korlátos; a plafon elérésekor ürül, tehát nem nőhet korlátlanul. Ez még nem JIT: natív gépi kód jelenleg nem készül.

Az AST megmarad a compiler szemantikai reprezentációjának. Erre építhető a következő numerikus lépcső anélkül, hogy a nyelvi szintaxist egy konkrét JIT backendhez kötnénk: `F32`/`F64`, numerikus tömbök, ciklusok, matematikai builtinok, monotonic időmérés, majd opcionális natív kódgenerálás.
