# Command-result surface assurance

This page owns the focused correctness and native visual evidence for completed-
command output grouping. User behavior and shell support remain authoritative in
[Shell integration](SHELL-INTEGRATION.md#prompt-ownership); the roadmap status
remains authoritative in [UI branding roadmap](UI-BRANDING-ROADMAP.md).

## Validation incident

The first native assertion proved that result geometry existed and that the
combined result rectangle contained varied pixels. That was insufficient: text,
the accent rail, or the divider could satisfy color diversity even if the
resting surface itself was imperceptible. It also did not prove that the surface
belonged to the command just completed. Completion was reported from incomplete
evidence, and manual use exposed the gap.

The remediation changes the acceptance contract rather than merely adjusting a
threshold:

1. every command must publish a result key newer than the previous result;
2. the result generation must own the immediately preceding prompt;
3. expected output tokens must be present in renderer-neutral visible text;
4. success/error state must match the completed command;
5. a native glyph-only region, excluding rail/divider decoration, must contain
   real output paint;
6. blank surface pixels must visibly differ from adjacent blank gutter pixels;
7. WGPU and the independent CPU fallback must pass the same contract;
8. generated snapshots and native hooks remain feature-gated test evidence and
   add no product I/O, PTY bytes, terminal rows, persistence, or telemetry.

Test-first execution also exposed a separate PowerShell defect. A shell-only
failure could observe a stale successful `LASTEXITCODE` from an earlier native
process and publish success styling. The prompt now captures PowerShell's
immediate `$?` state as semantic `0` success or `1` failure and leaves the user-
owned `LASTEXITCODE` unchanged. A focused regression reproduces the stale-zero
case before invoking the prompt.

## Automated contract

The native command matrix covers:

| Case | Required result |
|---|---|
| single-line `Write-Output` | fresh success surface and visible token |
| two-line PowerShell output | one fresh multiline success surface and both tokens |
| exact external `cmd.exe /D /C` invocation | fresh success surface and native output token |
| `Write-Error` | fresh failure surface, visible error token, and failure styling |
| `Get-Item` provider pipeline | fresh success surface, pipeline token, and filename |

Focused reproduction and ownership checks:

```text
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File tools/ci/test_shell_integration.ps1
cargo test -p automexia-terminal --bin automexia --locked --features native-gui-test-hooks result_
cargo test -p rio-vt --lib --locked semantic_command
bash tools/ci/test_shell_sources.sh
```

Native Windows validation:

```text
cargo build -p automexia-terminal --bin automexia --locked --features visual-test-hooks
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File tests/integration/resize-stress-windows.ps1 -Binary target/debug/automexia.exe -ResourceReport <private-wgpu-report> -ResultCapture <private-wgpu-png>
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File tests/integration/resize-stress-windows.ps1 -Binary target/debug/automexia.exe -ResourceReport <private-cpu-report> -ResultCapture <private-cpu-png> -UseCpuRenderer
```

The screenshots and path-bearing reports are private, ignored local evidence.
Portable QA bundles exclude terminal-content PNGs and retain only bounded,
redacted summaries.

## Recorded local evidence

Fresh Windows runs on 2026-08-25 produced:

| Evidence | WGPU | CPU fallback |
|---|---:|---:|
| representative commands | 5 passed | 5 passed |
| blank surface/gutter RGB distance | 27 | 27 |
| glyph color buckets | 38 | 38 |
| glyph luminance spread | 211 | 211 |
| handle growth | 73 | 73 |
| thread growth | 13 | 13 |
| private-byte growth | 45,637,632 | 46,444,544 |
| working-set growth | 25,546,752 | 26,464,256 |
| descendant-process growth | 10 | 10 |

Both native runs stayed below the existing ceilings of 384 handles, 48 threads,
536,870,912 private bytes, 536,870,912 working-set bytes, and 16 descendants.
The WGPU frame was visually inspected and showed ordinary, multiline, external,
error, provider/pipeline, and later history results with their persistent bands,
rails, gutters, rules, and success/failure badges. CPU glyph-area pixel evidence
matched WGPU; local image decoding confirmed the captured glyph region even
when an in-chat preview did not display the full frame correctly.

## Remaining gates

This evidence does not justify a universal completion claim. Fish currently has
`C/D` events without Automexia-owned `A/B` prompt generations; stock CMD has
`A/B` prompt markers without a supported truthful generic `C/D` completion hook.
Those shells remain ungrouped by design. Native Linux/macOS Bash/Zsh frames,
multi-theme and high-contrast captures, and Narrator/NVDA, VoiceOver, and Orca
delivery also remain outstanding. The feature therefore stays **Partially done**
in the roadmap until those requirements are resolved and recorded.
