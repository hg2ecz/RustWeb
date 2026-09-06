# 15. Web security checklist

## Fejlesztő

- dinamikus HTML text csak `{{ ... }}`;
- URL csak `@href/@action`;
- SQL csak `:bind`;
- mutáció `Transaction` alatt;
- POST action + CSRF; JSON POST-nál `X-CSRF-Token`;
- CORS csak exact allowlistből; wildcard ne legyen;
- authorization route boundaryn;
- role/price/tenant authority szerveroldalon;
- lista paginált;
- upload destination server-owned;
- kliens filename csak metadata;
- ne kérj nagyobb resource profile-t indok nélkül.

## Production átadás előtt

- HTTPS/public host; reverse proxy mögött loopback listener + explicit trusted proxy CIDR; forwarding metadata csak trusted proxytól authority;
- Redis session clusterhez;
- LDAPS/TOTP config, ha auth kell;
- DB least-privilege credential;
- AppFs data root elkülönítve;
- request és hard cgroup limitek;
- integration + load + security tesztek.

- object authorizationnál csak trusted session principal/role legyen authority; owner mező stabil/immutable legyen;

## Trust-boundary evidence

- invalid/untrusted forwarding metadata: fail-closed `400`, security audit `proxy/invalid_forwarding`;
- Host mismatch: `421`, `host/mismatch`;
- HTTPS-required: `426`, `transport/https_required`;
- CSRF/CORS/origin deny: `403`, külön `csrf`/`cors`/`origin` audit category;
- object/route authorization deny: `403`, `policy/forbidden`;
- rate-limit deny: `429`, `rate_limit/denied`;
- auditba ne kerüljön cookie, token, credential, teljes request body vagy secret.
