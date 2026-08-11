# librio-wasm

[`librio`](../librio) without its `pty` feature, compiled to
wasm32-unknown-unknown and exposed through wasm-bindgen. This inherited crate
is retained for engine compatibility; Automexia v0.4 does not publish a web or
npm SDK.

There is no PTY in a browser, so the host owns the transport: child
output goes in through `feed`, and bytes the terminal wants delivered to
the child (key encodings, mouse reports, DA responses) come back out
through the `output` callback. Wire those two to a WebSocket for a real
shell, or to an in-page interpreter for a demo.

Build:

```sh
cargo build -p librio-wasm --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir pkg \
  target/wasm32-unknown-unknown/release/librio_wasm.wasm
```

The crate is not published to crates.io. A public Automexia extension or web
SDK remains out of scope until the documented SDK milestone.
