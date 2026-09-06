#!/bin/sh
set -eu

# V1 release integrity: the resolved dependency graph is part of the artifact.
# Release verification must never re-resolve dependencies implicitly.
if [ ! -f Cargo.lock ]; then
  echo 'release verification refused: Cargo.lock is missing' >&2
  echo 'generate and review Cargo.lock in the trusted developer workspace before packaging' >&2
  exit 1
fi
cargo metadata --locked --format-version 1 >/dev/null
cargo check --locked --workspace
cargo test --locked --workspace
# M14 integrations crate is covered by workspace tests.
cargo run --locked -q -p rwlang-cli -- check examples/hello/app.rw
cargo run --locked -q -p rwlang-cli -- check examples/database/app.rw
cargo run --locked -q -p rwlang-cli -- check examples/crud/app.rw
cargo run --locked -q -p rwlang-cli -- check examples/a-z-products/main.rw
if cargo run --locked -q -p rwlang-cli -- check examples/security/sql-injection-rejected.rw >/dev/null 2>&1; then
  echo 'expected sql-injection-rejected.rw to fail' >&2; exit 1
fi
if cargo run --locked -q -p rwlang-cli -- check examples/security/sql-bind-mismatch-rejected.rw >/dev/null 2>&1; then
  echo 'expected sql-bind-mismatch-rejected.rw to fail' >&2; exit 1
fi
if cargo run --locked -q -p rwlang-cli -- check examples/security/xss-script-rejected.rw >/dev/null 2>&1; then
  echo 'expected xss-script-rejected.rw to fail' >&2; exit 1
fi
if cargo run --locked -q -p rwlang-cli -- check examples/security/xss-attribute-rejected.rw >/dev/null 2>&1; then
  echo 'expected xss-attribute-rejected.rw to fail' >&2; exit 1
fi
if cargo run --locked -q -p rwlang-cli -- check examples/security/optional-without-check-rejected.rw >/dev/null 2>&1; then
  echo 'expected optional-without-check-rejected.rw to fail' >&2; exit 1
fi
if cargo run --locked -q -p rwlang-cli -- check examples/security/list-direct-interpolation-rejected.rw >/dev/null 2>&1; then
  echo 'expected list-direct-interpolation-rejected.rw to fail' >&2; exit 1
fi
if cargo run --locked -q -p rwlang-cli -- check examples/security/unsafe-dynamic-href-rejected.rw >/dev/null 2>&1; then
  echo 'expected unsafe-dynamic-href-rejected.rw to fail' >&2; exit 1
fi
if cargo run --locked -q -p rwlang-cli -- check examples/security/url-helper-type-rejected.rw >/dev/null 2>&1; then
  echo 'expected url-helper-type-rejected.rw to fail' >&2; exit 1
fi
if cargo run --locked -q -p rwlang-cli -- check examples/security/validation-schema-rejected.rw >/dev/null 2>&1; then
  echo 'expected validation-schema-rejected.rw to fail' >&2; exit 1
fi
if cargo run --locked -q -p rwlang-cli -- check examples/security/auth-mode-rejected.rw >/dev/null 2>&1; then
  echo 'expected auth-mode-rejected.rw to fail' >&2; exit 1
fi
cargo run --locked -q -p rwlang-cli -- check examples/auth/app.rw
cargo run --locked -q -p rwlang-cli -- check examples/upload/app.rw
cargo run --locked -q -p rwlang-cli -- check examples/resource-profile/app.rw
cargo run --locked -q -p rwlang-cli -- check examples/json-api/app.rw
cargo run --locked -q -p rwlang-cli -- check examples/rate-limit/app.rw
cargo run --locked -q -p rwlang-cli -- check examples/static-assets/app.rw
if cargo run --locked -q -p rwlang-cli -- check examples/security/upload-path-rejected.rw >/dev/null 2>&1; then
  echo 'expected upload-path-rejected.rw to fail' >&2; exit 1
fi
if cargo run --locked -q -p rwlang-cli -- check examples/security/upload-form-binding-rejected.rw >/dev/null 2>&1; then
  echo 'expected upload-form-binding-rejected.rw to fail' >&2; exit 1
