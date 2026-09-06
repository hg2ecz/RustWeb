# 33. RWLang CLI és napi workflow

A publikus binárisok:

```text
/usr/local/bin/rwlang-cli
/usr/local/bin/rwlang-server
```

## Feladatok

- `rwlang-cli check <app.rw>`: compiler-szintű source ellenőrzés.
- `rwlang-cli migrate status|verify|apply --dir <migrations> --db-url-file <path>`: explicit migration lifecycle.
- `rwlang-cli auth ... --db-url-file <path>`: local-auth identity store adminisztráció.
- `rwlang-server --config /usr/local/etc/rwlang/server.toml --check-config`: startup preflight.
- `rwlang-server --config ... --print-effective-config`: secret-redacted effective config.

A CLI nem HTTP server és nem olvassa általánosan a teljes `server.toml`-t. Migration és local-auth admin esetén a szükséges DB URL külön secret fájlból érkezik.

## Fejlesztői ciklus

```text
edit
-> rwlang-cli check
-> migrate verify
-> szükség esetén dev migrate apply + verify
-> rwlang-server --check-config
-> server start
-> HTTP/integration test
```

## Release ciklus

```text
immutable release
-> rwlang-cli check
-> rwlang-server --check-config
-> migrate status + verify
-> backup/recovery gate
-> approved migrate apply
-> migrate verify
-> controlled restart/switch
-> live + ready + smoke
-> log/metric/audit review
```

A server startup nem futtat automatikus migrationt.

## Local-auth parancsok

```text
rwlang-cli auth init
rwlang-cli auth user-add
rwlang-cli auth password-set
rwlang-cli auth disable
rwlang-cli auth enable
rwlang-cli auth roles-set
rwlang-cli auth totp-enroll
rwlang-cli auth totp-disable
```

A TOTP enrolment secretet és egyszer megjelenő recovery code-okat ír stdout-ra; ezt csak kontrollált operátori terminálon futtasd.

## Secret file contract

A DB URL secret file regular, nem symlink, legfeljebb 16 KiB és pontosan egy nem üres sor. A password file regular, nem symlink, legfeljebb 4096 byte és egyetlen sor; a jelszó 12..1024 byte.

## Platform build

Ha magát az RWLang workspace-et release-eljük:

```bash
./verify.sh
cargo build --locked --release -p rwlang-server -p rwlang-cli
```

Eredmény:

```text
target/release/rwlang-server
target/release/rwlang-cli
```
