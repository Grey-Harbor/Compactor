# JSONL Event Format

Each line is one complete JSON object:

```json
{"event_id":"01K...","redirect_id":"docs","occurred_at":"2026-07-29T19:53:21.482Z","duration_ms":1.74,"outcome":"redirected","client":{"address":"203.0.113.8","user_agent":"curl/8.7.1"},"request":{"method":"GET","scheme":"https","host":"go.example.com","path":"/docs","query":null,"protocol":"HTTP/1.1","headers":{}},"response":{"status_code":308,"location":"https://docs.example.com/current/"}}
```

Timestamps are UTC RFC 3339 values. Duration is finite, non-negative milliseconds.
Location and redirect ID are null when unavailable. The captured request headers
are limited to `referer`, `accept`, `accept-language`, and `x-request-id`;
`user-agent` has its dedicated client field.

Writes append, serialize concurrent access, and flush per record. Flush does not
perform a per-event `fsync`, so v0.1 does not promise survival across power loss or
repair partial records after a process/filesystem failure. JSONL is an event
adapter format, not the event architecture.
