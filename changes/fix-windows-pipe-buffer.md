# Fix Windows pipe buffer ownership

- Use safe shared storage for the bounded Windows PTY pipe buffer.
- Preserve exact byte order and partial transfers, including zero-capacity operations.
- Cover wraparound, saturation, independent FIFO behavior and joined cross-thread endpoint cleanup.