fi
cargo test --locked -p storage
cargo test --locked -p rwlang-server resource_limits::tests
test -f crates/server/src/resource_limits.rs
printf '%s\n' 'M17 verification passed'

echo "checking M18 release artifacts"
test -f RELEASE-CHECKLIST.md
test -x examples/load/smoke.sh

if cargo run --locked -q -p rwlang-cli -- check examples/negative/json-form-mixed-rejected.rw >/dev/null 2>&1; then echo "expected compile failure: examples/negative/json-form-mixed-rejected.rw" >&2; exit 1; fi
if cargo run --locked -q -p rwlang-cli -- check examples/negative/json-upload-mixed-rejected.rw >/dev/null 2>&1; then echo "expected compile failure: examples/negative/json-upload-mixed-rejected.rw" >&2; exit 1; fi
printf '%s\n' 'M19 JSON/CORS verification passed'
printf '%s\n' 'M20 static asset verification passed'

echo "checking M21 lifecycle artifacts"
grep -q '^shutdown_grace_ms = 30000$' config/server.toml.sample
grep -q '^ExecStart=/usr/local/bin/rwlang-server --config /usr/local/etc/rwlang/server.toml$' examples/systemd/rwlang.service
grep -q 'TimeoutStopSec=45s' examples/systemd/rwlang.service


echo "checking M23 observability artifacts"
test -f docs/hu/09-observability.md
grep -q 'rwlang-server' docs/hu/33-cli-workflow.md
printf '%s\n' 'M23 observability verification passed'

echo "checking M24 migration workflow"
cargo test --locked -p migrations
tmp_m24="$(mktemp -d)"
trap 'rm -rf "$tmp_m24"' EXIT INT TERM
mkdir -p "$tmp_m24/migrations"
printf '%s\n' 'CREATE TABLE smoke(id BIGINT PRIMARY KEY);' > "$tmp_m24/migrations/0001_init.sql"
printf 'sqlite://%s/db.sqlite?mode=rwc\n' "$tmp_m24" > "$tmp_m24/db-url"
cargo run --locked -q -p rwlang-cli -- migrate status --dir "$tmp_m24/migrations" --db-url-file "$tmp_m24/db-url" >/dev/null
cargo run --locked -q -p rwlang-cli -- migrate apply --dir "$tmp_m24/migrations" --db-url-file "$tmp_m24/db-url" >/dev/null
cargo run --locked -q -p rwlang-cli -- migrate verify --dir "$tmp_m24/migrations" --db-url-file "$tmp_m24/db-url" >/dev/null
cargo run --locked -q -p rwlang-cli -- migrate status --dir "$tmp_m24/migrations" --db-url-file "$tmp_m24/db-url" | grep -q applied
rm -rf "$tmp_m24"
trap - EXIT INT TERM
test -f docs/hu/10-database-migrations.md
printf '%s\n' 'M24 migration verification passed'

echo "checking M25 local authentication"
cargo test --locked -p auth
m25_tmp="$(mktemp -d)"
trap 'rm -rf "$m25_tmp"' EXIT INT TERM
printf 'sqlite://%s/auth.db?mode=rwc\n' "$m25_tmp" > "$m25_tmp/auth-url"
printf '%s\n' 'correct horse battery staple 2026' > "$m25_tmp/password"
chmod 600 "$m25_tmp/auth-url" "$m25_tmp/password"
cargo run --locked -q -p rwlang-cli -- auth init --db-url-file "$m25_tmp/auth-url" >/dev/null
cargo run --locked -q -p rwlang-cli -- auth user-add --db-url-file "$m25_tmp/auth-url" --username alice --password-file "$m25_tmp/password" --role User --role Editor >/dev/null
cargo run --locked -q -p rwlang-cli -- auth roles-set --db-url-file "$m25_tmp/auth-url" --username alice --role User --role Publisher >/dev/null
cargo run --locked -q -p rwlang-cli -- auth totp-enroll --db-url-file "$m25_tmp/auth-url" --username alice --recovery-count 4 | grep -q 'Recovery codes'
cargo run --locked -q -p rwlang-cli -- auth disable --db-url-file "$m25_tmp/auth-url" --username alice >/dev/null
cargo run --locked -q -p rwlang-cli -- auth enable --db-url-file "$m25_tmp/auth-url" --username alice >/dev/null
cargo run --locked -q -p rwlang-cli -- auth password-set --db-url-file "$m25_tmp/auth-url" --username alice --password-file "$m25_tmp/password" >/dev/null
rm -rf "$m25_tmp"
trap - EXIT INT TERM
printf '%s\n' 'M25 local authentication verification passed'

