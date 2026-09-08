# Security monitoring and redaction

RWLang security controls emit structured security events rather than arbitrary free-form request data.

Platform event coverage includes authentication/MFA abuse, authorization/policy denial, CSRF/origin/CORS denial, webhook verification failure, idempotency conflict and resource exhaustion. Critical operations retain typed mandatory audit events.

Security event schemas do not accept arbitrary request payload/message fields. Principal/source identifiers are emitted as redacted values; raw identifiers may be used only as bounded in-memory correlation keys for alerting.

## Redacted values

`Redacted<T>` is proof-bearing state rather than an annotation cast. It is produced by the trusted builtin:

```rwlang
let safe = redact(value);
```

The runtime discards the original printable representation and exposes only the redacted form. `Sensitive<T>` or `Secret<T>` does not become public/redacted merely through a type annotation.

## Burst alerts

The platform maintains bounded burst detection for selected security event classes with cooldown behavior, so repeated attacks generate actionable alerts without unbounded alert/log amplification.
