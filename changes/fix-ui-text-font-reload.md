# Refresh UI text when the configured font changes

The existing font-reload boundary now refreshes immediate-mode labels as well
as terminal text. It clears stale font-derived measurements, glyph slots and
queued labels while retaining backend allocations and scale. Tests compare
repeated prepared-font replacements with fresh CPU pixels and measurements;
the text benchmark covers replacement and full-owner reconstruction separately.