echo "checking M26 business scalar types"
cargo run --locked -q -p rwlang-cli -- check examples/business-types/app.rw
cargo test --locked -p compiler m26_type_compile_tests
cargo test --locked -p runtime m26_value_type_tests
test -f docs/hu/11-business-types.md
printf '%s\n' 'M26 business scalar verification passed'

echo "checking M27 reusable forms"
cargo run --locked -q -p rwlang-cli -- check examples/forms/app.rw
if cargo run --locked -q -p rwlang-cli -- check examples/negative/form-schema-bad-validation-rejected.rw >/dev/null 2>&1; then
  echo 'expected form-schema-bad-validation-rejected.rw to fail' >&2; exit 1
fi
cargo test --locked -p compiler m27_form_schema_compile_tests
cargo test --locked -p runtime m27_form_runtime_tests
test -f docs/hu/12-forms.md
printf '%s\n' 'M27 reusable form verification passed'

echo "checking M28 public cache"
cargo run --locked -q -p rwlang-cli -- check examples/cache/app.rw
if cargo run --locked -q -p rwlang-cli -- check examples/negative/cache-user-dependent-rejected.rw >/dev/null 2>&1; then
  echo 'expected cache-user-dependent-rejected.rw to fail' >&2; exit 1
fi
cargo test --locked -p compiler public_cache_compile_tests
test -f docs/hu/13-cache.md
printf '%s\n' 'M28 public cache verification passed'

echo "checking M29 components and layouts"
cargo run --locked -q -p rwlang-cli -- check examples/components/app.rw
cargo test --locked -p compiler m29_component_layout_compile_tests
cargo test --locked -p runtime m29_component_layout_runtime_tests
test -f docs/hu/14-components-layouts.md
printf '%s\n' 'M29 component/layout verification passed'

echo "checking M30 config artifacts"
test -f config/server.toml.sample
grep -q '^\[server\]' config/server.toml.sample
grep -q '^\[database\]' config/server.toml.sample
grep -q 'url_file' config/server.toml.sample

echo "checking M31 structured logging and audit"
grep -q '^\[logging\]' config/server.toml.sample
grep -q 'audit_file' config/server.toml.sample
grep -q 'ExecReload=.*/kill -HUP' examples/systemd/rwlang.service
test -f examples/logrotate/rwlang
printf '%s\n' 'M31 logging/audit verification passed'

echo "checking M32 safe Markdown"
cargo run --locked -q -p rwlang-cli -- check examples/markdown/app.rw
cargo test --locked -p compiler m32_markdown_compiler_tests
cargo test --locked -p runtime m32_markdown_runtime_tests
test -f docs/hu/16-markdown-rich-text.md
printf '%s\n' 'M32 safe Markdown verification passed'

echo "checking M33 typed media/images"
cargo run --locked -q -p rwlang-cli -- check examples/media/app.rw
if cargo run --locked -q -p rwlang-cli -- check examples/negative/image-direct-interpolation-rejected.rw >/dev/null 2>&1; then
  echo 'expected image-direct-interpolation-rejected.rw to fail' >&2; exit 1
fi
cargo test --locked -p compiler m33_image_compiler_tests
cargo test --locked -p language-core m33_image_ref_tests
cargo test --locked -p storage image_tests
test -f docs/hu/17-media-library.md
grep -q 'max_image_pixels = 40000000' config/server.toml.sample
printf '%s\n' 'M33 typed media verification passed'

