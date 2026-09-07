# Platform által kezelt böngészős session

Az RWLang böngészős autentikációja egyetlen, platform által kezelt session authorityt használ. Az alkalmazáskódnak nem kell böngészős autentikációs cookie-t generálnia, rotálnia, serializálnia vagy konfigurálnia.

## Biztonságos happy path

A szerver garantálja:

- sikeres autentikáció után új session azonosító és új CSRF token keletkezik;
- logoutkor az autentikált session érvénytelen lesz, majd friss anonymous session jön létre;
- lokális jelszó-, MFA-, disabled-state- és más auth-generation változás után a régi autentikált session a következő kérésnél érvénytelen lesz;
- production cookie neve `__Host-rw_session`, továbbá `Path=/`, `HttpOnly` és `Secure` attribútumot kap;
- alapértelmezésben `SameSite=Lax`, credentialed cross-origin telepítésnél `SameSite=None`, de továbbra is `Secure`;
- nincs `Domain` attribútum, ezért a cookie host-only marad.

A normál RWLang alkalmazáskód ezért autentikációs szándékot fejez ki (`auth user`, `auth mfa`, `auth role ...`), nem session transport mechanikát.

## Fejlesztői ergonomia

A security-critical cookie policy egyetlen szervermodulban él és regressziós tesztek védik. Nem kell route-onként cookie flag-eket vagy login utáni kézi rotation hívásokat írni.

Alkalmazásszintű bearer token továbbra is indokolt explicit domain workflow-khoz, például password resethez vagy API credentialhöz. Ezeket nem szabad második böngészős login-session rendszerként használni a platform session mellett.
