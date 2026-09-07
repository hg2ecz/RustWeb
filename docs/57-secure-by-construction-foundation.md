# Secure-by-construction language foundation

RWLang treats common web-security requirements as language semantics wherever they can be enforced reliably. The design goal is not to make developers memorize a security checklist. The safe path should be the shortest path.

This change introduces the first breaking secure-by-construction language layer.

## 1. Route access is never implicit

Every route must now state its access policy explicitly:

```rwlang
route home GET "/" public => home;
route account GET "/account" auth user => account;
route billing POST "/billing" auth mfa => billing;
route admin GET "/admin" auth role Admin => admin;
```

A route without `public` or `auth ...` is rejected with `SEC-A01-001`.

This removes ambient public exposure. Adding a new endpoint always requires an explicit security decision, while the syntax remains one short token for the common public case.

## 2. Redirects are local and compiler-owned by default

Action redirects accept only safe local string literals:

```rwlang
return Ok(redirect("/account"));
```

These are rejected:

```rwlang
return Ok(redirect(target));
return Ok(redirect("//example.org"));
return Ok(redirect("https://example.org"));
```

Dynamic string redirects fail with `SEC-A01-003`; unsafe literal redirects fail with `SEC-A01-002`.

The compiler accepts a local path only when it starts with exactly one `/`, contains no backslash, NUL, CR, or LF, and cannot be interpreted as a protocol-relative URL.

Future typed route-target syntax can make dynamic internal redirects ergonomic without reopening string-based redirect vulnerabilities.

## 3. External raw strings are automatically bounded

Every route-supplied `String` from query, form, or JSON input receives an implicit maximum length of 4096 when no explicit `length` rule exists.

This:

```rwlang
route search GET "/search" query q<String> public => search;
```

is compiled as if the route also contained:

```rwlang
validate q length 0 4096
```

Developers can narrow the contract normally:

```rwlang
route search GET "/search"
    query q<String>
    validate q length 1 120
    public => search;
```

An explicit length rule replaces the default rather than adding a duplicate check.

This gives every raw textual request value a language-level resource bound without forcing repetitive validation boilerplate.

## 4. Security diagnostics

Compiler security errors now have stable identifiers. The first set is:

| Code | Meaning |
| --- | --- |
| `SEC-A01-001` | route has no explicit access policy |
| `SEC-A01-002` | redirect literal is not a safe local path |
| `SEC-A01-003` | dynamic String redirect is forbidden |
| `SEC-A05-001` | unsafe SQL construct |
| `SEC-A05-002` | unsafe HTML construct |

The `A01` and `A05` portions intentionally map to the corresponding OWASP Top 10 risk families. The identifiers are compiler diagnostics, not a claim of complete OWASP compliance.

## Design rule

RWLang should prefer the following order:

1. make the unsafe state unrepresentable;
2. otherwise enforce a secure default automatically;
3. otherwise require an explicit capability or policy;
4. use warnings only when compilation cannot prove enough to fail closed.

Developer ergonomics are part of the security model. Security features that require repeated opt-in are easy to forget, so RWLang should generally make safe behavior implicit and unsafe exceptions explicit.

## Next language layers

The next planned secure-by-construction work should build on this foundation rather than adding unrelated lints:

- typed trust flow (`Untrusted<T>` / validated domain values);
- `Secret<T>` and `Sensitive<T>` information-flow restrictions;
- authorization proof types such as `Authorized<T, Permission>`;
- typed route targets replacing string-built dynamic redirects;
- explicit outbound-network capabilities and SSRF-safe egress types;
- explicit public API projection instead of implicit model serialization;
- bounded collection/input types;
- typed browser/API route semantics with automatic CSRF/session rules;
- security audit requirements for privilege-changing mutations.
