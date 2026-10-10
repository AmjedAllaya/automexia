# Native operating-system information tags

Native Linux distributions, including Linux Mint and LMDE, now publish an OS
information tag without requiring WSL or the DevOps extension. Shared, bounded
os-release parsing supports unfamiliar distribution names, caches discovery per
shell and falls back to the host OS. macOS/BSD identity and PowerShell on Unix
use the same display path. WSL launch metadata remains separate.

Existing tag IDs, colors and visibility are retained. Optional prompt metadata
clears across nested shells, stale provider OS tags cannot duplicate the core tag,
and full OS names remain available to accessibility. Regression coverage includes
parser fixtures, shell publishers, installation layouts, frame admission, core
projection and native Unix UI checks.

The OS setting uses a compact, platform-neutral label; measured preview-column
coverage prevents truncation at fractional display scales.

The tag preview measures labels before choosing its column count, preserving
readability at larger interface fonts. Reviewed references retain exact pixel
comparisons across themes and scales.
