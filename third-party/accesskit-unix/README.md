# AccessKit Unix adapter

This is the Unix adapter for [AccessKit](https://accesskit.dev/). It exposes an AccessKit accessibility tree through the AT-SPI protocol.

## Compatibility with async runtimes

While this crate's API is purely blocking, it internally spawns asynchronous tasks on an executor.

This unpublished Automexia adaptation uses the existing async-io executor.
The unused alternative Tokio runtime is omitted; see [source provenance](UPSTREAM.md).
