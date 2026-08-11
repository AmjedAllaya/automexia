# Third-party notices

Automexia Terminal is MIT-licensed, but it includes the following separately
licensed components. Their licenses apply only to the identified files.

## Rio and inherited engine sources

The inherited Rio, Sugarloaf, Corcovado, Teletypewriter, and related engine
sources retain their upstream MIT notices. See `LICENSE`, `NOTICE.md`, and the
crate-local license files.

## Cascadia Code Nerd Font

`sugarloaf/src/font/resources/CascadiaCode/*.ttf` is distributed under the
SIL Open Font License 1.1. The complete license is preserved at
`sugarloaf/src/font/resources/CascadiaCode/LICENSE`.

## Nerd Fonts Symbols Only

`rio-fonts/resources/SymbolsNerdFontMono/SymbolsNerdFontMono-Regular.ttf` is
from the Nerd Fonts Symbols Only distribution and is distributed under the SIL
Open Font License 1.1. The complete applicable license is preserved beside the
font. Upstream: https://github.com/ryanoasis/nerd-fonts

## librashader-derived runtime files

Files under `sugarloaf/src/components/filters/runtime/` identify source adapted
from SnowflakePowered/librashader and remain subject to
Mozilla Public License 2.0. Each affected source file retains its MPL-2.0 notice. The license is
available at https://www.mozilla.org/MPL/2.0/ and the upstream source is
https://github.com/SnowflakePowered/librashader.

## NewPixie CRT shader bundle

The bundled NewPixie CRT shader sources contain their upstream permissive
MIT/Unlicense terms in the source files. Those notices must not be removed.

The former `fubax_vr` bundle was removed because some files used Creative
Commons NonCommercial terms that are incompatible with unrestricted Automexia
distribution. No GPL-derived manual scripts are distributed.
