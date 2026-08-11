#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
export TERM_PROGRAM=Automexia
PROMPT_COMMAND='printf user-hook >/dev/null'

# shellcheck source=/dev/null
source "$root/shell-integration/bash/automexia.bash" >/dev/null
# shellcheck source=/dev/null
source "$root/shell-integration/bash/automexia.bash" >/dev/null

count=$(grep -o '__automexia_pre_prompt' <<<"$PROMPT_COMMAND" | wc -l)
[[ $count -eq 1 ]]
[[ $PROMPT_COMMAND == printf\ user-hook* ]]

set +e
false
__automexia_pre_prompt >/dev/null
status=$?
set -e
[[ $status -eq 1 ]]

grep -qF "AUTOMEXIA_SHELL_INTEGRATION" "$root/shell-integration/bash/automexia.bash"
grep -qF '\xCE\xBB' "$root/shell-integration/bash/automexia.bash"
[[ $PS1 == *$'\u03BB'* ]]
! grep -Eiq 'alias (docker|kubectl)=|function (ax|kgp)' "$root/shell-integration/bash/automexia.bash"
echo 'PASS: Bash integration is active, idempotent, status-preserving, UTF-8-safe, and command-neutral'
