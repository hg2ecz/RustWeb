# RWLang — Rust Web Language

RWLang is a Rust-based language/runtime/server ecosystem specialized for web application development. The V1 scope focuses on secure-by-default behavior, typed input/output, compiler-enforced policies, explicit capabilities, auditability, and production operability.

## Getting started

For the canonical English documentation, start here:

1. [Documentation index](docs/README.md)
2. [Dependency security and reproducible builds](docs/19-dependency-security.md)
3. [Optimistic locking and concurrent edits](docs/24-optimistic-locking.md)
4. [IPv6-ready outbound egress](docs/32-ipv6-egress.md)

The production-oriented reference application is in `examples/starter-project/`.

Canonical documentation is English. Hungarian documentation is kept separately under [`docs/hu/`](docs/hu/) and the Hungarian project overview is [`README_hu.md`](README_hu.md). The canonical English book source is under [`docs/book/`](docs/book/), with the Hungarian translation under [`docs/book/hu/`](docs/book/hu/).

## Build and verification

The workspace uses Rust edition 2024. The repository intentionally does not pin a specific compiler version; reproducible dependency resolution is provided by the committed `Cargo.lock` and `--locked` builds.

```bash
./verify.sh
cargo build --locked --release -p rwlang-server -p rwlang-cli
```

Installed binaries are intended to live at:

```text
/usr/local/bin/rwlang-server
/usr/local/bin/rwlang-cli
```

The primary production configuration path is:

```text
/usr/local/etc/rwlang/server.toml
```

Start the server with:

```bash
rwlang-server --config /usr/local/etc/rwlang/server.toml
```

Preflight validation:

```bash
rwlang-server --config /usr/local/etc/rwlang/server.toml --check-config
```

Configuration precedence is:

```text
defaults < TOML config < targeted CLI override
```

Stable production policy belongs in trusted configuration rather than in a long command line. SIGHUP reopens logs and, in behind-proxy mode, transactionally reloads domain/application hosting state. Application source changes are also detected automatically by the shared source-reload supervisor; process-level settings still require restart. See `docs/39-automatic-source-reload.md` for the deployment and failure semantics.

## Debian package

On Debian/Ubuntu build hosts, `make deb` creates an installable `rwlang_1.0.0-1_<arch>.deb`. Debian-managed installs intentionally use `/usr/bin` and `/etc/rwlang` rather than `/usr/local`; see [the Debian packaging guide](docs/36-debian-package.md).

## V1 capability overview

- typed routing, forms, and JSON APIs;
- typed SQL bind/decode, migrations, and optimistic locking;
- local/LDAP authentication, TOTP, roles/permissions, and object authorization;
- first-class `Date`, `DateTime`, `Uuid`, `Decimal`, `Slug`, `Email`, `Url`, and enums;
- domain objects and modules;
- safe HTML, components/layouts, Markdown, and image/media handling;
- CSRF, Host/Origin/Fetch Metadata checks, HTTPS, and static/media confinement;
- AppFs and IPv4/IPv6 outbound-network policy;
- rate limiting, public cache, and resource profiles;
- structured server/access/audit logs and business audit trails;
- canonical slug redirects, PRG/flash, and 409 conflict UX;
- config-first deployment plus backup/restore/upgrade/rollback workflows.

## Deliberate V1 non-goals

The V1 core does not include full-text search, background jobs, email sending, a scheduler, revision history, a workflow engine, soft delete, generated admin CRUD, module visibility/export, S3 media, image resize/thumbnail generation, HTTP/2 or HTTP/3, SSE/WebSocket, an OpenTelemetry SDK, or private cache.

RWLang does not expose a general Rust-like `pub`/`mut` language surface in V1. Modules are application-root-relative namespaces: `mod foo;` loads exactly `foo.rw`, declarations stay under `foo::...`, and cross-module references must use that qualified path. Nested modules map canonically (`foo::bar` → `foo/bar.rw`); there is no `mod.rw` fallback, relative traversal, or automatic directory discovery. V1 has no separate public/private export layer. Compute code may explicitly reassign an existing local with `set name = expr` under static typing and the instruction budget. Persistent business-state mutation still goes through explicit query/transaction/action paths.

## Language syntax essentials

RWLang uses explicit statement boundaries. Simple statements end with `;`; newlines are whitespace only and there is no automatic semicolon insertion. Non-block top-level declarations such as `mod path;` and `route ... => handler;` also end with `;`, while block declarations and control-flow blocks end with `}` and do not take a trailing semicolon. See [`docs/56-statement-terminators.md`](docs/56-statement-terminators.md).

The numeric core includes checked `+`, `-`, `*`, `/`, `%`, integer shifts `<<`/`>>`, integer bitwise `&`/`^`/`|`, boolean `!`/`&&`/`||` with short-circuit evaluation, and F32 math builtins including `ln`, `log10`, `log`, `exp`, `pow`, `round`, `floor`, and `ceil`. See [`docs/44-math-and-timing.md`](docs/44-math-and-timing.md).

The Unicode-aware string core includes `trim`, `trimStart`, `trimEnd`, `lower`, `upper`, `stringLen`, `contains`, `startsWith`, `endsWith`, `replace`, `split`, `substring`, `indexOf`, `lastIndexOf`, `charAt`, and `repeat`, plus the regex API. See [`docs/46-string-builtins.md`](docs/46-string-builtins.md) and [`docs/49-regular-expressions.md`](docs/49-regular-expressions.md).

## Documentation language policy

English is the canonical project documentation language. Hungarian material must live under a `hu/` directory or use an explicit `_hu` suffix. See [the documentation index](docs/README.md) for the current layout.


Execution note: application expressions are executed by a bounded stack bytecode VM; native JIT compilation is not enabled yet. See `docs/40-bytecode-execution.md`.
