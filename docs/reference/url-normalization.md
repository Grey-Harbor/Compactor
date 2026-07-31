# URL Normalization

Use this reference when generating redirect keys or diagnosing an unexpected
`404`. Compactor looks up a canonical URL built from the effective request scheme,
host, port, and the request-target path.

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

Given these requests, the lookup keys are:

| Request | Canonical lookup key |
| --- | --- |
| `http://Example.COM:80/` | `http://example.com/` |
| `https://example.com:443/docs/` | `https://example.com/docs/` |
| `https://example.com:8443/docs` | `https://example.com:8443/docs` |
| `https://example.com/docs?from=mail` | `https://example.com/docs` |

Configure the exact canonical key that a request produces. Do not pre-normalize
paths beyond the rules above, and include a non-default effective port.

Incoming query text is recorded independently. On a redirect, Compactor preserves
the destination's configured query, appends the incoming query after it, preserves
ordering, and permits duplicate names. Fragments are not forwarded.

For example, a destination of `https://new.example/docs?lang=en` and an incoming
query of `ref=email&lang=fr` produce:

```text
https://new.example/docs?lang=en&ref=email&lang=fr
```

When Compactor runs behind a proxy, trusted forwarded metadata can change the
effective scheme and host used here. Configure that boundary before adding source
records; see [Run behind a reverse proxy](../how-to/reverse-proxy.md).
