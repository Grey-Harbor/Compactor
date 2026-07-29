# JSON Source Format

The reference source is one object with a `redirects` array:

```json
{
  "redirects": [{
    "id": "docs",
    "canonical_url": "https://go.example.com/docs",
    "redirect_url": "https://docs.example.com/current/",
    "status_code": 308,
    "response_headers": {"Cache-Control": "public, max-age=300"}
  }]
}
```

All fields except `response_headers` are required. Unknown fields, duplicate IDs,
duplicate normalized canonical URLs, relative or invalid destinations, unsupported
statuses, and malformed or prohibited headers reject the entire source.

Compactor owns `Location`, `Content-Length`, `Connection`, `Transfer-Encoding`,
`Date`, and `Server`; configuration cannot set them. The source loads once at
startup. JSON is an adapter format, not the source architecture.
