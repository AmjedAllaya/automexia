# Third-party notices

Automexia Terminal is MIT-licensed, but it includes the following separately
licensed components. Their licenses apply only to the identified files.

## Rio and inherited engine sources

The inherited Rio, Sugarloaf, Corcovado, Teletypewriter, and related engine
sources retain their upstream MIT notices. See `LICENSE`, `NOTICE.md`, and the
crate-local license files.

## RustCrypto InOut

`third-party/inout/` contains a modified copy of InOut 0.2.2, licensed under
MIT OR Apache-2.0. Both original licenses are included in that directory;
`UPSTREAM.md` records source provenance, the local correction and review policy.

## AccessKit native adapters

`third-party/accesskit-windows/` contains a modified AccessKit Windows 0.35.1,
licensed under MIT OR Apache-2.0, with Chromium-derived code under its included
BSD license. The directory preserves all three license texts and source notices.
`UPSTREAM.md` records the exact origin and local boundary corrections.

`third-party/accesskit-macos/` contains a modified AccessKit macOS 0.27.1 under
the same MIT OR Apache-2.0 terms and included Chromium BSD notice.
`third-party/accesskit-unix/` contains a modified AccessKit Unix 0.24.0 under
MIT OR Apache-2.0. Both directories retain their exact-source license texts and
document local changes in their own `UPSTREAM.md` files.

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


## Microsoft Windows Console ConPTY (Windows packages)

Windows packages include the pinned `Microsoft.Windows.Console.ConPTY` runtime:
`conpty.dll`, `x64/OpenConsole.exe`, and `arm64/OpenConsole.exe`. These files keep
their original Microsoft signatures and MIT license. The exact NuGet version,
source and hashes are recorded in `packaging/windows/conpty-runtime.json` and
in the release SBOM. Upstream: https://github.com/microsoft/terminal

Copyright (c) Microsoft Corporation. All rights reserved.

MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED *AS IS*, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
