# Preserve terminal parsing across malformed and fragmented input

- Reject invalid OSC color widths and non-hexadecimal components before arithmetic, preserving all supported color precisions.
- Ignore overflowing DCS headers without activating Sixel, capability queries, or synchronized updates, and recover for subsequent valid commands.
- Preserve control dispatch and following printable text when UTF-8 characters arrive across reads, including repeat-character behavior.
