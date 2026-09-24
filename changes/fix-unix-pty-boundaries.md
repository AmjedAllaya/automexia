# Fix Unix PTY launch boundaries

- Validate account lookup status, result identity, bounded string storage, and Unicode before using native account data; stop buffer growth at an explicit limit.
- Read PTY names through caller-owned platform buffers and return errors for invalid descriptors or malformed names.
- Own both PTY descriptors immediately after creation and release them when command preparation or spawning fails.
- Reuse the transactional process launcher for the existing fork entry point while preserving its inherited environment, arguments, and macOS bare-shell login convention.
- Register child-exit signals before launching and preserve mapped standard streams when the parent starts with closed standard descriptors.
