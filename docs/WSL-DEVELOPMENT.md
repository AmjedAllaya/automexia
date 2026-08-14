# Windows and WSL development

Automexia supports Windows and Linux development on the same machine, but each
toolchain must build from the filesystem native to that operating system.

## Why WSL reports slow I/O

Microsoft's
[WSL file-storage guidance](https://learn.microsoft.com/windows/wsl/setup/environment#file-storage)
recommends keeping files on the same operating-system filesystem as the tools
that operate on them. Linux Cargo work in `/mnt/c` or `/mnt/d` crosses the
Windows/WSL filesystem boundary for every metadata read, dependency scan, and
object write. A Rust workspace amplifies that cost across thousands of small
files. The WSL performance notification is therefore expected when Linux tools
operate on this Windows checkout; it is not evidence that Automexia runtime I/O
is leaking or consuming storage.

The inverse rule also applies: keep Visual Studio, Windows Cargo/MSVC, MSI
packaging, and native ConPTY/GPU tests in the NTFS checkout.

## Supported dual-native layout

Keep two Git checkouts and exchange source changes through commits:

```text
Windows / MSVC / ConPTY:
D:\workstation\projects\...\automexia-terminal\standalone

WSL / Linux Cargo / Unix PTY:
~/src/automexia-terminal
```

Create the WSL checkout from a WSL shell:

```bash
mkdir -p ~/src
git clone https://github.com/AmjedAllaya/automexia-terminal ~/src/automexia-terminal
cd ~/src/automexia-terminal
git switch <branch>
cargo xtask doctor
cargo ready
```

Before a Linux build, `pwd` must not begin with `/mnt/`. Leave
`CARGO_TARGET_DIR` unset or point it to a location under the Linux filesystem,
such as `$HOME/.cache/automexia-target`. Do not share `target/`, a Cargo
registry, or compiler caches between Windows and WSL.

Use Git to synchronize the checkouts:

```bash
git status
git add <files>
git commit -s -m "type(scope): summary"
git push
```

Then fetch and switch or pull that branch in the other native checkout. Never
copy build artifacts between the two environments.

## Workflow safeguards

`cargo xtask doctor` reports one of:

- `workspace I/O host-native/ok` for a normal Windows or Unix checkout;
- `workspace I/O WSL-native/ok` for a WSL checkout under the Linux filesystem;
- an actionable advisory when either source or `CARGO_TARGET_DIR` is on a
  mounted Windows drive.

Compilation-heavy project workflows fail before building when invoked from WSL
with source or target storage under `/mnt/<drive>`:

- `cargo dev`;
- `cargo automexia`;
- `cargo ready`;
- `cargo ci`;
- `cargo qa`;
- `cargo xtask check`.

Focused read-only diagnosis remains available, and raw Cargo commands keep their
standard behavior. For a one-off diagnostic only, the guard can be acknowledged:

```bash
AUTOMEXIA_ALLOW_SLOW_WSL_MOUNT=1 cargo xtask check
```

Do not use the override for routine work, CI, benchmarks, fuzzing, or release
evidence. It only acknowledges the known performance penalty; it cannot remove
it.

## Windows-triggered fuzzing

`cargo xtask test image-decoder-fuzz --seconds N` remains a supported Windows
command. The runner translates the current working tree once, copies its source
into a disposable WSL-native directory under `/tmp`, and performs all Cargo,
libFuzzer, corpus, and target I/O there. The copy excludes `.git`, the
workspace target, and generated fuzz target, corpus, and artifact directories
while retaining current tracked and untracked source edits.

The shell uses `pipefail`, explicit nightly Rust, bounded time/RSS, and a cleanup
trap. The disposable source, corpus, and build artifacts are removed on normal
exit, error, or interruption.

## Troubleshooting

If the guard reports a mounted Windows source:

1. Run `pwd` and confirm it starts with `/mnt/c` or `/mnt/d`.
2. Commit or stash source work in the Windows checkout.
3. Clone or update the same branch under `~/src`.
4. Run `cargo xtask doctor` and require `WSL-native/ok`.
5. Re-run the Linux command.

If only the Cargo target is mounted, unset `CARGO_TARGET_DIR` or move it to
the Linux filesystem. Use `cargo storage` and `cargo purge` independently
inside each checkout; neither command deletes source or the other environment's
artifacts.
