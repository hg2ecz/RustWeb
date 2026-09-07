# Platform-owned browser sessions

RWLang browser authentication uses one platform-owned session authority. Application code does not need to mint, rotate, serialize, or configure the browser authentication cookie.

## Secure happy path

The server owns these invariants:

- successful authentication rotates the session identifier and CSRF token;
- logout invalidates the authenticated session and creates a fresh anonymous session;
- local password, MFA, disabled-state, and other auth-generation changes invalidate older authenticated sessions on the next request;
- production cookies use the `__Host-rw_session` name, `Path=/`, `HttpOnly`, and `Secure`;
- `SameSite=Lax` is the default; credentialed cross-origin deployments use `SameSite=None` while retaining `Secure`;
- the cookie has no `Domain` attribute, preserving host-only scope.

This means normal RWLang application code should express authentication intent (`auth user`, `auth mfa`, `auth role ...`) rather than session transport mechanics.

## Developer ergonomics

Security-critical cookie construction lives in one server policy module and is covered by regression tests. Applications do not need per-route cookie settings or manual login rotation calls.

Application-level bearer tokens remain appropriate for explicit domain workflows such as password reset or API credentials. They should not be used as a second browser-login session system alongside the platform session.
