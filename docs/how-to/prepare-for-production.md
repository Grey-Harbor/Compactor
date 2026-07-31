# Prepare Compactor for Production

Use this checklist before placing Compactor on a public request path. It turns the
reference implementation into an explicit operating agreement between the team
that owns redirects, the proxy, the runtime, and the event collector.

## Confirm the product fit

Compactor is a good fit when redirects can be prepared as a complete configuration
document, activated by restarting the service, and analyzed outside the request
path. Choose another product when you require a dashboard, runtime mutation,
authentication, campaigns, built-in analytics, or zero-restart configuration.

Review [Why Compactor is not a URL shortener](../explanation/why-not-a-url-shortener.md)
before treating a missing management feature as an implementation gap.

## Assign operational ownership

Record who owns each boundary:

| Boundary | Required owner decision |
| --- | --- |
| Redirect source | Generate, review, distribute, and roll back the complete JSON document. |
| Reverse proxy | Terminate TLS and replace forwarding metadata from clients. |
| Compactor runtime | Run the pinned image, restart after source changes, and monitor logs and health. |
| Event storage | Provide writable storage and define collection, retention, rotation, and recovery. |
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

## Define rollout and rollback

Treat the source and image as separately versioned inputs. For every change:

1. Validate the complete source in a non-production instance.
2. Keep the previous source and image digest available.
3. Restart one instance and check `/healthz` before shifting traffic.
4. Exercise a known redirect and confirm its event.
5. Roll out remaining instances.
6. Roll back both source and image independently when necessary.

Compactor loads the source only at startup. Replacing the file without restarting
does not change active redirects.

## Give automation an exact contract

Humans and AI agents should make changes from the same inputs:

- the [JSON source format](../reference/json-source-format.md) for fields and
  validation;
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
- intended hosts and paths return expected statuses and locations;
- untrusted forwarding headers cannot change lookup identity;
- health checks are event-free and monitorable;
- the runtime UID can append to persistent event storage;
- event collection and disk limits are tested;
- source and image rollback procedures are documented; and
- operators know that sink failure is logged but does not replace the HTTP result.
