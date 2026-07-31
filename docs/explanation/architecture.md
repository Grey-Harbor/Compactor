# Architecture

Compactor separates policy-free domain contracts from transport and storage
details. The HTTP layer is the orchestrator because it alone has enough context to
normalize the request, choose the response, sanitize metadata, and measure the
transaction. Sources resolve; sinks persist. Neither reconstructs the other's work.

For an adopter, that separation creates a short and inspectable request path:

1. derive a canonical lookup key from trusted request metadata;
2. ask the source for one redirect definition;
3. choose the HTTP response;
4. construct one sanitized transaction event; and
5. ask the sink to persist it without changing the response.

The order is deliberate. The source cannot learn about HTTP-specific client
metadata, and the sink cannot reinterpret the redirect decision. A failing sink
therefore remains observable without turning successful redirects into errors.

This keeps new adapter combinations possible without turning Compactor into a
general framework. For component ownership, failure behavior, and the full data
flow, see the maintained root [architecture document](../../ARCHITECTURE.md).
