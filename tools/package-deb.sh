#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

VERSION=${RWLANG_DEB_VERSION:-1.0.0-1}
OUT_DIR=${RWLANG_DEB_OUT_DIR:-dist}
SKIP_BUILD=${RWLANG_DEB_SKIP_BUILD:-0}
BIN_DIR=${RWLANG_DEB_BIN_DIR:-target/release}
MAINTAINER=${RWLANG_DEB_MAINTAINER:-RWLang Project <noreply@localhost>}
ARCH=${RWLANG_DEB_ARCH:-}

usage() {
  cat <<'USAGE'
Usage: tools/package-deb.sh [options]

Build a Debian binary package containing rwlang-server and rwlang-cli.

Options:
  --version VERSION     Debian version, default: 1.0.0-1
  --output-dir DIR      Output directory, default: dist
  --skip-build          Do not run Cargo; use existing binaries
  --bin-dir DIR         Directory containing rwlang-server/rwlang-cli
  --arch ARCH           Debian architecture override
  -h, --help            Show this help

Environment equivalents:
  RWLANG_DEB_VERSION, RWLANG_DEB_OUT_DIR, RWLANG_DEB_SKIP_BUILD,
  RWLANG_DEB_BIN_DIR, RWLANG_DEB_ARCH, RWLANG_DEB_MAINTAINER
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      [ "$#" -ge 2 ] || { echo 'missing value for --version' >&2; exit 2; }
      VERSION=$2
      shift 2
      ;;
    --output-dir)
      [ "$#" -ge 2 ] || { echo 'missing value for --output-dir' >&2; exit 2; }
      OUT_DIR=$2
      shift 2
      ;;
    --skip-build)
      SKIP_BUILD=1
      shift
      ;;
    --bin-dir)
      [ "$#" -ge 2 ] || { echo 'missing value for --bin-dir' >&2; exit 2; }
      BIN_DIR=$2
      shift 2
      ;;
    --arch)
      [ "$#" -ge 2 ] || { echo 'missing value for --arch' >&2; exit 2; }
      ARCH=$2
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case "$VERSION" in
  *[!0-9A-Za-z.+:~-]*|'')
    echo "invalid Debian version: $VERSION" >&2
    exit 2
    ;;
esac

command -v dpkg-deb >/dev/null 2>&1 || {
  echo 'dpkg-deb is required to build the Debian package' >&2
  exit 1
}

if [ -z "$ARCH" ]; then
  if command -v dpkg-architecture >/dev/null 2>&1; then
    ARCH=$(dpkg-architecture -qDEB_HOST_ARCH)
  else
    case "$(uname -m)" in
      x86_64) ARCH=amd64 ;;
      aarch64|arm64) ARCH=arm64 ;;
      armv7l) ARCH=armhf ;;
      *) echo 'cannot determine Debian architecture; use --arch' >&2; exit 1 ;;
    esac
  fi
fi

