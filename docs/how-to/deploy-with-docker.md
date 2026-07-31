# Deploy with Docker

Use this guide to run the included production image with persistent event storage.

The image runs as UID/GID `10001` and expects:

- a read-only source at `/etc/compactor/redirects.json`;
- a writable event directory at `/var/lib/compactor`;
- HTTP traffic on port `8080`.

## Start the included deployment

The Compose file mounts `examples/redirects.json` read-only and uses a managed
volume for events. Build and start it:

```sh
docker compose up --build -d
docker compose ps
curl --fail http://localhost:8080/healthz
```

Exercise the example redirect:

```sh
curl -i 'http://localhost:8080/project?from=docker'
```

Confirm a `302` response and a `Location` containing `from=docker`. Inspect the
newest event inside the container:

```sh
docker compose exec compactor sh -c \
  'tail -n 1 /var/lib/compactor/events.jsonl'
```

## Use production configuration

Replace the example source mount with a reviewed file managed by your deployment
system. Keep it read-only. For a bind-mounted event directory, make the host path
writable by UID/GID `10001` before starting the container.

Pin production deployments to a version tag or image digest rather than `latest`.
On upgrade, keep the previous image and redirect source available, start one
instance, check `/healthz`, and exercise a known redirect before shifting traffic.

Configure cache policy with environment variables when the defaults do not fit:

```yaml
environment:
  COMPACTOR_REDIRECT_CACHE_TTL_SECONDS: "300"
  COMPACTOR_REDIRECT_CACHE_MAX_ENTRIES: "10000"
```

The source mount must expose atomic file replacement if redirects will change
without a container restart. Some single-file bind mounts do not follow a rename
performed on the host; use a read-only directory mount or your platform's atomic
configuration projection and verify the behavior. See
[Configure the JSON redirect source](configure-json-source.md) before rollout.

## Stop without deleting events

```sh
docker compose down
```

Do not add `--volumes` when event data must remain. Compactor does not own volume
backup, rotation, or retention.

For TLS termination, continue with the [reverse-proxy guide](reverse-proxy.md).
