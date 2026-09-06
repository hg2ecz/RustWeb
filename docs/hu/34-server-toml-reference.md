# 34. `server.toml` konfigurációs referencia

A production config ajánlott helye:

```text
/usr/local/etc/rwlang/server.toml
```

Indítás és validáció:

```bash
rwlang-server --config /usr/local/etc/rwlang/server.toml
rwlang-server --config /usr/local/etc/rwlang/server.toml --check-config
rwlang-server --config /usr/local/etc/rwlang/server.toml --print-effective-config
```

Precedence:

```text
built-in defaults < server.toml < explicit CLI override
```

A parser fail-closed: ismeretlen section/key és duplikált TOML elem hiba; a config legfeljebb 1 MiB, reguláris nem-symlink fájl; TOML filesystem path csak abszolút lehet. Secret kizárólag `*_file` hivatkozással kerül a trusted configból a runtime-ba.

## Sectionök

- `[server]`: alkalmazás, listener, development-cookie policy.
- `[tls]`: cert/key, handshake timeout, HTTP redirect listener, public host.
- `[database]`: DB URL secret file, insecure transport explicit policy.
- `[redis]`: Redis URL secret file, insecure transport explicit policy.
- `[auth]`: LDAP/local-auth/TOTP/role és login limiter.
- `[web]`: trusted proxy, Origin/CORS.
- `[storage]`: AppFs root/mode, upload- és pixel-limit.
- `[static_assets]`: static root, URL prefix, cache és precompressed asset policy.
- `[lifecycle]`: live/ready és graceful shutdown.
- `[observability]`: metrics listener és access logging.
- `[logging]`: server/access/audit fájlok és stderr.
- `[rate_limit]`: rate-limit policy file és memory fallback.
- `[cache]`: public cache ceilingek, single-flight wait és memory fallback.
- `[reload]`: automatikus alkalmazás-source figyelés globális alapértékei (`enabled`, `poll_interval_ms`, `debounce_ms`).
- `[limits]`: HTTP/runtime/session/process budgetek és resource profile file.
- `[cgroup]`: opcionális Linux cgroup memory/swap/CPU/PID budget külön delegált cgroupnál; systemd baseline mellett hagyd kikapcsolva.

HTTPS reverse proxy mögött a production upstream kivétel a "TLS a processben" szabály alól, de csak szűk formában: az RWLang loopbacken hallgat, `server.insecure_dev_cookies = false`, van explicit `tls.public_host`, a proxy címe szerepel a `web.trusted_proxy_cidrs` listában, és a proxy hiteles `X-Forwarded-Proto: https` (vagy `Forwarded: proto=https`) metadata-t állít elő. Certificate/key ilyenkor a proxyn marad.

A teljes kulcslista, built-in defaultok és a production sample eltérései a könyv `A server.toml konfigurációs referencia` fejezetében, illetve a `config/server.toml.sample` fájlban találhatók.

## Fontos default/sample eltérés

A built-in runtime default szigorúbb lehet a production sample-nél. Például:

- `limits.max_instructions`: built-in `100000`, sample `5000000`;
- `limits.max_runtime_alloc_bytes`: built-in 32 MiB, sample 256 MiB;
- `limits.max_process_memory_bytes`: built-in nincs, sample 1 GiB.

Az eltérés szándékos: a sample az operátor által explicit vállalt production budgetet mutatja.

## Config és alkalmazáskód változtatás

Process-szintű config változásnál:

1. config template módosítása;
2. secret külön file-ban;
3. `--check-config`;
4. `--print-effective-config`;
5. controlled restart;
6. live/ready + log/metric/audit ellenőrzés.

Behind-proxy módban a `SIGHUP` a logok újranyitása mellett tranzakciósan újraolvassa a domain/application hosting konfigurációt; listener, DB/Redis/auth kapcsolat, cgroup és más process-szintű állapot továbbra is restartot igényel.

Az alkalmazás `.rw` forrásának módosításához normál esetben nem kell `SIGHUP`: a `[reload]` supervisor a teljes ismert modulgráfot `mtime + size` alapján figyeli, debounce után candidate runtime-ot fordít, és csak siker esetén cseréli le az adott domaint. Hibás új kódnál a régi generáció marad aktív. Domainenként `[domains.reload]` blokkal írható felül, process-szinten `--no-source-reload` kapcsolóval tiltható. Részletesen: [Automatikus alkalmazás-forráskód reload](38-automatikus-forraskod-reload.md).

## Cgroup authority

A repository systemd mintája maga állít `MemoryMax`, `MemorySwapMax`, `CPUQuota` és `TasksMax` limiteket, ezért a production sample `[cgroup]` blokkja nincs bekapcsolva. Az RWLang `[cgroup]` író módja alternatív út külön, írható/delegált cgroup v2 könyvtárhoz; ne használd ugyanarra a processre a két authorityt egyszerre.
