# RWLang — Rust Web Language

RWLang egy Rust-alapú, webalkalmazás-fejlesztésre specializált nyelv/runtime/server ökoszisztéma. A V1 fókusza: secure-by-default működés, typed input/output, compiler által kikényszerített policyk, explicit capabilityk, auditálhatóság és production üzemeltethetőség.

## Secure-by-construction nyelvi alap

A route-ok többé nem publikusak implicit módon: minden route `public` vagy `auth ...` policyt kér. A dinamikus String redirect tiltott, a normál redirect csak compiler által ellenőrzött lokális útvonal-literal lehet. A külső query/form/JSON `String` értékek explicit szabály nélkül automatikus 4096 karakteres felső korlátot kapnak. Részletek: [`docs/hu/57-secure-by-construction-foundation.md`](docs/hu/57-secure-by-construction-foundation.md).


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

- typed routing/forms/JSON, domain és nominális típusok, exhaustive enum `match`;
- typed SQL, statikus DB row-bound, optimistic locking és explicit transaction outcome;
- local/LDAP auth, TOTP/MFA, permission, object/mutation authorization és auth abuse protection;
- platform-owned session + purpose-safe token hash/expiry/revoke/rotation lifecycle;
- `Secret<T>` / `Sensitive<T>` flow, explicit public projection és trusted `redact(...)`;
- critical-operation contract audit/transaction/idempotency garanciákkal;
- tenant authority és compiler-enforced tenant isolation;
- verified webhook replay protection és staged/verified file publish;
- effect/capability modell és named SSRF-hardened outbound integration;
- typed HTTP metadata, generált CSP/security headerek és strict production policy;
- purpose/lifecycle typed crypto keyek és AES-256-GCM authenticated encryption;
- request/DB/file/outbound resource cap, hard deadline és cumulative I/O budget;
- structured security event/redaction/burst alerting alap;
- supply-chain capability/provenance lock és release evidence.

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

A canonical aktuális állapot: [SECURITY-STATUS.md](SECURITY-STATUS.md). A webes/production ellenőrzőlista: [docs/hu/15-security-checklist.md](docs/hu/15-security-checklist.md). A projekt fail-closed elve: ismeretlen vagy nem bizonyítható állapotnál ne legyen implicit permissive fallback.


Végrehajtás: az alkalmazáskifejezéseket korlátos stack-alapú bytecode VM futtatja; natív JIT még nincs. Lásd: `docs/hu/39-bytecode-vegrehajtas.md`.
