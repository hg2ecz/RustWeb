# RWLang documentation

English is the canonical documentation language for the RWLang repository.

## Language layout

- Canonical English documentation lives directly under `docs/`.
- Hungarian translations and legacy Hungarian documentation live under `docs/hu/`.
- The canonical English LaTeX book lives under `docs/book/`; the Hungarian translation lives under `docs/book/hu/`.
- A document outside a `hu/` directory should be English unless its filename has an explicit `_hu` suffix.

This convention also applies to READMEs in examples and other subdirectories.

## Core language reference

- [Modules and namespaces](21-modules-and-namespaces.md) - application-root-relative `::` namespaces and explicit cross-module references.
- [Numeric operators, F32 math and monotonic timing](44-math-and-timing.md) - checked arithmetic, `%`, shifts, bitwise/logical operators, short-circuit logic, logarithms, exponentials and rounding.
- [Typed string builtins](46-string-builtins.md) - Unicode-aware trimming, case conversion, search, `replace`, `split`, substring/index operations, character access and repetition.
- [Regular expressions](49-regular-expressions.md) - typed regex matching, replacement and captures.
- [Statement terminators](56-statement-terminators.md) - explicit `;`, no automatic semicolon insertion, and the `mod`/`route` top-level rule.

## Current canonical English documents

- [Release checklist](../RELEASE-CHECKLIST.md)
- [V1 release notes](../RELEASE-NOTES-V1.0.md)
- [Dependency security and reproducible builds](19-dependency-security.md)
- [Modules and namespaces](21-modules-and-namespaces.md)
- [Optimistic locking and concurrent edits](24-optimistic-locking.md)
- [IPv6-ready outbound egress](32-ipv6-egress.md)
- [Debian package build and dpkg installation](36-debian-package.md)
- [Multi-domain hosting](37-multi-domain-hosting.md)
- [Reverse-proxy application-server mode](38-reverse-proxy-application-server.md)
- [Automatic application source reload](39-automatic-source-reload.md)

The larger developer/operator documentation set currently has a Hungarian edition under [`hu/`](hu/). New or revised canonical documentation should be written in English first; Hungarian versions should be maintained as translations.

## Source examples as compatibility surface

Positive RWLang examples are enumerated in `examples/positive-entrypoints.txt` and are treated as part of the language compatibility surface. The dedicated `examples/module-namespaces/` application demonstrates the canonical R48 module rules: application-root-relative resolution, explicit source-graph membership, nested `::` namespaces, and qualified cross-module references. Negative and security fixtures are maintained separately as rejection tests.

## Book

The canonical long-form English book lives under [`book/`](book/). The Hungarian translation is isolated under [`book/hu/`](book/hu/). Use `make book`, `make book-hu`, or `make books` from the repository root.

## Repository documentation rule

When adding a document:

1. write the canonical version in English;
2. keep it at the normal project path;
3. if a Hungarian translation is needed, put it under the nearest `hu/` directory or use `_hu.md` when a directory split would be awkward;
4. do not mix Hungarian prose into the canonical English document except for quoted user-facing examples where the language is itself relevant.


- [Bytecode expression execution](40-bytecode-execution.md)

- [F32 numeric values](41-f32-numerics.md)

- [F32 arrays](42-f32-arrays.md)

- [Budgeted while loops and local assignment](43-budgeted-while.md)

- [F32 math and monotonic timing](44-math-and-timing.md)

- [Compute if, explicit F32 conversion, and FFT4096](45-if-conversion-and-fft.md)

- [Typed string builtins](46-string-builtins.md)


- [Typed String lists](47-string-lists.md)

- [Typed String dictionaries](48-string-dictionaries.md)

- [Regular expressions](49-regular-expressions.md)

- [Maintainability and clean-code boundaries](50-maintainability-and-clean-code.md)

- [Typed errors and explicit module dependencies](51-typed-errors-and-explicit-dependencies.md)

- [52. Typed server configuration and TLS errors](52-typed-server-boundaries.md)

- [53. Typed authentication, policy, resource-profile, and CLI errors](53-typed-auth-policy-and-cli-errors.md)

- [54. Typed runtime and cache boundaries](54-typed-runtime-boundaries.md)

- [55. Typed backend, reload, and HTTP boundaries](55-typed-backend-reload-and-http-boundaries.md)
- [Statement terminators](56-statement-terminators.md) - explicit `;`, no automatic semicolon insertion.
