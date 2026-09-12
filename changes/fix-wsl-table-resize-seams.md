Fixed WSL/ConPTY resize corruption that discarded table column spacing at the
scrollback boundary or repainted over adjacent text at an exact cursor margin.
Copying wrapped blank fragments now preserves gaps without adding newlines.

Added real WSL listing, queued resize, narrow-pane, retained-selection, exact CPU
pixel and correctness-checked benchmark coverage. Other native desktop and
architecture validation remains separate; no shell command is replayed.
