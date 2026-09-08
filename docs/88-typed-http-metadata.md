# Typed HTTP metadata

Server response metadata is a typed boundary rather than arbitrary `(String, String)` mutation.

Implemented domain/boundary types include:

- closed `HeaderName` values for platform-owned response headers;
- bounded/control-character-safe header values;
- `MediaType`;
- `FileName`;
- `ContentDisposition`.

Header values reject CR/LF and unsafe control bytes. Invalid response metadata fails closed before response bytes are emitted. Safe filenames are bounded single segments: path separators, traversal forms, dotfile-style unsafe names and response-splitting characters are rejected.

Persisted idempotency responses are revalidated when replayed; serialized header strings do not bypass the typed boundary.
