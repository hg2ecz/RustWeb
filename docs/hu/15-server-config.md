# 15. Server konfigurációs állomány

Productionban az ajánlott indítás egy trusted TOML konfigurációs állomány:

```bash
rwlang-server --config /usr/local/etc/rwlang/server.toml
```

A projektben teljes, kommentelt minta található:

```text
config/server.toml.sample
```

## Precedence

```text
beépített defaultok < TOML config < CLI override
```

Például a configban lehet:

```toml
[server]
app = "/srv/rwlang/app/app.rw"
listen = "0.0.0.0:443"

[limits]
max_connections = 4096
request_timeout_ms = 15000
```

és ideiglenesen felülírható:

```bash
rwlang-server --config /usr/local/etc/rwlang/server.toml --max-connections 512
```

## Secret szabály

A TOML **nem fogad plaintext secret mezőket**. DB, Redis, LDAP bind és local-auth URL csak fájlhivatkozással adható meg:

```toml
[database]
url_file = "/run/secrets/rwlang/database-url"

[redis]
url_file = "/run/secrets/rwlang/redis-url"
```

Az olyan kulcs, mint `database.url`, ismeretlen kulcs és startup error.

## Fail-closed parser

- ismeretlen section/key: error;
- duplikált key/table: error;
- config file nem lehet symlink;
- maximum 1 MiB;
- TOML-ból érkező filesystem path csak abszolút lehet;
- a normál server startup-validációk ugyanúgy érvényesek.

## Ellenőrzés

```bash
rwlang-server --config /usr/local/etc/rwlang/server.toml --check-config
```

Ez listener indítása nélkül ellenőrzi a TOML-t, a secret-file referenciákat, lefordítja az alkalmazást, validálja a route rate-limit/resource-profile hivatkozásokat és beolvassa/parse-olja a TLS cert/key párost. DB/Redis hálózati connectivity-t nem tesztel; arra a readiness endpoint és az integration tesztek szolgálnak.

Effective, secretmentes összefoglaló:

```bash
rwlang-server --config /usr/local/etc/rwlang/server.toml --print-effective-config
```

A kimenet secret értéket nem ír ki; csak azt jelzi, hogy DB/Redis/auth konfigurálva van-e.

## CLI használat

A CLI opciók megmaradnak developmentre és célzott override-ra. Productionban a nagy, stabil konfiguráció legyen TOML-ban, ne hosszú systemd `ExecStart` argumentumlistában.

## Logging

```toml
[logging]
server_file = "/var/log/rwlang/server.log"
access_file = "/var/log/rwlang/access.log"
audit_file = "/var/log/rwlang/audit.log"
stderr = true
```

A log pathok configból abszolútak, a parent directoryt az operator hozza létre. A server a fájlt létrehozza, ha nem létezik.

A config precedence továbbra is `defaults < config < CLI`. Behind-proxy módban a **SIGHUP a log-reopen mellett a domain/application hosting állapotot tranzakciósan újraépíti**, de listener, DB/Redis/auth kapcsolat, cgroup és más process-szintű config változásához továbbra is `--check-config`, majd controlled restart ajánlott. Az `.rw` alkalmazásforrás változását normál esetben az automatikus `[reload]` supervisor kezeli külön SIGHUP nélkül.
