# Configure JSONL Events

Use this guide to choose an event file, verify writes, and connect external
collection or retention.

## Prepare writable storage

Choose an existing directory writable by the runtime user. For a host process
running under a dedicated `compactor` account, prepare it explicitly:

```sh
sudo install -d -o compactor -g compactor -m 0750 /var/lib/compactor
export COMPACTOR_EVENTS_FILE=/var/lib/compactor/events.jsonl
```

Replace the account name and path to match your service manager. Do not make the
directory world-writable to work around an ownership mismatch.

When using the production container, the directory must be writable by UID/GID
`10001`. Mount the source file read-only and the event directory read-write.

Start Compactor. Startup fails before listening if the parent directory is missing
or the event file cannot be opened.

## Verify an event

Send a non-health request, then inspect the newest record:

```sh
curl -i http://127.0.0.1:8080/not-configured
tail -n 1 /var/lib/compactor/events.jsonl | jq .
```

The response is `404`, and the formatted event has outcome `not_found`. The file
itself remains JSONL: one compact JSON object per physical line.

## Connect collection and retention

Compactor opens the file in append mode and flushes after every event. It does not:

- create parent directories;
- call `fsync` for each record;
- rotate or reopen the file;
- limit disk use;
- ship events;
- manage retention; or
- repair a partial final record after a process or filesystem failure.

Prefer a collector that tails the active file and owns downstream durability. If
you rotate by renaming the file, restart Compactor so it opens the new path; the
running process otherwise continues writing to its existing file handle. Document
how your collector treats a partial final line.

See the [event format](../reference/jsonl-event-format.md) for the exact fields and
the [event contract](../reference/redirect-event-contract.md) before implementing a
different sink.
