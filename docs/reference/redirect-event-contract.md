# Redirect Event Contract

Implement this contract when completed request events need to go somewhere other
than the bundled JSONL file. The application constructs the event; the sink only
persists or transmits that already-sanitized value.

```rust
#[async_trait]
pub trait RedirectEventSink: Send + Sync {
    async fn emit(
        &self,
        event: &RedirectEvent,
    ) -> Result<(), RedirectEventSinkError>;
}
```

The HTTP/application layer supplies a complete sanitized event. A sink must not:

- inspect the original request;
- recalculate client identity or trusted-proxy behavior;
- add, remove, or re-sanitize captured metadata;
- invent timestamps or determine outcomes; or
- change the HTTP response when persistence fails.

Supported outcomes are `redirected`, `not_found`, `invalid_request`, and
`source_error`. Sink failure is operational and is never itself a redirect
outcome. The application logs that failure after choosing the HTTP response.

`event_id` is a sortable ULID unique to the transaction. `redirect_id` is the
optional stable source identity and is absent for unresolved requests.

An adapter should preserve event ordering where practical and document its own
delivery guarantees. The contract does not promise retries, batching,
deduplication, or exactly-once delivery. Consumers should use `event_id` as the
transaction identity if their downstream system needs deduplication.

For the complete event schema and the bundled sink's durability behavior, see the
[JSONL event format](jsonl-event-format.md).
