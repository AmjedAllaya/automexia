# Image previews

Automexia has two complementary image paths. They solve different problems and
must not be confused.

## Inline terminal graphics

A program running in the PTY can explicitly send image escape sequences. The
inherited Rio engine parses the sequence, stores bounded image state, and the
Automexia renderer places the resulting texture inside the owning pane. Images
scroll, clip, clear, and switch screen buffers with terminal content.

Supported protocol families are:

- Kitty Graphics, including direct placements and Unicode placeholders;
- iTerm2 OSC 1337 inline images, including pixel/cell/percentage sizing and the
  `doNotMoveCursor=1` extension;
- Sixel.

This is the normal path for image-aware programs. Kitty's official protocol
uses `kitten icat image.png` as its basic example; iTerm2 uses `imgcat`; WezTerm
provides `wezterm imgcat image.png`; and file managers such as Yazi render the
currently selected file through an appropriate terminal graphics protocol.
Automexia preserves its own `TERM_PROGRAM=Automexia` identity and does not
pretend to be Kitty, Rio, iTerm2, or WezTerm. A client can query the advertised
`sixel`, `iterm2`, or `kitty` terminal capabilities or explicitly emit one of
the supported protocols.

Protocol support does not mean that every third-party application recognizes a
new terminal brand automatically. If an application's detection table has not
yet learned Automexia, use its explicit Kitty/iTerm2/Sixel adapter setting
rather than changing `TERM_PROGRAM` or `TERM` to impersonate another terminal.
Multiplexers also need to pass the selected graphics protocol through.

## Local quick look

Automexia also previews a local raster path already visible in terminal output.
This is an emulator-owned overlay; it does not write escape sequences into the
PTY or alter scrollback.

- Hover a supported image filename or path. A 100 ms stability delay avoids
  decoding every cell crossed during fast pointer movement.
- Click the filename/path to pin its preview.
- While pinned, `Down`/`Right` selects the next visible image path and
  `Up`/`Left` selects the previous one. Navigation wraps at both ends.
- Press `Esc` to close a pinned preview. Typing another key or clicking outside
  it also returns input ownership to the shell.
- Keyboard: select a path and press `Ctrl`+`Alt`+`I` on Windows/Linux/BSD or
  `Cmd`+`Alt`+`I` on macOS.
- Command palette: run **Preview Selected Image**. With no selection, the
  command uses the supported path under the pointer.

Bare filenames from ordinary `ls`, quoted names containing spaces, rooted
paths, explicit relative paths, Unicode names, and Automexia's file-listing
glyph prefixes are supported. Relative paths resolve
against validated OSC 7 current-directory metadata, then the immutable launch
directory while the first prompt is still starting. On Windows, a known WSL
session may translate `/mnt/<drive>/...` to the local drive or another absolute
Linux path through `\\wsl.localhost\\<validated-distro>\\...`.

When a full-screen terminal application enables mouse reporting, Automexia
does not steal its mouse events. Hold `Shift` while hovering or clicking to use
quick look through the terminal's standard host-UI override. Hover previews do
not capture arrow keys; only an explicit click or keyboard/palette action pins
the card and enables image navigation.

The card keeps the source aspect ratio, never enlarges a small image, limits a
large GPU upload to 1280x960, flips at pane edges, and disappears instead of
overlapping the terminal when the pane is physically unusable. An unpinned
hover card disappears when the pointer leaves the candidate or window. A
pinned card survives pointer movement and is dismissed by `Esc`, ordinary
typing, an outside click, scrolling, selection, route replacement, or explicit
dismissal. Resizing recomputes its position from the current pane rectangle.

Quick look supports BMP, GIF, ICO, JPEG, PNG/PNM, TIFF, and WebP raster files.
It intentionally does not open SVG, PDF, URLs, remote hosts, directories,
symlinks, named pipes, or device files. Use an explicit trusted application for
those formats. Animated formats receive a static quick-look frame; protocol
clients own richer playback behavior.

## Security and performance contract

Terminal text is attacker-controlled, even when it comes from a local command.
Consequently quick look:

- performs candidate discovery and hit testing without filesystem access;
- waits for a stable 100 ms hover target or explicit click before submitting
  work, and never reads or decodes on the UI, renderer, or PTY threads;
- rejects URL schemes, arbitrary Windows UNC paths, control characters,
  symlinks, and non-regular files;
- limits the source file to 20 MiB and accepts only supported raster magic;
- parses dimensions before full decode, then enforces 4096x4096, 16,777,216
  decoded pixels, and a 96 MiB decoder-allocation ceiling;
- verifies the opened file's length and modification version before and after
  reading so a changing file is not cached as a stable thumbnail;
- decodes and downsizes on one worker with a 16-owner latest-request queue;
  a newer request replaces queued work from the same window;
- gives every window one bounded completion mailbox and a generation token, so
  replacement, dismissal, route changes, and window isolation reject obsolete
  completion without a shared result queue or cross-window eviction;
- retains at most 16 decoded thumbnails and 32 MiB in an access-ordered cache;
  cache hits share the exact pixel allocation with Sugarloaf and use a stable
  texture key/time rather than copying, hashing, decoding, or re-uploading;
