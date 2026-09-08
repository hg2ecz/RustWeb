# RWLang security status

RWLang is a security-oriented web programming language/runtime. Its security model follows one rule consistently:

> If a security property can be proven statically, make it a compiler invariant. If it cannot, enforce a secure runtime/platform default. If neither is possible, fail closed with a stable security diagnostic.

This document is the canonical high-level status of the implemented security model. Detailed language/runtime behavior lives in `docs/`.

## Current security architecture

### Access control and authority

Implemented:

- every route has an explicit access policy; there is no implicit public route;
- object authorization produces flow-sensitive compiler evidence;
- protected UPDATE/DELETE operations require mutation authorization evidence;
- named function permissions and MFA elevation are compiler/runtime contracts;
- critical operations compose permission, MFA, transaction, audit and idempotency requirements;
- tenant membership is platform-owned authority and scoped-model queries require active-tenant proof;
- effect/capability metadata prevents ambient DB and outbound-network authority;
- named outbound integrations do not expose arbitrary host/IP/port authority to RWLang source.

### Input, injection and browser boundaries

Implemented:

- typed SQL bind contracts; raw/interpolated SQL is rejected;
- structured HTML/template output and sensitivity-aware rendering;
- compiler-checked typed route redirects rather than string-built URLs;
- nominal/domain types with explicit validation/refinement;
- bounded request strings and bounded request collections;
- staged file uploads with byte-authoritative image inspection before publication;
- verified webhook routes with raw-body signature verification and replay protection;
- typed HTTP response metadata, media types, filenames and Content-Disposition;
- generated least-privilege CSP and platform-owned browser security headers;
- compressed inbound HTTP bodies are rejected unless a future bounded decompression API explicitly supports them.

### Secrets, credentials and cryptography

Implemented:

- `Secret<T>` and `Sensitive<T>` information-flow restrictions;
- explicit public projections; domain/database models are not generically serialized;
- purpose-specific password/token/session/CSRF/key types;
- Argon2id password hashing via platform policy;
- token hash-at-rest, expiry-aware verification, consume-once reset grants, session revocation and rotation proofs;
- purpose/lifecycle-specific signing, verification and encryption key types;
- authenticated user-data encryption using fixed AES-256-GCM, platform-generated nonces and a versioned authenticated envelope;
- active encryption keys can encrypt/decrypt, retiring keys decrypt only, retired keys cannot be used;
- `Redacted<T>` values can only be produced by the trusted `redact(...)` builtin rather than type annotation/casting.

### Integrity and exceptional conditions

Implemented:

- idempotent critical operations bind replay keys to request fingerprints and principal scope;
- webhook replay protection is separate from browser/client idempotency authority;
- first-class closed sum types use exhaustive `match` without wildcard fallthrough;
- transaction outcomes distinguish `Committed`, `RolledBack` and `CommitUnknown`;
- idempotent critical transactions must capture and exhaustively handle transaction outcome;
- public application errors are a closed safe set; internal/database/resource errors remain platform-owned;
- application/runtime failures fail closed rather than selecting permissive fallback behavior.

### Resource and availability safety

Implemented:

- route resource profiles under a platform hard ceiling;
- instruction/allocation budgets and a hard request deadline;
- database list queries require a static bounded row limit and runtime row caps;
- cumulative external-I/O accounting for request fields and outbound traffic;
- outbound response-body hard limits and strict transfer/content-encoding policy;
- file byte/pixel limits and staged processing;
- bounded string/list operations and bounded request collection cardinality.

General unrestricted `List<T>` / `Dict<K,V>` collection growth is intentionally not exposed until the same bounded-resource guarantees can be preserved.

### Authentication abuse resistance

Implemented:

- source+principal login attempt limits;
- per-principal limits across sources;
- per-source limits across principals;
- stricter MFA/recovery stage limits;
- fail-closed shared-limiter-store behavior;
- generic credential failure behavior to reduce account enumeration;
- successful login does not erase source-wide/principal-wide abuse history.

### Security monitoring

Implemented:

- typed application security events and mandatory audit for critical operations;
- platform security events for authentication abuse, MFA failures, policy denials, CSRF/origin/CORS failures, webhook verification, idempotency conflicts and resource exhaustion;
- security-event schema does not accept arbitrary request payload/message fields;
- principal/source values are redacted in emitted security events;
- bounded in-memory correlation can use raw keys without writing them to logs;
- platform-owned burst thresholds and alert cooldowns prevent both silent attacks and alert floods.

### Deployment and supply chain

Implemented:

- typechecked `production { ... }` security requirements;
- production startup rejects insecure HTTPS/cookie/source-reload/origin/database-TLS combinations;
- source reload cannot silently change the deployment security contract;
- exact `Cargo.lock` plus explicit direct-dependency capability inventory;
- checksummed crates.io-only external provenance in the current policy;
- `RW-SUPPLY-CHAIN.lock` seals Cargo lock + capability policy state;
- release/verification scripts require Cargo lock/manifests consistency before sealing or packaging;
- release manifest hashes the exact source tree.

## High-level OWASP Top 10:2025 posture

This is risk coverage, not a compliance claim.

| Area | RWLang status |
| --- | --- |
| Broken Access Control | Strong compiler/runtime enforcement: explicit route access, authorization/mutation proofs, permissions/MFA, tenant isolation, capabilities |
| Security Misconfiguration | Strong production policy, CSP/security headers, secure sessions, egress and startup fail-closed checks |
| Software Supply Chain Failures | Strong foundation: exact lock, capability inventory, provenance restrictions and sealed supply-chain state |
| Cryptographic Failures | Strong: typed credential/token/key purposes, Argon2id, hash-at-rest, lifecycle-aware keys and AEAD user-data encryption |
| Injection | Very strong: SQL/HTML/redirect/header/filename/outbound URL authority are typed or structurally constrained |
| Insecure Design | Strong: critical-operation contracts, idempotency, explicit exceptional states, capabilities and resource invariants |
| Authentication Failures | Strong: platform sessions, rotation/revocation, MFA, typed credentials and multi-dimensional abuse protection |
| Software/Data Integrity Failures | Strong: webhook verification/replay protection, idempotency, AEAD and supply-chain provenance |
| Logging and Alerting Failures | Strong core: typed events, mandatory critical audit, redaction, platform auto-events and burst alerts |
| Mishandling of Exceptional Conditions | Strong: closed public errors, exhaustive sum types, transaction outcome states, deadlines and bounded resources |

## Deliberate remaining non-core work

The seven concentrated hardening iterations are complete. Remaining work should be justified by a concrete threat model rather than by control-count or checklist coverage.

Potential later work:

- controlled `unsafe` boundary with production deny policy, once a real secure-surface exception is needed;
- `rw security --owasp/--asvs/--nist/--iso/--sarif` assurance reporting from compiler/runtime metadata;
- IDE proof/effect visibility and quick fixes;
- generated security architecture documentation;
- generic collections only together with preserved cardinality/resource invariants;
- job/queue payload safety only if a first-class job/queue subsystem is introduced.

## Merge and release evidence

A release must not be described as green unless the exact release tree passes the real toolchain gates:

```bash
cargo fmt --all -- --check
cargo metadata --locked --format-version 1 >/dev/null
cargo check --workspace --locked
cargo test --workspace --locked
./verify.sh
```

Repository architecture/clean-code gates are valuable but are not substitutes for Cargo compilation/tests.
