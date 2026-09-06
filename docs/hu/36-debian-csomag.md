# Debian csomag készítése és telepítése `dpkg -i` paranccsal

Az RWLang két telepítési konvenciót támogat. A kézi/forrásból történő telepítés a projektben használt lokális adminisztrátori prefixet követi:

```text
/usr/local/bin/rwlang-server
/usr/local/bin/rwlang-cli
/usr/local/etc/rwlang/server.toml
```

A Debian csomag más: a `dpkg` által birtokolt fájlok Debian-konvenció szerint kerülnek a rendszerbe. A `.deb` ezért `/usr/bin` alá telepíti a binárisokat és `/etc/rwlang` alá a konfigurációt; csomagkezelt fájlt nem tesz `/usr/local` alá.

## Csomag készítése

Debian/Ubuntu build gépen, Rust és `dpkg-deb` mellett:

```bash
make deb
```

vagy közvetlenül:

```bash
tools/package-deb.sh
```

A script locked release buildet készít a két publikus binárisból, majd alapértelmezés szerint létrehozza:

```text
dist/rwlang_1.0.0-1_<arch>.deb
```

Hasznos felülírások:

```bash
tools/package-deb.sh --version 1.0.0-2
tools/package-deb.sh --output-dir /tmp/packages
```

CI-ban már elkészített binárisok is csomagolhatók:

```bash
tools/package-deb.sh --skip-build --bin-dir target/release
```

## A csomag tartalma

```text
/usr/bin/rwlang-server
/usr/bin/rwlang-cli
/etc/rwlang/server.toml
/etc/rwlang/rate-limits.toml
/etc/rwlang/resource-profiles.toml
/usr/lib/systemd/system/rwlang.service
/usr/lib/tmpfiles.d/rwlang.conf
/etc/logrotate.d/rwlang
/usr/share/doc/rwlang/
```

Az `/etc/rwlang/server.toml`, a rate-limit/resource-profile policy fájlok és az `/etc/logrotate.d/rwlang` conffile-ok, ezért a helyi operátori módosításokat a `dpkg` frissítéskor a szokásos Debian módon kezeli.

A csomag létrehozza az `rwlang` rendszerfelhasználót/csoportot, valamint a `/srv/rwlang/data`, `/var/log/rwlang` és `/run/secrets/rwlang` könyvtárakat. A szolgáltatást szándékosan **nem indítja el automatikusan**, mert egy általános runtime csomag nem ismerheti az alkalmazást, a credentialöket, TLS kulcsokat, adatbázis URL-t és public hostot.

## Telepítés

```bash
sudo dpkg -i dist/rwlang_1.0.0-1_amd64.deb
```

Ezután telepítsd az alkalmazást és a secreteket, állítsd be az `/etc/rwlang/server.toml` fájlt, majd ellenőrizd:

```bash
sudo rwlang-server --config /etc/rwlang/server.toml --check-config
```

Csak sikeres ellenőrzés után indítsd:

```bash
sudo systemctl enable --now rwlang.service
```

## Közvetlen TLS és reverse proxy

A csomagolt konfigurációs minta a repository direct-TLS mintáját követi 80/443 porton. Ezért a systemd unit kizárólag `CAP_NET_BIND_SERVICE` capabilityt kap, így az unprivileged `rwlang` user is tud privileged portra bindolni.

Apache/Nginx mögött, például `127.0.0.1:8080` listenerrel ezt a capabilityt site-specific systemd override-ban távolítsd el. A TLS authority ilyenkor a reverse proxy, az RWLang oldalon pedig explicit `public_host` és trusted proxy CIDR szükséges.
