# Automatikus alkalmazás-forráskód reload

Az RWLang a frissen feltöltött `.rw` alkalmazáskódot process restart nélkül is érvényre tudja juttatni. A source reload tranzakciós: a megváltozott alkalmazásból először külön candidate runtime készül, és az élő domain csak sikeres fordítás és validáció után vált át rá. Hibás új kód esetén az előző működő generáció szolgál tovább.

## Konfiguráció

Globális alapértékek:

```toml
[reload]
enabled = true
poll_interval_ms = 1000
debounce_ms = 250
```

Domainenként felülírható:

```toml
[[domains]]
host = "example.com"
workdir = "/srv/rwlang/domains/example.com/current"
app = "main.rw"

[domains.reload]
enabled = true
poll_interval_ms = 1500
debounce_ms = 300
```

Karbantartáskor kikapcsolható globálisan `reload.enabled = false`, egy domainre `domains.reload.enabled = false`, vagy az egész processre a `--no-source-reload` kapcsolóval.

## Mit figyel a szerver?

A compiler visszaadja az alkalmazás tényleges source dependency graphját: a konfigurált entrypointot és minden közvetlen vagy tranzitív `mod` fájlt. A közös reload supervisor csak ezek olcsó fájlrendszer-metaadatait (`mtime + size`) ellenőrzi. Nem járja végig folyamatosan a domain teljes workdirját, nem hash-el minden fájlt, és HTTP requestenként sem végez source-ellenőrzést.

A canonical modulútvonalak mellett a konfigurált logikai `app` útvonal is figyelt. Ez az atomikus release-mintánál fontos:

```text
/srv/rwlang/domains/example.com/current -> releases/2026-09-05.2
```

A `current` symlink átállítása ezért akkor is új fordítást indít, ha a korábbi dependency graph canonical útvonalai még az előző release könyvtárába mutattak.

Az összes domaint egyetlen közös supervisor kezeli, nem domainenként külön polling thread. Minden domain a saját effektív `poll_interval_ms` értéke szerint kerül ellenőrzésre.

## Változás, debounce, fordítás, commit

Forrásváltozás észlelése után az RWLang `debounce_ms` ideig stabil fájlállapotra vár. Így egy többfájlos feltöltés nem indít minden egyes fájl után külön fordítást.

Ezután külön candidate alkalmazás fordul és ugyanazokon a hosting-validációkon megy át, mint startup/reload során. Siker esetén csak az adott domain runtime-ja cserélődik atomikusan. A régi runtime-ot már használó requestek azon fejeződnek be, az új requestek pedig az új generációt kapják.

Sikeres source reload előtt a domain public-cache route generationjei is előrelépnek. Emiatt a régi kód által generált HTML/JSON nem marad látható csak azért, mert a korábbi TTL még nem járt le.

## Új, törölt vagy késve feltöltött almodul

Új modul természetesen követhető: az új `mod` deklaráció miatt a már ismert parent source megváltozik, ez fordítást indít. Siker után a compiler új dependency graphot ad vissza, amely már tartalmazza az új modult is.

Figyelt modul törlése vagy átnevezése szintén változás. A candidate fordítás hibázik, a hiba logolódik, a korábbi működő generáció pedig aktív marad.

Több fájl feltöltésekor előfordulhat, hogy a parent modul már hivatkozik egy új child fájlra, de az még nem érkezett meg. Ilyenkor az első fordítás jogosan elbukhat. Az RWLang a stabil, sikertelen candidate-et exponenciális backoffal újrapróbálja: 2 másodperctől indul, legfeljebb 60 másodpercig ritkul. Így a később megérkező modul külön kézi reload nélkül is életbe léphet, miközben egy tartósan hibás forrás nem okoz folyamatos újrafordítást.

## Hiba és naplózás

A sikertelen automatikus reload **nem állítja le a domaint**. Az előző valid generáció fut tovább. Fontos strukturált logesemények:

- `source_change_detected`
- `source_reload_committed`
- `source_reload_rejected`
- `source_reload_cache_invalidation_failed`
- `source_reload_stale`

A `source_reload_rejected` tartalmazza a canonical domaint, az aktív generationt és a compiler/validációs hibát, ezért a szintaktikai és modulhibák diagnosztizálhatók a server logból.

## Ajánlott deployment

Normál, csak alkalmazáskódot érintő kiadásnál:

1. töltsd fel/építsd fel az új immutable release könyvtárat;
2. szükség esetén futtasd az `rwlang-server --config ... --check-config` preflightot;
3. atomikusan állítsd át a domain `current` symlinkjét, vagy frissítsd a figyelt forrásokat;
4. a source-reload supervisor lefordítja és siker esetén commitolja az új generációt;
5. ellenőrizd a `source_reload_committed` eseményt, a health/readiness állapotot és a smoke tesztet.

A listener, DB/Redis/auth kapcsolat, cgroup limit, logging sink és más process-szintű beállítás továbbra is a normál config/restart lifecycle része. Behind-proxy módban a `SIGHUP` a domain/application konfiguráció tranzakciós reloadjára szolgál; az automatikus source reload ennél könnyebb, kifejezetten alkalmazáskódra szánt út.
