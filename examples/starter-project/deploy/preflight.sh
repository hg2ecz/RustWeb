#!/bin/sh
set -eu

SERVER=${SERVER:-/usr/local/bin/rwlang-server}
CLI=${CLI:-/usr/local/bin/rwlang-cli}
CONFIG=${CONFIG:-/usr/local/etc/rwlang/server.toml}
APP=${APP:-/srv/rwlang/current/main.rw}
MIGRATIONS=${MIGRATIONS:-/srv/rwlang/current/migrations}
MIGRATION_DB_URL_FILE=${MIGRATION_DB_URL_FILE:-/run/secrets/rwlang/migration-db-url}

"$SERVER" --config "$CONFIG" --app "$APP" --check-config
"$CLI" check "$APP"
"$CLI" migrate verify --dir "$MIGRATIONS" --db-url-file "$MIGRATION_DB_URL_FILE"

echo "RWLang preflight passed; backup/restore readiness must be confirmed separately before schema mutation."
