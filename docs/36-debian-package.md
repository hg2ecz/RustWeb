# Debian package build and `dpkg -i` installation

RWLang supports two installation conventions. A manual/source install uses the local-administrator prefix documented throughout the project:

```text
/usr/local/bin/rwlang-server
/usr/local/bin/rwlang-cli
/usr/local/etc/rwlang/server.toml
```

A Debian package is different: files owned by `dpkg` follow Debian filesystem conventions. The `.deb` therefore installs the binaries under `/usr/bin` and the conffile under `/etc/rwlang` rather than writing package-managed files into `/usr/local`.

## Build the package

On a Debian/Ubuntu build host with Rust and `dpkg-deb` installed:

```bash
make deb
```

Equivalent direct invocation:

```bash
tools/package-deb.sh
```

The script performs a locked release build of both public binaries and creates, by default:

```text
dist/rwlang_1.0.0-1_<arch>.deb
```

Useful overrides:

```bash
tools/package-deb.sh --version 1.0.0-2
tools/package-deb.sh --output-dir /tmp/packages
```

CI may package already-built binaries without invoking Cargo:

```bash
tools/package-deb.sh --skip-build --bin-dir target/release
```

## Package contents

The package installs:

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

`/etc/rwlang/server.toml`, the rate-limit/resource-profile policy files, and `/etc/logrotate.d/rwlang` are conffiles, so local operator edits are preserved by `dpkg` across upgrades in the normal Debian manner.

The package also creates the system user/group `rwlang` and the runtime directories `/srv/rwlang/data`, `/var/log/rwlang`, and `/run/secrets/rwlang` with restrictive ownership. It intentionally does **not** start the service automatically: a generic runtime package cannot know the deployed application, credentials, TLS keys, database URL, or public host.

## Install

```bash
sudo dpkg -i dist/rwlang_1.0.0-1_amd64.deb
```

If the local system reports unrelated dependency issues, resolve them through the package manager, for example:

```bash
sudo apt-get -f install
```

Then deploy the application and secrets, edit `/etc/rwlang/server.toml`, and validate it:

```bash
sudo rwlang-server --config /etc/rwlang/server.toml --check-config
```

Only after validation should the service be enabled:

```bash
sudo systemctl enable --now rwlang.service
```

## Direct TLS versus reverse proxy

The packaged configuration template follows the repository's direct-TLS sample on ports 80/443. The packaged unit therefore carries only `CAP_NET_BIND_SERVICE`, allowing the unprivileged `rwlang` user to bind those ports.

For Apache or Nginx deployments where RWLang listens on an unprivileged loopback port such as `127.0.0.1:8080`, remove that capability in a site-specific systemd override. The reverse proxy remains the TLS authority and RWLang should be configured with explicit `public_host` and trusted proxy CIDRs as described in the deployment chapters.
