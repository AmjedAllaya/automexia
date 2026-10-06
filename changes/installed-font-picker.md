Fonts now offers a searchable installed-font list with live terminal preview,
visible selection, keyboard navigation and explicit Apply/Cancel. Native font
discovery and preview preparation share one bounded worker; rapid browsing rejects
stale loads and never saves until Apply. Refresh detects newly installed fonts.
Shared UI text now snaps bitmap glyph placement to physical pixels, preventing
GPU atlas-edge artifacts at fractional positions and display scales.
