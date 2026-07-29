# Architecture

Compactor separates policy-free domain contracts from transport and storage
details. The HTTP layer is the orchestrator because it alone has enough context to
normalize the request, choose the response, sanitize metadata, and measure the
transaction. Sources resolve; sinks persist. Neither reconstructs the other's work.

This keeps new adapter combinations possible without turning v0.1 into a framework.
For component responsibilities and data flow, see the maintained root
[architecture document](../../ARCHITECTURE.md).
