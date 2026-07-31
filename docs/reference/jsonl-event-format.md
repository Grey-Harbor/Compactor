# JSONL Event Format

The reference sink writes one complete JSON object per physical line. For human
review, the same single event is shown pretty-printed below:

```json
{
  "event_id": "01K1C6Y7M4T2Q8J3A5N9P0R6VW",
  "redirect_id": "docs-home",
  "occurred_at": "2026-07-29T19:53:21.482Z",
  "duration_ms": 1.74,
  "outcome": "redirected",
  "client": {
    "address": "203.0.113.8",
    "user_agent": "curl/8.7.1"
  },
  "request": {
    "method": "GET",
    "scheme": "https",
    "host": "go.example.com",
    "path": "/docs",
    "query": "source=reference",
    "protocol": "HTTP/1.1",
    "headers": {
      "accept": "*/*",
      "x-request-id": "request-123"
    }
  },
  "response": {
    "status_code": 308,
    "location": "https://docs.example.com/current/?source=reference"
  }
}
```

The stored form contains no indentation or embedded newlines. Format records only
when reading them:

```sh
tail -n 1 events.jsonl | jq .
```

## Top-level fields

| Field | Type | Definition |
| --- | --- | --- |
| `event_id` | string | Sortable ULID generated independently for this transaction. |
| `redirect_id` | string or null | Stable source-owned ID when a definition matched; otherwise null. |
| `occurred_at` | string | UTC RFC 3339 timestamp recorded after response determination. |
| `duration_ms` | number | Finite, non-negative processing milliseconds; excludes sink latency. |
| `outcome` | string | `redirected`, `not_found`, `invalid_request`, or `source_error`. |
| `client` | object | Resolved client address and user agent. |
| `request` | object | Sanitized observed request identity and captured allowlisted headers. |
| `response` | object | Intended status and redirect location. |

## Nested fields

| Object | Field | Type | Definition |
| --- | --- | --- | --- |
| `client` | `address` | string or null | Resolved IP, or null when recording is disabled or unavailable. |
| `client` | `user_agent` | string or null | Truncated `User-Agent`, or null when absent or invalid text. |
| `request` | `method` | string | HTTP method as received. |
| `request` | `scheme` | string | Direct or trusted-proxy-resolved public scheme. |
| `request` | `host` | string | Direct or trusted-proxy-resolved public authority, including a non-default port. |
| `request` | `path` | string | Raw request path preserved independently from canonical lookup. |
| `request` | `query` | string or null | Raw incoming query without `?`, or null when absent. |
| `request` | `protocol` | string | HTTP version such as `HTTP/1.1`. |
| `request` | `headers` | object | Captured allowlisted request headers that fit configured byte budgets. |
| `response` | `status_code` | integer | Intended HTTP status. |
| `response` | `location` | string or null | Final absolute destination after query forwarding, or null. |

## Outcomes

| Outcome | Status | Meaning |
| --- | ---: | --- |
| `redirected` | configured 3xx | A definition matched. `redirect_id` and `location` are present. |
| `not_found` | `404` | Lookup succeeded with no definition. |
| `invalid_request` | `400` or `405` | Metadata, URL identity, or method was rejected before a successful lookup. |
| `source_error` | `500` | The source adapter failed during an uncached lookup. Background refresh failures keep serving stale redirects and therefore emit `redirected` for those requests. |

Sink failure is not an outcome. The event describes the response Compactor
determined before emission; a sink error is logged and does not replace that HTTP
result.

## Capture limits

Captured request headers are limited to `referer`, `accept`, `accept-language`,
and `x-request-id`; `User-Agent` has the dedicated client field. Values are
truncated at valid UTF-8 boundaries. Header values are considered in the fixed
allowlist order and omitted when adding a truncated value would exceed the total
budget. Cookies, credentials, authorization, and arbitrary headers are never
captured.

## Write and durability behavior

Writes append, serialize concurrent access, and flush per record. Flush does not
perform a per-event `fsync`, so Compactor does not promise survival across power loss or
repair partial records after a process/filesystem failure. JSONL is an event
adapter format, not the event architecture.
