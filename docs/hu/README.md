# RWLang webalkalmazás-fejlesztői kézikönyv

**RWLang = Rust Web Language.** Ez a kézikönyv webfejlesztőnek szól: hogyan készíts, tesztelj és adj át productionre egy valódi RWLang alkalmazást. A fő útvonal egy ajánlott megoldást mutat; a compiler/runtime belső részletei csak ott jelennek meg, ahol fejlesztői vagy security döntéshez szükségesek.

## V1 ajánlott tanulási út

### 1. Első működő alkalmazás

1. [Gyors kezdés](01-gyors-kezdes.md)
2. [Projektstruktúra, modulok és Slug](21-modules-slugs-project-layout.md)
3. [Nyelvi és HTML alapok](03-nyelv-html.md)
   - [Numerikus operátorok, matematika és monoton időmérés](43-matematika-es-idomeres.md)
   - [String műveletek](45-string-muveletek.md)
   - [Statement terminátorok](55-statement-terminatorok.md)
4. [Routing, input és validáció](04-routing-input.md)
5. [Adatbázis és typed SQL](05-adatbazis.md)
6. [CRUD és tranzakciók](06-crud.md)
7. [Domain objectek](22-domain-objects.md)

### 2. Üzleti webalkalmazás

8. [Date, DateTime, Uuid és Decimal](11-business-types.md)
9. [Enumok](23-enums.md)
10. [Domain-validáció](26-domain-validation.md)
11. [Újrafelhasználható formok](12-forms.md)
12. [PRG, flash és conflict UX](28-prg-flash-conflict.md)
13. [Optimistic locking](../24-optimistic-locking.md)
14. [Üzleti audit trail](25-business-audit-trail.md)
15. [Canonical URL és régi slugok](27-canonical-url.md)

### 3. Auth, API és tartalom

16. [Auth és Redis](07-auth-redis.md)
17. [Objektumszintű authorization](20-object-authorization.md)
18. [JSON API és CORS](05-json-api-cors.md)
19. [Komponensek és layoutok](14-components-layouts.md)
20. [Biztonságos Markdown](16-markdown-rich-text.md)
21. [Képek és media library](17-media-library.md)
22. [Upload és AppFs](09-upload-appfs.md)
23. [Public cache](13-cache.md)
24. [Statikus assetek](06-static-assets.md)

### 4. Production

25. [Database migrations](10-database-migrations.md)
26. [Rate limiting](08-rate-limiting.md)
27. [Resource profile-ok](10-resource-profile.md)
28. [HTTPS és böngészőbiztonság](08-https-security.md)
29. [Server konfiguráció](15-server-config.md)
30. [Lifecycle és health](07-lifecycle-health.md)
31. [Observability](09-observability.md)
32. [Dependency security és reprodukálható build](../19-dependency-security.md)
33. [Tesztelés és hibakeresés](12-teszteles-hibak.md)
34. [Web security checklist](15-security-checklist.md)
35. [Production deployment és starter project](29-production-deployment.md)
36. [Backup, restore, upgrade és rollback](30-backup-restore-upgrade-rollback.md)
37. [V1 fejlesztői átadási ellenőrzőlista](31-v1-developer-guide.md)
38. [CLI és napi workflow](33-cli-workflow.md)
39. [`server.toml` konfigurációs referencia](34-server-toml-reference.md)
40. [Production checklist és minták](35-production-checklist.md)
41. [Debian csomag készítése és dpkg telepítés](36-debian-csomag.md)
42. [Több domain kiszolgálása](37-tobb-domain-kiszolgalas.md)
43. [Automatikus alkalmazás-forráskód reload](38-automatikus-forraskod-reload.md)

## V1 modulnévtér-szabály

Az R48 óta a `mod` source-graph deklaráció és namespace-határ, nem globális include. A `mod foo;` pontosan a `<app-root>/foo.rw` forrást tölti be `foo` névtérként; más modulból `foo::Name` alakú kvalifikált hivatkozás szükséges. A nested `foo::bar` modul kizárólag `foo/bar.rw`, nincs `mod.rw`, `../`, `self::`, `super::`, `crate::` vagy automatikus almappa-discovery. A részletes szerződés: [Projektstruktúra, modulok és Slug](21-modules-slugs-project-layout.md).

## V1 aktuális kifejezés- és string surface

Az aritmetikai mag ellenőrzött `+`, `-`, `*`, `/`, `%` műveleteket, `Int` shiftet (`<<`, `>>`), bitenkénti `&`, `^`, `|` operátorokat és `Bool` logikát (`!`, `&&`, `||`) ad; a `&&` és `||` short-circuit. Az F32 builtin készlet része többek között az `ln`, `log10`, `log`, `exp`, `pow`, `round`, `floor` és `ceil`. Részletesen: [Numerikus operátorok, F32 matematika és monoton időmérés](43-matematika-es-idomeres.md).

A Unicode-tudatos String API tartalmazza a `trim`, `trimStart`, `trimEnd`, `lower`, `upper`, `stringLen`, `contains`, `startsWith`, `endsWith`, `replace`, `split`, `substring`, `indexOf`, `lastIndexOf`, `charAt` és `repeat` műveleteket; regexhez külön typed API tartozik. Részletesen: [String műveletek](45-string-muveletek.md) és [Reguláris kifejezések](48-regexp.md).

Az egyszerű statementek és a blokk nélküli `mod`/`route` deklarációk explicit `;` terminátort használnak. A sortörés nem terminátor. Részletesen: [Statement terminátorok](55-statement-terminatorok.md).

