# Deny Kitty external resource access without application permission

- Reject file, temporary-file, and shared-memory image transfers and queries with a permission error before decoding resource names or retaining chunks.
- Remove native file access, shared-memory mapping, and temporary deletion from the terminal parser, and omit resource-content previews from parser logs.
- Preserve inline images, quiet replies, transfer isolation, and recovery after rejection. External transports remain unavailable pending a separately validated application consent broker.
