# HTTP Adapter Protocol

Use this reference when implementing a service consumed by Compactor or diagnosing
an adapter exchange. These are provider-neutral wire contracts; a provider may be
a control plane, custom API, or persistence gateway.

## Redirect resolution

Compactor sends one authoritative lookup:

```http
GET /resolve?url=https%3A%2F%2Fgo.example%2Fdocs HTTP/1.1
Accept: application/json
User-Agent: Compactor/0.2.0
Authorization: Bearer <configured-token>
```

This protocol has no built-in tenant parameter. If the configured endpoint has
provider-defined query parameters, they remain in order and Compactor appends one
percent-encoded `url` value containing the query-free canonical key. An endpoint
already containing any `url` parameter is invalid. Incoming client headers are
never forwarded.

`200 OK` requires an unencoded `application/json` or `application/*+json` body.
An optional media-type parameter such as `charset=utf-8` is accepted. The default
maximum is 65,536 bytes and is enforced while chunks arrive.

```json
{
  "id": "docs-current",
  "canonical_url": "https://go.example/docs",
  "redirect_url": "https://docs.example.com/current/",
  "status_code": 308,
  "response_headers": {
    "Cache-Control": "public, max-age=300"
  }
}
```

Unknown fields, malformed values, unsupported statuses, prohibited response
headers, and a canonical URL that differs from the requested key are source
errors. File and HTTP records use the same adapter-edge conversion.

| Remote result | Source result |
| --- | --- |
| `200` with a valid record | Found |
| `404` | Authoritative not found; body and content type ignored |
| Any other status | Source error |
| Timeout, transport, encoded body, oversized body, invalid content type or record | Source error |

Compactor does not follow redirects, honor `Retry-After`, retry, or cache in the
adapter. A cold source error produces `500/source_error`; a refresh error leaves
the stale redirect successful and the runtime permits another refresh after its
fixed 30-second cooldown.

## Event delivery

Compactor performs one bounded request after selecting the response:

```http
POST /events HTTP/1.1
Content-Type: application/json
User-Agent: Compactor/0.2.0
Authorization: Bearer <configured-token>
```

The body is the exact serialized [redirect event](redirect-event-contract.md).
Every `2xx` status is success. Any other status, timeout, or transport failure is
a sink error. There are no retries, batches, transformations, or durable queues.
The already selected redirect is unchanged, although awaiting the single POST may
delay its delivery by at most the configured request timeout. `duration_ms` stops
before sink delivery begins.

Both adapters share one pooled rustls client when selected together. Connections
have a shared bounded connect timeout, requests have independent total timeouts,
TLS verification is enabled, and redirects are disabled. Response bodies,
credentials, authorization headers, and complete endpoint URLs are not logged.
