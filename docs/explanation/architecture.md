# Architecture

Compactor separates policy-free domain contracts from transport, runtime, and
storage details. The HTTP layer normalizes the request, chooses the response,
sanitizes metadata, and measures the transaction. The redirect runtime owns
residency and freshness. Sources resolve authoritative state; sinks persist
completed events. None reconstructs another layer's work.

For an adopter, that separation creates a short and inspectable request path:

1. derive a canonical lookup key from trusted request metadata;
2. ask the runtime for one redirect definition, allowing it to use or refresh its cache;
3. choose the HTTP response;
4. construct one sanitized transaction event; and
5. ask the sink to persist it without changing the response.

The order is deliberate. The source cannot learn about HTTP-specific client
metadata or choose cache policy, and the sink cannot reinterpret the redirect
decision. A refresh failure can preserve a stale redirect, while a failing sink
remains observable without turning successful redirects into errors.

This keeps new adapter combinations possible without turning Compactor into a
general framework. For component ownership, failure behavior, and the full data
flow, see the maintained root [architecture document](../../ARCHITECTURE.md).
