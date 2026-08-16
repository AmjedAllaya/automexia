#!/usr/bin/env fish

set root (path resolve (path dirname (status filename))/../..)
set fixture (mktemp -d)
function cleanup --on-event fish_exit
    rm -rf -- "$fixture"
end
set -gx TERM_PROGRAM Automexia
set -gx AUTOMEXIA_CONFIG_HOME "$fixture/config"
mkdir -p "$AUTOMEXIA_CONFIG_HOME/generated/completion/fish"

set artifact "$AUTOMEXIA_CONFIG_HOME/generated/completion/fish/kubectl.fish"
printf '%s\n' 'complete -c kubectl -a managed-candidate' >"$artifact"
command sha256sum "$artifact" | read -l digest remainder; or exit 1
printf '%s\n' "$digest" >"$artifact.sha256"
source "$root/shell-integration/fish/automexia.fish" >/dev/null; or exit 1
set health (automexia_completion_health)
string match -q '*shell=fish*' -- "$health"; or exit 1
string match -q '*loaded=kubectl*' -- "$health"; or exit 1
string match -q '*managed-candidate*' -- (complete -c kubectl); or exit 1

# fish_postexec exposes the completed process status through $status; its event
# argument is the command line. A failing command must not be reported as a
# successful semantic prompt generation.
set failed_marker (begin; false; __automexia_fish_postexec; end | string escape)
string match -q '*133\\;D\\;1*' -- "$failed_marker"; or exit 1

# A native definition must prevent managed replacement in a clean adapter
# generation, and disabled mode must leave all native state untouched.
set -e AUTOMEXIA_COMPLETION_ADAPTER_FISH_LOADED
set -e __automexia_completion_loaded
set -e __automexia_completion_collisions
complete -e -c kubectl
complete -c kubectl -a native-candidate
source "$root/shell-integration/completion/fish/automexia-completion.fish"; or exit 1
string match -q '*native-candidate*' -- (complete -c kubectl); or exit 1
not string match -q '*managed-candidate*' -- (complete -c kubectl); or exit 1
string match -q '*collisions=kubectl*' -- (automexia_completion_health); or exit 1

set -gx AUTOMEXIA_COMPLETION_DISABLED 1
set -e AUTOMEXIA_COMPLETION_ADAPTER_FISH_LOADED
source "$root/shell-integration/completion/fish/automexia-completion.fish"; or exit 1
string match -q '*state=disabled*' -- (automexia_completion_health); or exit 1

set fish_adapter_benchmark (python3 "$root/tools/ci/measure_completion_adapter.py" --shell fish); or exit 1

set -e AUTOMEXIA_COMPLETION_ADAPTER_FISH_LOADED AUTOMEXIA_COMPLETION_DISABLED
complete -e -c kubectl
set unsafe_completion "$fixture/unsafe-fish"
mkdir -p "$unsafe_completion"
printf '%s\n' 'complete -c kubectl -a linked-candidate' >"$unsafe_completion/kubectl.fish"
command sha256sum "$unsafe_completion/kubectl.fish" | read -l unsafe_digest remainder; or exit 1
printf '%s\n' "$unsafe_digest" >"$unsafe_completion/kubectl.fish.sha256"
rm -rf -- "$AUTOMEXIA_CONFIG_HOME/generated/completion/fish"
ln -s "$unsafe_completion" "$AUTOMEXIA_CONFIG_HOME/generated/completion/fish"
source "$root/shell-integration/completion/fish/automexia-completion.fish"; or exit 1
not string match -q '*linked-candidate*' -- (complete -c kubectl); or exit 1
string match -q '*state=unsafe-path/native-fallback*' -- (automexia_completion_health); or exit 1

set -e AUTOMEXIA_COMPLETION_ADAPTER_FISH_LOADED
set -gx AUTOMEXIA_CONFIG_HOME relative-config-root
source "$root/shell-integration/completion/fish/automexia-completion.fish"; or exit 1
string match -q '*state=unsafe-path/native-fallback*' -- (automexia_completion_health); or exit 1

echo "PASS: Fish integration is syntax-valid, editor-owned, digest-verified, linked-parent-safe, native-first, disable-safe, $fish_adapter_benchmark, and health-reporting"
