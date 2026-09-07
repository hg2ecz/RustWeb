# Tenant membership authority

RWLang's platform authentication session now carries a trusted, bounded set of tenant membership claims.
This is the authority foundation for a later compiler-enforced `scoped by tenantId` model system.

## Security properties

- Tenant IDs are canonical ASCII identifiers with a 128-byte maximum.
- A session carries at most 64 unique memberships.
- Anonymous sessions carry no memberships.
- Local-auth memberships are stored in the local auth database, not application input.
- Changing local-auth memberships increments `auth_generation`, invalidating existing sessions.
- LDAP deployments may provide `auth.memberships_file` / `--auth-memberships-file` with `username=tenant,tenant` mappings.
- LDAP role + membership mappings are hashed into the session generation. Mapping changes invalidate persisted sessions on the next request, including Redis-backed sessions after restart.
- Memberships are exposed to the runtime only as the internal `__authMemberships` value. Application code does not receive an ambient public tenant variable yet.

## Local auth administration

```text
rwlang-cli auth user-add ... --tenant acme --tenant beta
rwlang-cli auth memberships-set --db-url-file auth-url.txt --username alice --tenant acme
```

Membership changes are authentication-authority changes and therefore revoke previously issued authenticated sessions through generation mismatch.

## LDAP/operator mapping

Configuration file:

```toml
[auth]
memberships_file = "memberships.txt"
```

`memberships.txt`:

```text
alice=acme,beta
bob=acme
```

The mapping is fail-closed: invalid tenant IDs, duplicate users, or duplicate memberships are rejected during startup.

## Why there is no `currentTenant` yet

Membership and active tenant are different concepts. RWLang intentionally does not guess an active tenant from the first membership. The next tenant-isolation iteration will bind a route/model tenant scope to one of these trusted memberships and produce a compiler proof. Until then, the membership claim is platform authority only.
