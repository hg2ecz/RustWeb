# RWLang — Rust Web Language

RWLang egy Rust-alapú, webalkalmazás-fejlesztésre specializált nyelv/runtime/server ökoszisztéma. A V1 fókusza: secure-by-default működés, typed input/output, compiler által kikényszerített policyk, explicit capabilityk, auditálhatóság és production üzemeltethetőség.

## Első lépések

Webfejlesztőként innen indulj:

1. [Webalkalmazás-fejlesztői kézikönyv](docs/hu/README.md)
2. [Gyors kezdés](docs/hu/01-gyors-kezdes.md)
3. [Starter project és production deployment](docs/hu/29-production-deployment.md)
4. [Security checklist](docs/hu/15-security-checklist.md)

A teljes referenciaalkalmazás: `examples/starter-project/`.

## Build és verifikáció

A workspace Rust edition 2024-et használ, de nincs konkrét compiler/toolchain verzióhoz pinelve. A reprodukálható dependency feloldást a committed `Cargo.lock` és a `--locked` build adja.

```bash
./verify.sh
```

A release előtt a teljes `verify.sh` legyen zöld. A kiadási döntéshez a canonical angol [release checklist](RELEASE-CHECKLIST.md) szerinti automatizált és környezetfüggő evidence is szükséges.

## Production indítás

Productionban a TOML config az elsődleges interface:

```bash
rwlang-server --config /usr/local/etc/rwlang/server.toml
```

Preflight:

```bash
rwlang-server --config /usr/local/etc/rwlang/server.toml --check-config
```

Precedence:

```text
defaults < TOML config < célzott CLI override
```

A hosszú production CLI flaglista nem ajánlott; a stabil policyk trusted configban legyenek. SIGHUP újranyitja a logokat és behind-proxy módban tranzakciósan újratölti a domain/application hosting állapotot. Az alkalmazás-források változását a közös source-reload supervisor automatikusan is észleli; a process-szintű beállításokhoz továbbra is restart kell. Részletesen: `docs/hu/38-automatikus-forraskod-reload.md`.

## Debian csomag

Debian/Ubuntu build gépen a `make deb` telepíthető `rwlang_1.0.0-1_<arch>.deb` csomagot készít. A Debian csomagkezelő által birtokolt telepítés szándékosan `/usr/bin` és `/etc/rwlang` útvonalakat használ a kézi `/usr/local` telepítés helyett. Részletek: [docs/hu/36-debian-csomag.md](docs/hu/36-debian-csomag.md).

## V1 fő capabilityk

- typed routing, forms és JSON API;
- typed SQL bind/decode, migrations és optimistic locking;
- local/LDAP auth, TOTP, role/permission és object authorization;
- first-class `Date`, `DateTime`, `Uuid`, `Decimal`, `Slug`, `Email`, `Url` és enum;
- domain objectek és modulrendszer;
- safe HTML/components/layout/Markdown/Image;
- CSRF, Host/Origin/Fetch Metadata, HTTPS és static/media confinement;
- AppFs és IPv4/IPv6 outbound-network capability policy;
- rate limiting, public cache és resource profile-ok;
- structured server/access/audit log és business audit trail;
- canonical slug redirect, PRG/flash és 409 conflict UX;
- config-first deployment, backup/restore/upgrade/rollback operator flow.

## Tudatos V1 non-goalok

A következők nem részei a jelenlegi V1 magnak: full-text search, background jobs, email küldés, scheduler, revision history, workflow engine, soft delete, admin CRUD generation, module visibility/export, S3 media, image resize/thumbnail, HTTP/2/3, SSE/WebSocket, OTel SDK és private cache.

Különösen: RWLangban továbbra sincs Rust-szerű általános `pub`/`mut` nyelvi surface. A modulok source organizationt adnak; visibility/export későbbi, külön tervezendő capability. Compute kódban meglévő lokális explicit `set name = expr` formában módosítható statikus típussal és instruction budget alatt; a tartós üzleti állapotmódosítás továbbra is explicit query/transaction/action útvonalakon történik.

## Operator dokumentáció

- [Server config](docs/hu/15-server-config.md)
- [Observability](docs/hu/09-observability.md)
- [Production deployment](docs/hu/29-production-deployment.md)
- [Backup/restore/upgrade/rollback](docs/hu/30-backup-restore-upgrade-rollback.md)
- [IPv6 egress](docs/32-ipv6-egress.md)
- [CLI workflow](docs/hu/33-cli-workflow.md)
- [`server.toml` referencia](docs/hu/34-server-toml-reference.md)
- [Production checklist](docs/hu/35-production-checklist.md)
- [Release checklist](RELEASE-CHECKLIST.md)

## Security

A webes security ellenőrzőlista: [docs/hu/15-security-checklist.md](docs/hu/15-security-checklist.md). A projekt fail-closed elve: ismeretlen vagy nem bizonyítható állapotnál ne legyen implicit permissive fallback.


Végrehajtás: az alkalmazáskifejezéseket korlátos stack-alapú bytecode VM futtatja; natív JIT még nincs. Lásd: `docs/hu/39-bytecode-vegrehajtas.md`.
