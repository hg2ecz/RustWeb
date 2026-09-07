# Típusos biztonsági események

Az RWLang biztonsági eseményt egyszer kell deklarálni és egy modellhez kötni:

```rw
security event RoleGranted for User;
```

Tranzakción belül az esemény röviden kibocsátható:

```rw
security RoleGranted user.id;
```

Opcionálisan publikus, nem érzékeny állapotváltozás is rögzíthető:

```rw
security RoleChanged user.id from oldRole to newRole;
```

`Secret<T>` és `Sensitive<T>` érték továbbra sem kerülhet a security event mezőibe. A critical operation konkrét eseményt is megkövetelhet:

```rw
critical RoleChange {
    permission UserAdmin
    mfa
    transaction
    audit RoleGranted
}
```

Más audit vagy más security event nem teljesíti ezt a szerződést. Így a fejlesztői kód rövid marad, a kritikus állapotváltozás pedig nyelvi szinten megfigyelhető.