## V1 nyelvi határ: `pub` és `mut`

RWLang jelenlegi V1 surface-e nem tartalmaz általános Rust-szerű `pub` vagy `mut` kulcsszót.

- `pub`: a jelenlegi `mod` namespace/source-graph mechanizmus, nem visibility/export rendszer. A V1-ben nincs külön public/private export réteg, ezért ne vezessünk be jelentés nélküli `pub`-ot.
- `mut`: továbbra sincs általános Rust-szerű `mut`. Compute blokkokban meglévő lokális érték explicit `set name = expr` formában módosítható, statikus típusmegőrzéssel és instruction budget alatt. Üzleti/tartós állapotmódosítás továbbra is action/query/transaction/capability útvonalon történik.

## Státuszjelölések

- **IMPLEMENTÁLT** — tényleges kódút van a workspace-ben.
- **TRUSTED RUST ADAPTER** — runtime/integrációs Rust felület, de még nem `.rw` standard API.
- **NEM TÁMOGATOTT** — tudatos non-goal vagy későbbi feature.

- [IPv6-ready outbound egress](../32-ipv6-egress.md)
- [CLI és napi workflow](33-cli-workflow.md)
- [`server.toml` konfigurációs referencia](34-server-toml-reference.md)
- [Production checklist és minták](35-production-checklist.md)
- [Debian csomag készítése és dpkg telepítés](36-debian-csomag.md)

## LaTeX könyv

A fejezetenként `\\input`-olt, webalkalmazás-fejlesztőknek szóló könyv forrása: [`book/`](../book/hu/README.md). Külön telepítési és konfigurációs fejezetet tartalmaz közvetlen egy-domaines HTTPS, Apache és Nginx reverse proxy mögötti üzemhez, majd form-, wiki- és CMS-fejlesztési útvonalat ad.

44. [Budgetelt `while` ciklus és lokális értékadás](42-budgetelt-while.md)

86. [F32 matematika és monoton időmérés](43-matematika-es-idomeres.md)

87. [Compute `if`, explicit F32 konverzió és FFT4096](44-if-konverzio-es-fft.md)


- [Típusos String listák](46-string-listak.md)

- [Típusos String dictionary](47-string-dictionary.md)

- [Reguláris kifejezések](48-regexp.md)

- [Karbantarthatóság és clean-code modulhatárok](49-karbantarthatosag-es-clean-code.md)

- [Típusos hibák és explicit modulfüggőségek](50-tipusos-hibak-es-explicit-fuggosegek.md)

- [51. Típusos szerverkonfigurációs és TLS hibák](51-tipusos-szerver-konfiguracios-es-tls-hibak.md)

- [52. Típusos auth-, policy-, resource-profile- és CLI-hibák](52-tipusos-auth-policy-resource-es-cli-hibak.md)

- [53. Típusos runtime és cache hibahatárok](53-tipusos-runtime-hatarok.md)

- [54. Típusos backend, reload és HTTP hibahatárok](54-tipusos-backend-reload-es-http-hatarok.md)
- [Statement terminátorok](55-statement-terminatorok.md) - explicit `;`, nincs automatikus semicolon insertion.

- [65. Explicit publikus projection](65-explicit-public-projection.md) - mező-allowlist alapú modell JSON-kimenet.
- [66. Secret-fogyasztói szerződések](66-secret-fogyasztoi-szerzodesek.md) - explicit secret sinkek és fail-closed audit boundaryk.

- [Típusos credentialek és jelszó-primitívek](67-tipusos-credential-es-jelszo.md)

- [Credential-purpose típusok](68-credential-purpose-tipusok.md) - nem felcserélhető jelszó-, token-, session-, CSRF- és kulcsidentitások.

- [Purpose-biztos token primitívek](69-purpose-safe-token-primitives.md) - purpose-típusos kiadás, validált bemutatás és constant-time token összehasonlítás.
- [Lejárat-tudatos tokenellenőrzés](71-lejarat-tudatos-token-ellenorzes.md) - session/reset ellenőrzés kötelező lejárati feltétellel.
- [Atomikus token-életciklus](72-atomikus-token-eletciklus.md) - egyszer használható reset grant és aktuális session visszavonás compiler proofokkal.
- [Atomikus session rotáció](73-atomikus-session-rotacio.md) - őrzött session-hash csere presented-token és friss issued-token proofokkal.

- [Platform által kezelt böngészős session](74-platform-altal-kezelt-browser-session.md)

- [Függvényszintű jogosultságok](75-fuggvenyszintu-jogosultsagok.md) - névvel ellátott üzleti permissionök compiler által kikényszerített handler-szerződéssel.

- [MFA-elevációs proofok](76-mfa-elevacios-proofok.md) - handler-szintű MFA-szerződés névvel ellátott permissionnel kombinálva.

- [Route budget profilok](78-route-budget-profilok.md) - secure-by-default route eroforrasprofilok operátori limitekkel.

- [Korlátos request-kollekciók](79-korlatos-request-kollekciok.md) - korlátos `List<String>` query/form/JSON inputok biztonságos alapértelmezéssel.

- [Tenant membership authority](81-tenant-membership-authority.md)
- [Compiler altal kikenyszeritett tenant izolacio](82-compiler-altal-kikenyszeritett-tenant-izolacio.md) - aktiv tenant proof, scoped modellek es SQL tenant guardok.
