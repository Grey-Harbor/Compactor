# Deploy with Docker

The image runs as UID/GID `10001` and expects:

- a read-only source at `/etc/compactor/redirects.json`;
- a writable event directory at `/var/lib/compactor`;
- HTTP traffic on port `8080`.

Build and start the included example:

```sh
docker compose up --build -d
docker compose ps
curl --fail http://localhost:8080/healthz
```

Compose uses a managed volume so the runtime user can write events. For a bind
mount, make its host directory writable by UID `10001`. Stop with
`docker compose down`; omit `--volumes` when event data must remain.

For TLS termination, continue with the [reverse-proxy guide](reverse-proxy.md).
