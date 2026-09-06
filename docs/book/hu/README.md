# RWLang webalkalmazás-fejlesztőknek -- LaTeX könyv

A magyar könyv fejezetenként `\input`-olt LaTeX forrásból épül, és ugyanazt a közös tipográfiai stílust használja, mint az angol kiadás.

A repository gyökeréből:

```bash
make book-hu
```

Vagy közvetlenül:

```bash
make -C docs/book/hu pdf
```

Kimenet:

```text
docs/book/hu/rwlang-webfejlesztoknek-hu.pdf
```

Tisztítás:

```bash
make -C docs/book/hu clean
make -C docs/book/hu distclean
```

A `distclean` a generált PDF-et is eltávolítja, így a forrás-release csomag tiszta marad.

## Szerkezet

A könyv első olvasásra, kezdő RWLang-felhasználónak van rendezve:

1. modell-szemlélet, telepítés, A-Z minimális alkalmazás;
2. alap webalkalmazás-fejlesztés;
3. wiki/CMS gyakorlati alkalmazás;
4. haladó integráció és tesztelés;
5. production és üzemeltetés;
6. referencia és operátori gyorssegédlet.

A teljes `server.toml` referencia szándékosan a könyv végén van: az első alkalmazáshoz csak a szükséges minimális konfigurációt vezetjük be.
