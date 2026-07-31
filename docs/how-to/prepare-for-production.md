# Prepare Compactor for Production

Use this checklist before placing Compactor on a public request path. It turns the
reference implementation into an explicit operating agreement between the team
that owns redirects, the proxy, the runtime, and the event collector.

## Confirm the product fit

Compactor is a good fit when redirects can be prepared as authoritative source
data, activated through a reviewed source rollout, and analyzed outside the
request path. Choose another product when you require a dashboard, mutation API,
authentication, campaigns, or built-in analytics.

Review [Why Compactor is not a URL shortener](../explanation/why-not-a-url-shortener.md)
before treating a missing management feature as an implementation gap.

## Assign operational ownership

Record who owns each boundary:

| Boundary | Required owner decision |
| --- | --- |
| Redirect source | Own JSON rollout or the HTTP service's availability, contract, authentication, and rollback. |
| Reverse proxy | Terminate TLS and replace forwarding metadata from clients. |
| Compactor runtime | Run the pinned image, choose cache policy, and monitor logs and health. |
| Event destination | Own JSONL storage or HTTP receiver availability, retention, and recovery. |
| DNS and traffic | Route only the intended public hosts to this deployment. |

Do not deploy until every row has an owner.

## Validate redirect behavior

Create representative definitions for every public host, path style, status code,
destination query, and response header you plan to use. Test at least:

- a matching `GET` and `HEAD` request;
- a valid path with no definition (`404`);
- an unsupported method (`405` with `Allow: GET, HEAD`);
- query forwarding with duplicate and blank values;
- trailing-slash and case-sensitive path differences; and
- a rejected source containing a duplicate ID or canonical URL.

Use the [URL normalization reference](../reference/url-normalization.md) as the
lookup oracle. Do not rely on browser normalization or filesystem intuition.

## Secure proxy metadata

Keep `COMPACTOR_TRUSTED_PROXIES` empty unless a known proxy must reconstruct the
public URL. When it is required:

1. List only the immediate proxy IP addresses or narrow CIDRs.
2. Configure that proxy to replace client-supplied `Forwarded` and
   `X-Forwarded-*` values.
3. Test a direct untrusted request containing forged forwarding headers.
4. Test malformed metadata from a trusted peer and confirm it returns `400`.

Follow [Run behind a reverse proxy](reverse-proxy.md) for a concrete configuration.

## Make event storage durable enough

The JSONL sink appends and flushes each record, but does not call `fsync`, rotate
files, repair partial writes, or manage retention. Decide:

- how much event loss is acceptable after a host or power failure;
- whether a collector tails the file or the file is shipped on a schedule;
- how rotation coordinates with a process that keeps the file open; and
- how disk usage is monitored and capped.

If those guarantees are insufficient, implement a different
`RedirectEventSink`; do not infer stronger durability from a successful flush.
With the HTTP sink, decide whether best-effort one-attempt delivery is sufficient.
There is no retry, batching, or durable spool, and an unavailable receiver loses
the event after the bounded call even though the selected redirect still succeeds.

## Choose cache and source policy

The default five-minute TTL limits ordinary source work while making updates
eventually visible. Set `COMPACTOR_REDIRECT_CACHE_TTL_SECONDS` from the required
change-propagation window and the source's cost. Set
`COMPACTOR_REDIRECT_CACHE_MAX_ENTRIES` from the active redirect working set and
available memory. These are operational decisions; automation must not infer them
from document size alone.

Only successful definitions are cached. Repeated requests for nonexistent keys
therefore reach the source each time; enforce abusive-request rate limits at the
reverse proxy. A cached redirect can remain stale indefinitely while its source
fails, with one retry per key every 30 seconds. Decide whether that availability
tradeoff is acceptable for security-sensitive redirect changes.

## Define rollout and rollback

Treat the source and image as separately versioned inputs. For every change:

1. Validate the complete source in a non-production instance.
2. Keep the previous source and image digest available.
3. Atomically replace the source on one instance; never rewrite it in place.
4. Exercise a cold and a changed redirect after the TTL, inspect events, and
   monitor source-error logs.
5. Roll out the same validated source to remaining instances.
6. Roll back both source and image independently when necessary.

Roll back by atomically restoring the previous complete document. Cold lookups
recover immediately and resident definitions recover through refresh; no restart
is required. `/healthz` does not reread the source, so it cannot prove that a new
document is valid.

For an HTTP source, deploy and validate the remote service before selecting it,
then exercise both a found and missing cold key. Roll back by restoring the prior
source selection or endpoint and restarting Compactor. Cached definitions protect
refresh failures indefinitely, but cold keys return `500`; rate-limit repeated
unknown keys at the proxy because not-found results are deliberately uncached.
For either HTTP adapter, use HTTPS, protect bearer files, and monitor timeout,
transport, status, and validation error categories. Health is intentionally not
readiness for these dependencies.

## Give automation an exact contract

Humans and AI agents should make changes from the same inputs:

- the [JSON source format](../reference/json-source-format.md) for fields and
  validation;
- the [HTTP adapter protocol](../reference/http-adapter-protocol.md) for remote
  source and sink behavior;
- the [URL normalization reference](../reference/url-normalization.md) for lookup
  identity;
- the [configuration reference](../reference/configuration.md) for runtime values;
- the [JSONL event format](../reference/jsonl-event-format.md) for output; and
- the repository verification commands in
  [Build Compactor from source](../tutorials/building-from-source.md).

Require automation to preserve unknown business intent: it may format and validate
a source, but it should not invent destinations, status codes, cache policy,
trusted proxy ranges, or retention policy.

## Production acceptance criteria

The deployment is ready when:

- the complete source passes startup validation;
- the selected cache TTL and entry limit fit rollout latency and working-set needs;
- intended hosts and paths return expected statuses and locations;
- untrusted forwarding headers cannot change lookup identity;
- health checks are event-free and monitorable;
- the runtime UID can append to persistent event storage;
- event collection and disk limits are tested;
- source and image rollback procedures are documented;
- source replacements are atomic and source-error logs are monitored; and
- operators know that sink failure is logged but does not replace the HTTP result.
