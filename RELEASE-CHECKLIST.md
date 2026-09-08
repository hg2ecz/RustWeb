# RWLang V1 release checklist

## Automated workspace gate

- [ ] `./tools/check-architecture.sh` passes
- [ ] `./tools/check-clean-structure.sh` passes
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo metadata --locked --format-version 1`
- [ ] `cargo check --locked --workspace`
- [ ] `cargo test --locked --workspace`
- [ ] `./tools/supply-chain-verify.sh` passes and `RW-SUPPLY-CHAIN.lock` is current
- [ ] security negative fixtures pass
- [ ] SQLite CRUD integration passes
- [ ] migration `verify` passes before and after `apply`
- [ ] local-auth administration smoke tests pass
- [ ] source examples compile through `rwlang-cli check`
- [ ] `RELEASE-MANIFEST.sha256` verifies successfully
- [ ] deterministic source package is produced from the exact release tree

## Environment-specific release evidence

`./verify.sh` is the repository-level automated gate. The following checks require real infrastructure or the target Linux/operator environment and must be recorded separately with date, environment and artifact hash:

- [ ] PostgreSQL integration
- [ ] MariaDB integration
- [ ] Redis session and TOTP replay integration against real Redis
- [ ] public HTTPS certificate and hostname behavior
- [ ] outbound DNS/CIDR/peer checks in a real network environment, including mixed A/AAAA resolution
- [ ] AppFs/openat2 behavior on the target Linux kernel
- [ ] cgroup/systemd resource ceilings on the target host
- [ ] slowloris/request-smuggling corpus against the target proxy/load-balancer topology
- [ ] load smoke test under approved resource limits
- [ ] no secrets, raw credentials, tokens, request bodies, or raw principal/source identifiers appear in security logs
- [ ] authentication-abuse and security-alert thresholds are exercised against the target Redis/logging topology
- [ ] systemd/container hardening profile is reviewed
- [ ] backup restore drill and upgrade/rollback rehearsal in an isolated production-like environment

## Deployment gate

- [ ] application source passes `rwlang-cli check`
- [ ] production config passes `rwlang-server --check-config` in the target environment
- [ ] application `production { ... }` policy matches the effective HTTPS/HSTS/database-TLS/origin/cookie topology
- [ ] named outbound integration targets resolve to reviewed host/port/CIDR/TLS policy
- [ ] release directory is immutable/read-only to the service
- [ ] release artifact SHA-256 and migration set are recorded
- [ ] migration credential is unavailable to the application service
- [ ] backup exists and restore readiness is confirmed before migration apply
- [ ] service-manager hard ceilings match approved operator policy
- [ ] rollout completes only after liveness and readiness succeed
- [ ] startup logs, structured security events, burst alerts, audit events and resource-profile startup audit are reviewed
- [ ] critical/idempotent transaction paths have explicit `Committed` / `RolledBack` / `CommitUnknown` handling

## Recovery and upgrade gate

- [ ] production state inventory covers application DB, local-auth DB, AppFs data root and Redis role
- [ ] RPO/RTO, retention, encryption and access policy are documented
- [ ] consistent DB + AppFs recovery-point strategy is documented
- [ ] local-auth DB is backed up separately as sensitive authentication material when enabled
- [ ] release/source/migration/config hashes are recorded without secret values
- [ ] latest backup has a successful isolated restore-drill record
- [ ] restore drill passes config check, source check, `migrate verify`, live/ready and application smoke tests
- [ ] schema compatibility is classified before rollout
- [ ] app-only rollback is used only with a backward-compatible schema
- [ ] no automatic reverse/down migration is part of the rollback path
- [ ] destructive restore has an explicit recovery point and accepted data-loss window
- [ ] Redis loss behavior is accepted: session reset, cache rebuild and rate-limit window reset

## IPv6 egress evidence

- [ ] mixed A/AAAA resolution is exercised
- [ ] one denied DNS candidate rejects the whole outbound request
- [ ] IPv6 connected peer is rechecked against configured CIDRs
- [ ] IPv4-mapped IPv6 follows IPv4 CIDR policy
- [ ] TLS hostname verification succeeds over an allowed IPv6 peer

## Final packaging gate

- [ ] `Cargo.lock` is present in the exact release workspace and reviewed
- [ ] `SUPPLY-CHAIN-CAPABILITIES.txt` is reviewed for every direct external dependency edge
- [ ] `RW-SUPPLY-CHAIN.lock` seals the exact `Cargo.lock` + capability policy state
- [ ] `./verify.sh` does not generate or modify `Cargo.lock`
- [ ] `cargo metadata --locked`, `cargo check --locked --workspace`, and `cargo test --locked --workspace` pass
- [ ] `RELEASE-MANIFEST.sha256` is generated from the exact release source tree
- [ ] `sha256sum -c RELEASE-MANIFEST.sha256` passes before packaging
- [ ] `tools/package-release.sh` produces the deterministic source artifact
- [ ] source artifact SHA-256 is recorded with release evidence
- [ ] `RELEASE-NOTES-V1.0.md` is published with the artifact
- [ ] no post-freeze feature change is included without explicit release-blocker justification
