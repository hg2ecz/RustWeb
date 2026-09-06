#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

fail() { echo "architecture: $*" >&2; exit 1; }

internal_deps() {
  cargo_toml=$1
  grep -o 'path = "../[^"]*"' "$cargo_toml" 2>/dev/null \
    | sed 's#.*path = "../##; s#"##' \
    | LC_ALL=C sort -u || true
}

check_allowed_deps() {
  crate=$1
  allowed=" $2 "
  cargo_toml="crates/$crate/Cargo.toml"
  [ -f "$cargo_toml" ] || fail "missing $cargo_toml"
  for dep in $(internal_deps "$cargo_toml"); do
    case "$allowed" in
      *" $dep "*) : ;;
      *) fail "$crate must not depend on internal crate $dep (allowed:$allowed)" ;;
    esac
  done
}

max_lines() {
  file=$1
  max=$2
  lines=$(wc -l < "$file" | tr -d ' ')
  [ "$lines" -le "$max" ] || fail "$file grew to $lines lines (architecture budget $max)"
}

# Internal dependency direction. These are allow-lists, so a newly introduced
# cross-layer dependency fails until its architectural role is reviewed.
check_allowed_deps language-core ""
check_allowed_deps data ""
check_allowed_deps integrations ""
check_allowed_deps storage ""
check_allowed_deps observability ""
check_allowed_deps auth "data"
check_allowed_deps compiler "language-core"
check_allowed_deps migrations "data"
check_allowed_deps runtime "language-core data compiler"
check_allowed_deps cli "auth compiler migrations"
check_allowed_deps server "auth compiler data language-core runtime storage observability"

# Production error APIs must stay typed. Trait-object transport abstractions are
# allowed, but dynamic error erasure is not.
boxed_errors=$(grep -RIl --include='*.rs' -E 'Box<dyn (std::error::Error|Error)>' crates 2>/dev/null || true)
[ -z "$boxed_errors" ] || fail "boxed error API found; use a typed orchestration error: $boxed_errors"

string_errors=$(grep -RIl --include='*.rs' -E 'Result<.*,[[:space:]]*String>' crates 2>/dev/null || true)
[ -z "$string_errors" ] || fail "String error API found; use a typed error: $string_errors"

# Public façades should stay façades. These budgets are regression alarms, not
# invitations to split cohesive code solely to satisfy a line count.
max_lines crates/server/src/main.rs 600
max_lines crates/server/src/operations.rs 280
max_lines crates/server/src/request_input.rs 160
max_lines crates/compiler/src/lib.rs 200
max_lines crates/compiler/src/expression.rs 500
max_lines crates/compiler/src/source_syntax.rs 230
max_lines crates/compiler/src/sql_syntax.rs 110
max_lines crates/auth/src/lib.rs 380
max_lines crates/runtime/src/lib.rs 120
max_lines crates/runtime/src/request_execution.rs 260
max_lines crates/runtime/src/statement_execution.rs 450
max_lines crates/runtime/src/response.rs 40

# Known one-file hotspots are frozen against further growth until their planned
# responsibility-based decomposition is complete.
max_lines crates/data/src/lib.rs 120
max_lines crates/data/src/database.rs 320
max_lines crates/data/src/sql.rs 360
max_lines crates/data/src/redis_store.rs 260
max_lines crates/data/src/types.rs 120
max_lines crates/data/src/error.rs 100
max_lines crates/language-core/src/lib.rs 80
max_lines crates/language-core/src/ast.rs 450
max_lines crates/language-core/src/values.rs 280
max_lines crates/language-core/src/web_types.rs 140
max_lines crates/language-core/src/schema.rs 100
max_lines crates/language-core/src/query.rs 80
max_lines crates/language-core/src/routing.rs 100
max_lines crates/language-core/src/program.rs 100
max_lines crates/language-core/src/config.rs 80
max_lines crates/language-core/src/error.rs 80
max_lines crates/migrations/src/lib.rs 80
max_lines crates/migrations/src/source.rs 240
max_lines crates/migrations/src/service.rs 190
max_lines crates/migrations/src/database.rs 150
max_lines crates/migrations/src/locking.rs 120
max_lines crates/migrations/src/history.rs 80
max_lines crates/migrations/src/types.rs 80
max_lines crates/migrations/src/error.rs 80
max_lines crates/integrations/src/lib.rs 80
max_lines crates/integrations/src/egress.rs 300
max_lines crates/integrations/src/secrets.rs 160
max_lines crates/integrations/src/https_client.rs 340
max_lines crates/integrations/src/error.rs 80
max_lines crates/storage/src/lib.rs 80
max_lines crates/storage/src/filesystem.rs 450
max_lines crates/storage/src/upload.rs 240
max_lines crates/storage/src/image.rs 180
max_lines crates/observability/src/lib.rs 80
max_lines crates/observability/src/events.rs 180
max_lines crates/observability/src/metrics.rs 360
max_lines crates/observability/src/logging.rs 320
max_lines crates/observability/src/error.rs 80

# Stable façade surfaces created by the completed auth extractions.
grep -q '^mod session;$' crates/auth/src/lib.rs || fail 'auth façade must own session as a private module'
grep -q '^mod local_user;$' crates/auth/src/lib.rs || fail 'auth façade must own local_user as a private module'
grep -q '^pub use session::{RedisSessionStore, SessionBackend, SessionFlash, SessionSnapshot, SessionStore};$' crates/auth/src/lib.rs \
  || fail 'auth façade must preserve the session public surface through explicit re-exports'
grep -q '^pub use local_user::{LocalUserAuth, LocalUserStore};$' crates/auth/src/lib.rs \
  || fail 'auth façade must preserve the local-user public surface through explicit re-exports'

printf '%s\n' 'architecture verification passed'

# R48 module resolution stays a small compiler-owned boundary.
max_lines crates/compiler/src/source_loader.rs 260
max_lines crates/compiler/src/module_namespace.rs 80
