# Publikus hibahatárok

Az RWLang különválasztja az alkalmazás által biztonságosan publikálható hibákat a belső runtime hibáktól.

Egy page vagy action az alábbi zárt hibakészlettel állhat le:

```rw
fail badRequest;
fail notFound;
fail forbidden;
fail conflict;
```

A szerver ezeket a meglévő biztonságos HTTP hibaválaszokra képezi. Alkalmazáskódból nincs `fail internal`, `fail database`, tetszőleges HTTP státuszkód vagy tetszőleges hibaszöveg. A belső, adatbázis-, erőforrás-limit- és infrastruktúrahibák platform-owned hibahatáron maradnak.

Így a normál fejlesztői út rövid, miközben stack trace, SQL részlet, credential vagy infrastruktúraüzenet nem tud véletlenül HTTP válaszba kerülni.

A `fail` terminális statement, ugyanúgy lezárja a page/action bodyt, mint egy sikeres return.

A `SEC-A10-003` diagnosztika tiltja az ismeretlen vagy belső hibatípus publikálását.
