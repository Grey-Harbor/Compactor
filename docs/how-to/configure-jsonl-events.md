# Configure JSONL Events

Choose an existing writable directory and configure the output file:

```sh
install -d -m 0750 /var/lib/compactor
export COMPACTOR_EVENTS_FILE=/var/lib/compactor/events.jsonl
```

Compactor creates the file, opens it in append mode, and flushes after every event.
It does not create parent directories, rotate files, or manage retention. Use an
external collector or rotation workflow. See the
[event format](../reference/jsonl-event-format.md) for the wire contract.
