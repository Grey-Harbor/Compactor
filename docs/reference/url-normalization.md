# URL Normalization

Lookup keys contain scheme, host, effective port, and path.

- Scheme and host are case-insensitive and serialize in lowercase.
- Default ports (`80` for HTTP, `443` for HTTPS) are removed.
- An empty path becomes `/`.
- Query and fragment never participate in lookup.
- User information and missing/malformed hosts are rejected.
- Path case, trailing slashes, and path semantics remain significant.

Incoming query text is recorded independently. On a redirect, Compactor preserves
the destination's configured query, appends the incoming query after it, preserves
ordering, and permits duplicate names. Fragments are not forwarded.
