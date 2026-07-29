# Configure the JSON Redirect Source

Create a document matching the [JSON source format](../reference/json-source-format.md),
then point Compactor at it:

```sh
export COMPACTOR_REDIRECTS_FILE=/etc/compactor/redirects.json
cargo run --release
```

Validate changes by restarting the process. Compactor loads the whole document
atomically and refuses to start if any definition is invalid; it never serves a
partially valid source. Replace the file through your configuration-management
system rather than attempting runtime mutation.