echo "checking M34 dependency hygiene"
test -f Cargo.lock
test ! -f rust-toolchain.toml
for manifest in crates/*/Cargo.toml; do
  grep -q 'edition = "2024"' "$manifest"
  if grep -q '^rust-version = ' "$manifest"; then echo "unexpected compiler version pin in $manifest" >&2; exit 1; fi
done
test -x tools/refresh-lock.sh
test -x tools/dependency-audit.sh
test -f docs/19-dependency-security.md
printf '%s\n' 'M34 dependency hygiene verification passed'

echo "checking M35 object authorization"
cargo run --locked -q -p rwlang-cli -- check examples/object-authorization/app.rw
if cargo run --locked -q -p rwlang-cli -- check examples/negative/object-auth-public-rejected.rw >/dev/null 2>&1; then
  echo 'expected object-auth-public-rejected.rw to fail' >&2; exit 1
fi
cargo test --locked -p compiler m35_object_authorization_compile_tests
cargo test --locked -p runtime m35_object_authorization_runtime_tests
test -f docs/hu/20-object-authorization.md
printf '%s\n' 'M35 object authorization verification passed'

echo "checking M36 modules, slug and developer workflow"
cargo run --locked -q -p rwlang-cli -- check examples/wiki/main.rw
cargo run --locked -q -p rwlang-cli -- check examples/news-site/main.rw
cargo test --locked -p compiler m36_modules_slug_compile_tests
cargo test --locked -p runtime m36_slug_runtime_tests
test -f docs/hu/21-modules-slugs-project-layout.md
grep -q '^# RWLang — Rust Web Language' README.md
grep -q 'mod models;' examples/news-site/main.rw
grep -q ':slug<Slug>' examples/news-site/pages.rw
printf '%s\n' 'M36 module/slug/developer workflow verification passed'

echo "checking M37 domain objects and namespaces"
cargo run --locked -q -p rwlang-cli -- check examples/domain-objects/main.rw
cargo test --locked -p compiler m37_domain_object_compile_tests
test -f docs/hu/22-domain-objects.md
grep -q '^object Article' examples/domain-objects/main.rw
grep -q 'Article.bySlug' examples/domain-objects/main.rw
printf '%s\n' 'M37 domain object verification passed'

echo "checking M38 first-class enums"
cargo run --locked -q -p rwlang-cli -- check examples/enums/main.rw
cargo test --locked -p compiler m38_enum_compile_tests
cargo test --locked -p runtime m38_enum_runtime_tests
test -f docs/hu/23-enums.md
grep -q '^enum ArticleStatus' examples/enums/main.rw
grep -q 'ArticleStatus.Published' examples/enums/main.rw
printf '%s\n' 'M38 enum verification passed'


echo "checking M39 optimistic locking / conflict contract"
cargo run --locked -q -p rwlang-cli -- check examples/optimistic-locking/main.rw
cargo test --locked -p compiler m39_optimistic_lock_compile_tests
cargo test --locked -p runtime m39_optimistic_lock_runtime_tests
test -f docs/24-optimistic-locking.md
printf '%s\n' 'M39 optimistic locking verification passed'

echo "checking M40 transactional business audit trail"
cargo run --locked -q -p rwlang-cli -- check examples/business-audit/main.rw
cargo test --locked -p compiler m40_business_audit_compile_tests
cargo test --locked -p runtime m40_business_audit_runtime_tests
test -f docs/hu/25-business-audit-trail.md
test -f examples/business-audit/migrations/0001_business_audit.sql
grep -q 'audit Article id action publish' examples/business-audit/main.rw
grep -q '_rw_business_audit' examples/business-audit/migrations/0001_business_audit.sql
printf '%s\n' 'M40 business audit verification passed'


echo "checking M41 domain validation and Email"
cargo run --locked -q -p rwlang-cli -- check examples/domain-validation/main.rw
cargo test --locked -p compiler m41_domain_validation_compile_tests
cargo test --locked -p runtime m41_domain_validation_runtime_tests
test -f docs/hu/26-domain-validation.md
grep -q 'email<Email>' examples/domain-validation/main.rw
grep -q 'validate confirmEmail same email' examples/domain-validation/main.rw
printf '%s\n' 'M41 domain validation verification passed'

echo "checking M42 domain validation II and DB conflict mapping"
cargo run --locked -q -p rwlang-cli -- check examples/domain-validation-m42/main.rw
cargo test --locked -p compiler m42_domain_validation_compile_tests
cargo test --locked -p runtime m42_domain_validation_runtime_tests
test -f docs/hu/26-domain-validation.md
printf '%s\n' 'M42 domain validation II verification passed'

echo "checking M43 canonical URL"
cargo run --locked -q -p rwlang-cli -- check examples/canonical-url/main.rw
cargo test --locked -p compiler m43_canonical_url_compile_tests
cargo test --locked -p runtime m43_canonical_url_runtime_tests
test -f docs/hu/27-canonical-url.md
printf '%s\n' 'M43 canonical URL verification passed'

echo "checking M44 PRG, flash and conflict UX"
cargo run --locked -q -p rwlang-cli -- check examples/prg-flash/app.rw
cargo test --locked -p compiler m44_prg_flash_compile_tests
cargo test --locked -p runtime m44_flash_runtime_tests
cargo test --locked -p auth m44_flash_session_tests
cargo test --locked -p rwlang-server m44_conflict_ux_tests
test -f docs/hu/28-prg-flash-conflict.md
grep -q 'flash success "Article saved"' examples/prg-flash/app.rw
grep -q '@flash()' examples/prg-flash/app.rw
printf '%s\n' 'M44 PRG/flash/conflict verification passed'

echo "checking M45 starter project and production deployment workflow"
cargo run --locked -q -p rwlang-cli -- check examples/starter-project/main.rw
test -f examples/starter-project/migrations/0001_create_articles.sql
test -f examples/starter-project/deploy/server.toml
test -f examples/starter-project/deploy/rwlang.service
test -x examples/starter-project/deploy/preflight.sh
sh -n examples/starter-project/deploy/preflight.sh
grep -q '^ExecStartPre=/usr/local/bin/rwlang-server --config /usr/local/etc/rwlang/server.toml --check-config$' examples/systemd/rwlang.service
grep -q '^ExecStart=/usr/local/bin/rwlang-server --config /usr/local/etc/rwlang/server.toml$' examples/systemd/rwlang.service
if grep -q -- '--tls-cert-file\|--max-instructions\|--max-runtime-alloc-bytes' examples/systemd/rwlang.service; then
  echo 'M45 systemd sample must remain config-first' >&2; exit 1
fi
test -f docs/hu/29-production-deployment.md
grep -q 'immutable release' docs/hu/29-production-deployment.md
grep -q 'migrate verify' examples/starter-project/deploy/preflight.sh
if grep -q 'migrate apply' examples/starter-project/deploy/preflight.sh; then
  echo 'M45 preflight must not mutate the database' >&2; exit 1
fi
printf '%s\n' 'M45 starter/deployment verification passed'

echo "checking M46 backup/restore/upgrade/rollback operator contract"
test -f docs/hu/30-backup-restore-upgrade-rollback.md
test -f examples/starter-project/deploy/backup-manifest.example
test -x examples/starter-project/deploy/release-record.sh
test -x examples/starter-project/deploy/restore-verify.sh
sh -n examples/starter-project/deploy/release-record.sh
sh -n examples/starter-project/deploy/restore-verify.sh
grep -q 'controlled maintenance/offline backup' docs/hu/30-backup-restore-upgrade-rollback.md
grep -q 'down migration' docs/hu/30-backup-restore-upgrade-rollback.md
grep -q 'nem üzleti source of truth' docs/hu/30-backup-restore-upgrade-rollback.md
if grep -q 'migrate apply' examples/starter-project/deploy/restore-verify.sh; then
  echo 'M46 restore verification must not mutate the database' >&2; exit 1
fi
grep -q 'restore verification refuses the documented production paths' examples/starter-project/deploy/restore-verify.sh
printf '%s\n' 'M46 recovery/operator contract verification passed'

echo "checking M46.1 clean-code structural refactor"
test -x tools/check-clean-structure.sh
sh -n tools/check-clean-structure.sh
./tools/check-clean-structure.sh
printf '%s\n' 'M46.1 clean-code structural verification passed'

echo "checking M46.2 clean-code responsibility extraction"
test -f crates/server/src/startup.rs
test -f crates/server/src/http_dispatch.rs
test -f crates/runtime/src/templates.rs
sh -n tools/check-clean-structure.sh
./tools/check-clean-structure.sh
printf '%s\n' 'M46.2 clean-code structural verification passed'

echo "checking M46.3 clean-code CLI responsibility extraction"
test -f crates/server/src/cli_config_apply.rs
sh -n tools/check-clean-structure.sh
./tools/check-clean-structure.sh
printf '%s\n' 'M46.3 clean-code CLI responsibility extraction passed'

echo "checking M47 V1 documentation/release audit"
test -f docs/hu/31-v1-developer-guide.md
grep -q 'nincs `pub`/export visibility rendszer' docs/hu/31-v1-developer-guide.md
grep -q 'Általános `mut` local/self state nincs' docs/hu/31-v1-developer-guide.md
grep -q 'rwlang-server' docs/hu/33-cli-workflow.md
grep -q 'rwlang-server --config /usr/local/etc/rwlang/server.toml' README.md
if grep -q '^--tls-cert-file ' docs/hu/08-https-security.md; then
  echo 'M47 HTTPS docs must stay config-first' >&2; exit 1
fi
printf '%s\n' 'M47 V1 documentation/release audit verification passed'


echo "checking M48 V1 release-candidate audit"
grep -q '^shutdown_grace_ms = 30000$' config/server.toml.sample
if grep -q -- '--shutdown-grace-ms' docs/hu/07-lifecycle-health.md docs/hu/18-uzemeltetes.md; then
  echo 'M48 production lifecycle docs must be config-first' >&2; exit 1
fi
if grep -q -- '--public-host\|--resource-profiles-file\|--cors-origin\|--static-root\|--allow-memory-rate-limit\|--allow-memory-cache\|--metrics-listen' docs/hu/18-uzemeltetes.md; then
  echo 'M48 operator guide must use TOML policy keys, not production CLI flag lists' >&2; exit 1
fi
grep -q 'Environment-specific release evidence' RELEASE-CHECKLIST.md
printf '%s\n' 'M48 V1 release-candidate audit verification passed'

echo "checking M49 IPv6 egress hardening and media fixture"
grep -q '^    id: Uuid$' examples/media/app.rw
grep -q '^    title: String$' examples/media/app.rw
grep -q '^    hero: Image$' examples/media/app.rw
if grep -q '^    id<Uuid>$' examples/media/app.rw; then
  echo 'M49 media example must use current model field syntax' >&2; exit 1
fi
grep -q '2001:db8:1200::/48' examples/egress/policy.toml
grep -q 'fn ipv4_mapped_ipv6_uses_ipv4_policy' crates/integrations/src/lib.rs
grep -q 'to_ipv4_mapped' crates/integrations/src/lib.rs
grep -q 'ip_allowed(&target.cidrs, addr.ip())' crates/integrations/src/lib.rs
grep -q 'ip_allowed(&target.cidrs, peer.ip())' crates/integrations/src/lib.rs
test -f docs/32-ipv6-egress.md
grep -q 'IPv4-mapped IPv6' docs/32-ipv6-egress.md
grep -q 'mixed A/AAAA' RELEASE-CHECKLIST.md
printf '%s\n' 'M49 IPv6 egress/media fixture verification passed'

echo "checking M50 positive-example compatibility gate"
test -x tools/check-positive-examples.sh
sh -n tools/check-positive-examples.sh
./tools/check-positive-examples.sh
grep -q 'query fn publishChanged' examples/business-audit/main.rw
grep -q 'Article.publishChanged(tx, id, version)?' examples/business-audit/main.rw
printf '%s\n' 'M50 positive-example compatibility verification passed'


echo "checking M51 final release packaging contract"
test -f RELEASE-NOTES-V1.0.md
test -x tools/release-manifest.sh
test -x tools/package-release.sh
sh -n tools/release-manifest.sh
sh -n tools/package-release.sh
grep -q 'release verification refused: Cargo.lock is missing' verify.sh
if grep -q 'cargo generate''-lockfile' verify.sh; then
  echo 'M51 verify.sh must not generate Cargo.lock during release verification' >&2; exit 1
fi
grep -q 'final source release must include `Cargo.lock`' RELEASE-NOTES-V1.0.md
printf '%s\n' 'M51 final release packaging verification passed'
