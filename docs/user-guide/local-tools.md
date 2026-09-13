# Local search and command examples

In an Automexia shell:

```text
amx find file Dockerfile
amx find text "connection refused"
amx explain tar
amx explain git log
```

These commands return terminal-native results, never browser searches. Outside
the session helper, use `automexia` instead of `amx`.

## Find files or text

Install **ripgrep** (`rg`) using your chosen package manager. Automexia does not
install it. Searches cover the current directory and its descendants: change to
the intended project/subdirectory first. File queries are literal path substrings,
not glob patterns. Text queries are literal strings, not regular expressions.
Use `--` before a query beginning with a hyphen.

Ripgrep's ignore-file, hidden-file and binary defaults remain in effect. Common
credential filenames and private configuration directories are also excluded;
symlinks are not followed. Do not treat these exclusions as a complete secret
scanner. There is no background index, search history or search-result cache.

Results preserve relative paths; text results include line numbers. Control and
bidirectional-formatting characters are shown as escapes. Newlines inside a
filename cannot impersonate additional results. Non-UTF-8 data requires direct
byte-oriented ripgrep use rather than a lossy Automexia display.

Searches stop at 1,000 results, 32,768 listed files, 4 MiB of captured stdout or
the command deadline. Text search additionally uses a 2 MiB file-size limit and
50 matches per file. Narrow the query or current directory after a limit error.
No matches is a successful, explicit result; a client failure is not.

## Offline examples

Install the **tealdeer** client providing `tldr`. Other clients are not silently
substituted because their offline flags differ. Prepare or refresh its cache
yourself with `tldr --update` when you choose to allow the download. `amx explain`
disables automatic updates, asks for raw uncolored examples, and never executes
them. Review placeholders and effects before copying any example.

The installed client owns its cache and any version-specific cache maintenance.
Use that client's documented cache controls for removal. Automexia does not
create another cache or promise to sandbox an installed executable.

## Preview, cancellation and WSL

```text
amx find text --preview "connection refused"
amx explain --preview tar
```

Preview prints the executable label and exact argument array without launching
the client. Explicit previews and results can contain your query or file data;
do not share them without review. Automexia errors do not echo client stderr or
session paths.

Press Ctrl+C to cancel. Local commands have a 15-second overall deadline and
bounded process cleanup. Windows-backed WSL sessions use the pane's Linux
directory, tools and HOME, with a guest deadline of 12 seconds. Install Python 3
and the requested client **inside that distribution**. Python runs only on the
explicit command path, in isolated mode. Native Linux/macOS do not need this
WSL helper. Reopen an existing session after updating its shell integration.

WSL tool discovery uses only absolute entries in the guest PATH, in their original
order. Empty and relative entries (including `.`) are ignored before Python is
selected, so a project-local executable cannot replace the interpreter through
those entries. A PATH without absolute entries, with control characters, over
8192 bytes or over 256 entries is rejected. Configure a trusted absolute tool
directory if needed; Automexia does not change your shell configuration or install
a replacement. Explicit absolute locations are trusted, not sandboxed. This
also applies to the WSL helpers for `amx open`, `amx edit` and `amx repo`.

Set `AUTOMEXIA_AMX=0` before loading shell integration to disable the helper.
Pre-existing `amx` aliases, functions and executables are preserved.

See [browser searches](google-search.md) for deliberately browser-based results.
