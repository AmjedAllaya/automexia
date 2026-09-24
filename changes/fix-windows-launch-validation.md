# Bound Windows launch preparation before copying

- Validate the entire quoted command, including escaping, argument separators, Unicode units, and the final terminator, before allocating its command-line buffers.
- Reject oversized or malformed explicit environment batches before copying wide entries or reading the inherited environment.
- Preserve existing Windows argument quoting, ordinal environment ordering, last-override behavior, and native resource limits.
