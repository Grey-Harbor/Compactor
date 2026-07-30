# URL Normalization

Lookup keys contain scheme, host, effective port, and path.

- Scheme and host are case-insensitive and serialize in lowercase.
- Default ports (`80` for HTTP, `443` for HTTPS) are removed.
- An empty path becomes `/`.
- Query and fragment never participate in lookup.
- User information and missing/malformed hosts are rejected.
- Backslashes are rejected because their interpretation is ambiguous in HTTP URL
  parsers.
- Path case, trailing slashes, repeated separators, literal dot segments,
  percent-encoded dot segments, and percent-escape spelling remain significant.

For example, `/a/../b`, `/b`, `/%2e%2e/b`, `/%7Euser`, and `/~user` are distinct
lookup paths. Compactor preserves the request-target path instead of applying
filesystem-style or browser-style path cleanup.

Incoming query text is recorded independently. On a redirect, Compactor preserves
the destination's configured query, appends the incoming query after it, preserves
ordering, and permits duplicate names. Fragments are not forwarded.
