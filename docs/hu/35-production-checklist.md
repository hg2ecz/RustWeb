# Production checklist és minták

A könyv függeléke összefoglalja a production telepítéshez használható gyorsreferenciát. A részletes indoklásért lásd a security, observability, deployment, recovery és server-config dokumentációt.

## Alap útvonalak

- binárisok: `/usr/local/bin/rwlang-server`, `/usr/local/bin/rwlang-cli`
- konfiguráció: `/usr/local/etc/rwlang/server.toml`
- secret fájlok: `/run/secrets/rwlang/`
- alkalmazás/release/data: `/srv/rwlang/`
- logok: `/var/log/rwlang/`

## Release gate

1. locked release build és `rwlang-cli check`;
2. migration `status -> verify -> apply -> verify`;
3. `rwlang-server --check-config`;
4. secret-, proxy-, auth-, CSRF/CORS-, DB-, storage-, Redis- és egress-policy review;
5. systemd/cgroup/resource limitek;
6. log/metrics/readiness;
7. friss és restore-drillel bizonyított backup;
8. rollback terv;
9. deploy után liveness + readiness + üzleti smoke.

Konkrét minták:

- `examples/systemd/rwlang.service`
- `examples/logrotate/rwlang`
- `examples/starter-project/deploy/`
