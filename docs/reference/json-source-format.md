# JSON Source Format

The reference adapter reads one UTF-8 JSON object with one `redirects` array.
This complete example shows two independently addressable redirects:

```json
{
  "redirects": [
    {
      "id": "docs-home",
      "canonical_url": "https://go.example.com/docs",
      "redirect_url": "https://docs.example.com/current/",
      "status_code": 308,
      "response_headers": {
        "Cache-Control": "public, max-age=300"
      }
    },
    {
      "id": "project-home",
      "canonical_url": "https://go.example.com/project",
      "redirect_url": "https://example.com/projects/compactor?ref=redirect",
      "status_code": 302
    }
  ]
}
```

## Document field

| Field | Type | Required | Constraint |
| --- | --- | --- | --- |
| `redirects` | array | yes | Contains zero or more redirect objects. Every object must validate. |

## Redirect fields

| Field | Type | Required | Constraint |
| --- | --- | --- | --- |
| `id` | string | yes | Non-empty after trimming. Must be unique in the document. Stored as an opaque stable identity. |
| `canonical_url` | string | yes | Absolute HTTP(S) public URL. Query and fragment are discarded for lookup. Its normalized value must be unique. |
| `redirect_url` | string | yes | Valid absolute destination URL. Its configured query is retained before any incoming query. |
| `status_code` | integer | yes | One of `301`, `302`, `303`, `307`, or `308`. |
| `response_headers` | object of string values | no | Defaults to `{}`. Names and values must be valid HTTP headers and cannot override protocol-owned headers. |

Unknown fields are rejected at both the document and redirect level. Duplicate IDs,
duplicate normalized canonical URLs, invalid destinations, unsupported statuses,
and malformed or prohibited headers reject the entire source.

## Status selection

| Status | Typical contract |
| ---: | --- |
| `301` | Permanent redirect that may permit clients to change a later method according to HTTP semantics. |
| `302` | Temporary redirect that may permit clients to change a later method. |
| `303` | Direct the client to retrieve the destination with `GET`. |
| `307` | Temporary redirect that preserves the request method. |
| `308` | Permanent redirect that preserves the request method. |

Compactor accepts only `GET` and `HEAD`, but status semantics still matter to
caches and clients. Choose redirect policy explicitly; do not infer it from URL
shape.

## Header ownership

Compactor owns `Location`, `Content-Length`, `Connection`, `Transfer-Encoding`,
`Date`, and `Server`; configuration cannot set them. The source loads once at
startup, and a failed load prevents the listener from accepting traffic.

JSON is the reference adapter format, not the source architecture. Implementations
of `RedirectSource` expose the same validated domain definition regardless of
their external storage.