case "$BIN_DIR" in
  /*) : ;;
  *) BIN_DIR="$ROOT/$BIN_DIR" ;;
esac

if [ "$SKIP_BUILD" != 1 ]; then
  command -v cargo >/dev/null 2>&1 || {
    echo 'cargo is required unless --skip-build is used' >&2
    exit 1
  }
  cargo build --locked --release -p rwlang-server -p rwlang-cli
fi

SERVER_BIN="$BIN_DIR/rwlang-server"
CLI_BIN="$BIN_DIR/rwlang-cli"
for bin in "$SERVER_BIN" "$CLI_BIN"; do
  [ -f "$bin" ] || { echo "missing binary: $bin" >&2; exit 1; }
  [ -x "$bin" ] || { echo "binary is not executable: $bin" >&2; exit 1; }
done

mkdir -p "$OUT_DIR"
OUT_DIR=$(CDPATH= cd -- "$OUT_DIR" && pwd)
TMP=$(mktemp -d "${TMPDIR:-/tmp}/rwlang-deb.XXXXXX")
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
PKGROOT="$TMP/pkg"
mkdir -p \
  "$PKGROOT/DEBIAN" \
  "$PKGROOT/usr/bin" \
  "$PKGROOT/usr/lib/systemd/system" \
  "$PKGROOT/usr/lib/tmpfiles.d" \
  "$PKGROOT/usr/share/doc/rwlang" \
  "$PKGROOT/etc/rwlang" \
  "$PKGROOT/etc/logrotate.d"

install -m 0755 "$SERVER_BIN" "$PKGROOT/usr/bin/rwlang-server"
install -m 0755 "$CLI_BIN" "$PKGROOT/usr/bin/rwlang-cli"

# Debian packages own /usr and /etc. Manual/source installs intentionally use
# /usr/local/bin and /usr/local/etc/rwlang instead.
sed \
  -e 's#/usr/local/etc/rwlang#/etc/rwlang#g' \
  config/server.toml.sample > "$PKGROOT/etc/rwlang/server.toml"
chmod 0644 "$PKGROOT/etc/rwlang/server.toml"
install -m 0644 examples/starter-project/deploy/rate-limits.toml \
  "$PKGROOT/etc/rwlang/rate-limits.toml"
install -m 0644 examples/starter-project/deploy/resource-profiles.toml \
  "$PKGROOT/etc/rwlang/resource-profiles.toml"

# The packaged default follows the direct-TLS sample (ports 80/443), so the
# unprivileged service receives only CAP_NET_BIND_SERVICE. Reverse-proxy
# deployments on an unprivileged port should remove these two lines.
sed \
  -e 's#/usr/local/bin#/usr/bin#g' \
  -e 's#/usr/local/etc/rwlang#/etc/rwlang#g' \
  -e '/^# Baseline assumes an unprivileged listener/,/^# CapabilityBoundingSet=CAP_NET_BIND_SERVICE$/d' \
  examples/systemd/rwlang.service \
  | awk '
      { print }
      /^LimitNOFILE=/ {
        print "AmbientCapabilities=CAP_NET_BIND_SERVICE"
        print "CapabilityBoundingSet=CAP_NET_BIND_SERVICE"
      }
    ' > "$PKGROOT/usr/lib/systemd/system/rwlang.service"
chmod 0644 "$PKGROOT/usr/lib/systemd/system/rwlang.service"

install -m 0644 examples/logrotate/rwlang "$PKGROOT/etc/logrotate.d/rwlang"

cat > "$PKGROOT/usr/lib/tmpfiles.d/rwlang.conf" <<'EOF_TMPFILES'
d /srv/rwlang 0755 root root -
d /srv/rwlang/data 0750 rwlang rwlang -
d /var/log/rwlang 0750 rwlang rwlang -
d /run/secrets/rwlang 0750 root rwlang -
EOF_TMPFILES

install -m 0644 README.md "$PKGROOT/usr/share/doc/rwlang/README.md"
install -m 0644 RELEASE-NOTES-V1.0.md "$PKGROOT/usr/share/doc/rwlang/RELEASE-NOTES-V1.0.md"

cat > "$PKGROOT/DEBIAN/conffiles" <<'EOF_CONFFILES'
/etc/rwlang/server.toml
/etc/rwlang/rate-limits.toml
/etc/rwlang/resource-profiles.toml
/etc/logrotate.d/rwlang
EOF_CONFFILES

SHLIB_DEPS=''
if command -v dpkg-shlibdeps >/dev/null 2>&1; then
  DEBHELP="$TMP/debhelp"
  mkdir -p "$DEBHELP/debian"
  cat > "$DEBHELP/debian/control" <<'EOF_CONTROL_HELP'
Source: rwlang
Section: web
Priority: optional
Maintainer: RWLang Project <noreply@localhost>
Standards-Version: 4.6.2

Package: rwlang
Architecture: any
Description: RWLang web application runtime and CLI
EOF_CONTROL_HELP
  SHLIB_LINE=$(cd "$DEBHELP" && dpkg-shlibdeps -O "$SERVER_BIN" "$CLI_BIN" 2>/dev/null || true)
  case "$SHLIB_LINE" in
    shlibs:Depends=*) SHLIB_DEPS=${SHLIB_LINE#shlibs:Depends=} ;;
  esac
fi

if [ -n "$SHLIB_DEPS" ]; then
  DEPENDS="$SHLIB_DEPS, adduser, ca-certificates"
else
  # Fallback for minimal build containers where dpkg-shlibdeps cannot resolve
  # the host shlibs database. Debian build hosts normally take the branch above.
  DEPENDS='libc6, adduser, ca-certificates'
fi

INSTALLED_SIZE=$(du -sk "$PKGROOT" | awk '{print $1}')
cat > "$PKGROOT/DEBIAN/control" <<EOF_CONTROL
Package: rwlang
Version: $VERSION
Section: web
Priority: optional
Architecture: $ARCH
Maintainer: $MAINTAINER
Depends: $DEPENDS
Installed-Size: $INSTALLED_SIZE
Description: RWLang web application runtime and CLI
 RWLang is a model-centered web application language/runtime. This package
 installs rwlang-server, rwlang-cli, a production configuration template,
 a systemd unit, logrotate policy, and runtime directory definitions.
EOF_CONTROL

cat > "$PKGROOT/DEBIAN/postinst" <<'EOF_POSTINST'
#!/bin/sh
set -e

if ! getent group rwlang >/dev/null 2>&1; then
  addgroup --system rwlang >/dev/null
fi
if ! getent passwd rwlang >/dev/null 2>&1; then
  adduser --system --ingroup rwlang --home /srv/rwlang --no-create-home \
    --shell /usr/sbin/nologin rwlang >/dev/null
fi

install -d -m 0755 -o root -g root /srv/rwlang
install -d -m 0750 -o rwlang -g rwlang /srv/rwlang/data /var/log/rwlang
install -d -m 0750 -o root -g rwlang /run/secrets/rwlang

if command -v systemd-tmpfiles >/dev/null 2>&1; then
  systemd-tmpfiles --create /usr/lib/tmpfiles.d/rwlang.conf || true
fi
if command -v systemctl >/dev/null 2>&1; then
  systemctl daemon-reload >/dev/null 2>&1 || true
fi

cat <<'EOF_MESSAGE'
RWLang installed.

The service is intentionally not started automatically. Before enabling it:
  1. deploy an application to /srv/rwlang/current/;
  2. configure /etc/rwlang/server.toml;
  3. install required secrets under /run/secrets/rwlang/;
  4. validate: rwlang-server --config /etc/rwlang/server.toml --check-config
  5. enable/start: systemctl enable --now rwlang.service

The packaged unit carries CAP_NET_BIND_SERVICE because the default template
uses direct TLS on 80/443. For Apache/Nginx reverse proxy deployments using
127.0.0.1:8080, remove AmbientCapabilities and CapabilityBoundingSet from a
systemd override or use a site-specific unit without that capability.
EOF_MESSAGE
EOF_POSTINST
chmod 0755 "$PKGROOT/DEBIAN/postinst"

cat > "$PKGROOT/DEBIAN/postrm" <<'EOF_POSTRM'
#!/bin/sh
set -e
if command -v systemctl >/dev/null 2>&1; then
  systemctl daemon-reload >/dev/null 2>&1 || true
fi
exit 0
EOF_POSTRM
chmod 0755 "$PKGROOT/DEBIAN/postrm"

SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-0}
export SOURCE_DATE_EPOCH
find "$PKGROOT" -exec touch -h -d "@$SOURCE_DATE_EPOCH" {} +

OUT="$OUT_DIR/rwlang_${VERSION}_${ARCH}.deb"
rm -f "$OUT"
dpkg-deb --root-owner-group --build "$PKGROOT" "$OUT" >/dev/null

dpkg-deb --info "$OUT" >/dev/null
dpkg-deb --contents "$OUT" >/dev/null

sha256sum "$OUT"
echo "$OUT"
