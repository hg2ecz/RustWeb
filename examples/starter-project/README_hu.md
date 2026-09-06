# RWLang starter project

Ez a könyvtár M45 production-közeli kiindulópont. Nem demo deployment defaults: a hostneveket, secreteket, limiteket és OS pathokat operatornak kell beállítania.

## Fejlesztés

```bash
cargo run --locked -q -p rwlang-cli -- check examples/starter-project/main.rw
```

A projekt `main.rw` entrypointból és explicit deklarált, névtérbe rendezett forrásfájlokból áll. A `mod models;` a `models.rw` fájlt a `models` névtérként tölti be; az `Article` és más deklarációk nem kerülnek globális scope-ba. Modulhatáron át ezért kvalifikált név kell, például `models::Article` vagy `queries::articleBySlug(...)`. A migration külön deploy művelet; az alkalmazásszerver nem futtat automatikus schema migrationt.

## Release artifact layout

Ajánlott immutable release könyvtár:

```text
/srv/rwlang/releases/2026-09-04.1/
  main.rw
  models.rw
  queries.rw
  pages.rw
  actions.rw
  migrations/
  public/
/srv/rwlang/current -> /srv/rwlang/releases/2026-09-04.1
```

A release könyvtár read-only. Írható adat kizárólag a külön `/srv/rwlang/data` alatt legyen.

## A mellékelt service/config authority modellje

A starter `deploy/server.toml` közvetlen TLS-t mutat 80/443 porton, miközben a process `rwlang` userként fut. Ezért a párosított systemd unit csak a `CAP_NET_BIND_SERVICE` capabilityt adja a service-nek; root futtatás nem szükséges. Ha Apache/Nginx mögött `127.0.0.1:8080` listenert használsz, ezt a capabilityt távolítsd el.

A process cgroup hard limitjeinek authorityja ebben a starterben **systemd** (`MemoryMax`, `MemorySwapMax`, `CPUQuota`, `TasksMax`). Emiatt a `server.toml` nem kapcsolja be az RWLang `[cgroup]` író módját. A két cgroup authorityt ne használd egyszerre.

## Production sorrend

1. build + `./verify.sh`;
2. artifact SHA-256 rögzítése;
3. adatbázis-backup és tényleges restore-readiness ellenőrzése;
4. `deploy/preflight.sh`;
5. `migrate apply` külön migration credentialdel;
6. új immutable release telepítése;
7. `current` symlink atomikus átállítása;
8. controlled `systemctl restart rwlang`;
9. `/health/live` és `/health/ready` ellenőrzése;
10. log/metrics/audit ellenőrzése.

Rollbacknél az alkalmazás artifact visszaállítható korábbira, de schema rollbacket nem szabad automatikusan feltételezni. A DB kompatibilitást előre kell megtervezni; részletes backup/restore/upgrade/rollback szerződés M46 feladata.

## M46 recovery

Részletes operator contract: `../../docs/hu/30-backup-restore-upgrade-rollback.md`.

A starter `deploy/release-record.sh` secretmentes release/config/migration hash rekordot készít. A `deploy/restore-verify.sh` csak izolált restore-test környezetben, read-only módon futtat config/source/migration ellenőrzést; production pathokra fail-closed módon megtagadja a futást.

A V1 safe backup baseline controlled service stop/drain után application DB + local-auth DB + teljes AppFs data root mentés. Redis session/cache/rate-limit állapot restore-ja nem szükséges; session reset elfogadott.
