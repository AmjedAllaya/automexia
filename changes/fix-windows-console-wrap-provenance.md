# Preserve long output through the Windows console transport

- Include a pinned Microsoft console runtime in Windows development builds and
  packages through the existing owned loader.
- Verify vendor hashes, architectures, signatures and software inventory without
  adding downloads or discovery to terminal operation.
- Exercise long, overflowing records through the packaged transport, including
  direct and nested WSL, while retaining the older transport as a diagnostic path.