- uses the same positive-z paint contract on WGPU, Metal, Vulkan, and CPU:
  card/UI geometry first, image pixels second, then dedicated UI labels;
- measures metadata before fitting the preview title, eliding long filenames
  into the remaining width so the title and dimensions never overlap;
- removes the active overlay, route pixels, and matching GPU texture immediately
  on dismissal; only the separately bounded CPU thumbnail LRU retains reusable
  decoded pixels, and cache replacement uses exact entry/byte accounting.

The iTerm2 protocol decoder separately caps decoded input at 64 MiB, validates
an optional declared `size`, and applies the same 4096x4096 and 96 MiB decoder
limits before constructing terminal graphics. Kitty/Sixel retain their
existing protocol-specific payload, dimension, and shared graphics-quota
checks.

## Verification

Focused checks are:

```text
cargo xtask test image-rendering
cargo xtask test image-rendering --native-gui
cargo test -p automexia-image --locked
cargo test -p automexia-terminal image_preview --bin automexia --locked
cargo test -p sugarloaf shared_rgba_preserves --locked
cargo xtask test image-decoder-fuzz --seconds 120
cargo test -p automexia-terminal bindings --locked
cargo test -p automexia-terminal command_palette --locked
cargo test -p rio-vt --features graphics bounded_decoder --locked
cargo xtask verify architecture
cargo bench -p automexia-terminal --bench image_preview --locked -- --noplot
```

The first command is the required PR gate. It runs the complete bounded decoder
and cache suite, preview state-machine tests, Sugarloaf CPU and texture-budget
tests, Rio VT/backend graphics regressions, and compiles the Criterion target.
The decoder suite covers every enabled raster codec, exact RGBA byte accounting,
straight-alpha transparency, portrait/landscape resize, malformed/truncated and
deterministically mutated input, invalid paths, cache eviction/replacement storms,
1,000 repeated warm hits, file-handle release, and absence of generated sidecar
or thumbnail files.

The release benchmark uses a 1600x1000 fixture and compares cold decode/resize
with a file-version-validated warm lookup. On the 2026-08-15 Windows
development host the medians were 23.879 ms and 47.813 us respectively
(approximately 499x faster for reuse); Criterion classified both changes
against its saved local baseline as within the configured noise threshold.
This is a local observation, not the
controlled 30-day performance baseline or a cross-host guarantee.

The full `cargo ready` gate remains required before merge. On Windows,
`cargo xtask test image-rendering --native-gui` prints two real relative image
filenames, drives hover/click/arrow/Escape through renderer-neutral cell
geometry, and repeats 16 open/dismiss cycles on both WGPU and the CPU fallback.
Every active cycle requires one route pixel entry and overlay; WGPU additionally
requires one exact `width * height * 4` texture allocation. Every dismissal
requires zero active pixel, overlay, texture, queued-request, and pending-result
state. The runner also applies strict process handle/thread/private-memory
ceilings, verifies the exact bounded thumbnail-cache accounting, samples
transparent and opaque image-body regions, rejects blank/obscured output, and
compares WGPU/CPU dimensions and luminance distributions within controlled
tolerances.

This proves the local Windows paths covered by the automation; it is not a
mathematical guarantee for every decoder input, GPU driver, compositor, remote
filesystem, or third-party protocol client. Controlled Linux/macOS GPU runs,
extended sanitizer/fuzz soak, multiplexer/client compatibility, and the 30-day
performance baseline remain release evidence. Visual review complements but
does not replace deterministic route, geometry, resource, and decoder tests.

`cargo-fuzz`/libFuzzer is supported on Unix-like systems. The xtask command
therefore installs/uses explicit nightly on Linux/macOS and automatically
routes Windows through WSL, avoiding an unsupported native-Windows ASan DLL
configuration. Nightly CI likewise installs nightly and runs
`cargo +nightly fuzz`; the workspace's pinned stable compiler is never used for
fuzz execution.

On 2026-08-14 the supported Windows-to-WSL command completed 544,609
libFuzzer executions in 121 seconds with no crash or sanitizer finding. The
run reached 2,504 covered edges and 5,383 features, retained 1,161 in-memory
corpus entries, and peaked at 357 MiB RSS. The current target exercises both
bounded decoding and path-token discovery. Local runs use disposable build and
writable-corpus directories, cap libFuzzer at 768 MiB RSS and 15 seconds per
input, and remove generated campaign state after completion so fuzzing cannot
silently fill the repository drive. A failing crash artifact must be retained and attached
to the security regression that fixes it.

## Primary references

- Kitty Graphics Protocol: <https://sw.kovidgoyal.net/kitty/graphics-protocol/>
- iTerm2 inline image protocol: <https://iterm2.com/documentation-images.html>
- WezTerm iTerm image support: <https://wezterm.org/imgcat.html>
- Yazi image preview adapters: <https://yazi-rs.github.io/docs/image-preview/>
- image crate ImageReader limits: <https://docs.rs/image/latest/image/struct.ImageReader.html>
- image crate limit semantics: <https://docs.rs/image/latest/image/struct.Limits.html>
- Rust Fuzz Book / cargo-fuzz: <https://rust-fuzz.github.io/book/cargo-fuzz.html>
- Ghostty features and native Quick Look: <https://ghostty.org/docs/features>
