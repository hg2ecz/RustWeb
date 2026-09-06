#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

if [ ! -f Cargo.lock ]; then
  echo 'release packaging refused: Cargo.lock is missing' >&2
  exit 1
fi

if [ ! -f RELEASE-MANIFEST.sha256 ]; then
  echo 'release packaging refused: generate RELEASE-MANIFEST.sha256 first' >&2
  exit 1
fi

if ! sha256sum -c RELEASE-MANIFEST.sha256 >/dev/null; then
  echo 'release packaging refused: source manifest verification failed' >&2
  exit 1
fi

OUT=${1:-rwlang-v1.0.0-source.tgz}
TMP=$(mktemp "${TMPDIR:-/tmp}/rwlang-release.XXXXXX")
trap 'rm -f "$TMP"' EXIT HUP INT TERM
rm -f "$OUT"

tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
  --exclude='./.git' --exclude='./target' \
  -cf - . | gzip -n > "$TMP"
mv "$TMP" "$OUT"
trap - EXIT HUP INT TERM
sha256sum "$OUT"
