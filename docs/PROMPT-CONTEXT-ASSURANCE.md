# Kubernetes prompt context assurance

The DevOps prompt badge shows only the namespace and retains the complete
context and namespace in its accessible description. This is local configuration
information, not proof of connectivity, authentication or namespace existence.
The larger Kubernetes glyph uses the shared optical-sizing policy and stays
separate from its label. No context suffix or decorative separator is appended.

Discovery refreshes on the existing three-second background schedule. A normal
`kubectl config set-context --current --namespace=sandbox` change to the selected
configuration does not require changing directories or reopening the pane.
Each open session retains its prompt history across pane and tab focus changes,
including hidden tabs. Closing a session removes its context state; disabling
context status clears that state and skips route tracking. Terminal cells,
selection and command input are unchanged. In narrow panes badges may be omitted
when they do not fit.

The distribution and validated shell username are projected immediately from
shell metadata without waiting for discovery. Window-title changes no longer
cancel valid discovery or discard its cache; directory, process, shell identity
and path-hint changes still do. During initial WSL discovery, the existing worker
publishes available Git/context fields before waiting for the guest Kubernetes
read. The existing 100 ms pending-context timer observes this intermediate state;
the completion wake publishes the final result. An ordinary background refresh
retains the previous namespace until it completes, avoiding a periodic blink.
No additional worker, persistent cache or faster idle polling is introduced.

On Windows, integrated WSL sessions use their distribution and shell identity to
locate the guest configuration. Bash, Zsh, Fish and PowerShell publish bounded
local HOME and exported KUBECONFIG hints, including changes without a directory
change. Native Windows PowerShell also publishes exported HOMEDRIVE, HOMEPATH
and USERPROFILE. Its Kubernetes lookup follows the first existing local
configuration in HOME, HOMEDRIVE+HOMEPATH, then USERPROFILE, instead of assuming
PowerShell's immutable home matches the Kubernetes client. Candidate inspection
runs on the discovery worker; prompts only encode bounded values. Older two-value
hooks keep their existing HOME contract. Old guest hooks without hints use the
conventional guest home fallback.
Guest reads are isolated in a
short-lived application helper with a 1.5-second deadline, cancellation, a 4 KiB
reply limit and native child reaping. A slow/unavailable guest never selects the
Windows host's cluster as a substitute. Nothing invokes kubectl, a shell, an
authentication helper or a Kubernetes API in the product discovery path.

The passive parser accepts YAML mapping reordering, quoted labels, inline maps
and JSON. The first file defining a named context wins, including an omitted
namespace, which means `default`. At most sixteen files, 1 MiB per file, 256
contexts per file, one YAML document and bounded parsing complexity are accepted.
Malformed input produces no context; credential fields and parser excerpts never
leave the parser. This is deliberately different from managed import review.

## Current limitations

Set `AUTOMEXIA_CONTEXT_PATH_HINTS=0` in the shell to stop sharing these paths; the
next prompt clears every published location field. Removing that setting restores
publication. Invalid/oversized values clear the complete snapshot, and incomplete
metadata frames retain the previous complete identity, location, directory and
title snapshot until the exact completion marker arrives. Malformed markers
cannot publish mixed shell facts; older unframed hooks remain compatible. Two values
are retained for Unix/guest shells; native Windows PowerShell retains five bounded
location values. Windows candidates are dropped when a guest or older hook resumes,
and are never forwarded to the guest helper. Generic serialization/debug output
excludes these values. Normal prompts replay cached
frames without new encoder processes; Unix encoding needs the existing base64
and tr utilities when a value changes. Bash 3.2 has an export-attribute subshell
fallback; native macOS execution of that fallback remains a separate gate.
Zsh uses targeted native executable lookups rather than expanding its commands
table, which can enumerate mounted PATH directories in each cold encoding
subshell. This preserves command selection and the user's hashing options.

CMD cannot dynamically encode changed variables through its native PROMPT alone.
It uses the environment inherited by the application and discards stale guest
hints. Windows network/device configuration paths are rejected. Guest `..` path
components are rejected instead of crossing the guest translation boundary;
use an absolute guest path for such an override. Native provider-backed filesystem
reads and physical desktop/screen-reader checks have separate assurance gates.
A guest helper failure hides its badge rather than
claiming another environment. GUI pixels and screen-reader delivery require
their own native verification; process and model tests do not establish them.

## Verification

Run the owner and boundary tests:

```text
cargo test --locked -p automexia-devops
cargo test --locked -p automexia-terminal --lib prompt_discovery
cargo test --locked -p automexia-terminal --bin automexia kubernetes_icon_raster
python tools/ci/check_prompt_discovery.py
python tools/ci/test_prompt_discovery.py
python tools/ci/test_shell_location_hints.py
cargo test --locked -p automexia-terminal --bin automexia fragmented_location_metadata
cargo test --locked -p automexia-terminal --bin automexia context_ownership_
cargo test --locked -p automexia-terminal --lib automexia::runtime::tests
cargo test --locked -p automexia-terminal --bin automexia title_change_accepts_progress
cargo bench --locked -p automexia-terminal --bench automexia_services -- prompt_identity_before_discovery
cargo bench --locked -p automexia-devops --bench kubernetes_context
python tools/ci/test_shell_location_hints.py --benchmark target/qa/prompt-replay.json
```

With kubectl installed, the explicit Linux/native client regression writes only
a temporary fictional configuration and never contacts its fake API endpoint:

```text
cargo test --locked -p automexia-devops --test kubernetes_prompt native_kubectl -- --ignored
```

For Windows with an installed WSL distribution, build the actual application and
run the helper rehearsal using that distribution's name:

```text
cargo build --locked -p automexia-terminal --bin automexia
python tools/ci/native_wsl_kubernetes_prompt.py --binary target/debug/automexia.exe --distro Ubuntu-24.04 --report target/qa/prompt-native.json
```

That rehearsal obtains real Bash/Zsh/Fish path frames through an independent
wire decoder, then runs real guest kubectl configuration changes, checks exact
namespace replacement/clearing, rejects invalid helper requests, repeats twenty
reads and removes its exact temporary fixture. The separate VT test checks
fragmented OSC decoding, replacement, clearing and cursor preservation.
Its report contains logical case
labels and timings, not private request arguments, credentials or host paths.
Helper process timings are separate from interactive input and frame latency.
Prompt replay benchmarks include shell startup, 1000 prompt callbacks and output
validation per sample; they are not a measurement of keystroke-to-pixel latency.
The same report separately records startup plus two validated prompts. Immediate
identity projection is benchmarked independently, with a literal username oracle.
Title-storm, all-input invalidation, intermediate publication, namespace retention,
stale operation and renderer pending-latch tests guard the loading behavior.
The Zsh regression verifies that the real encoder does not hash an unrelated
executable at the end of PATH; it does not rely on a machine-specific timeout.

See [ADR 0055](adr/0055-session-local-prompt-discovery.md) for the ownership boundary.
