#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
export TERM_PROGRAM=Automexia
PROMPT_COMMAND='printf user-hook >/dev/null'

# Use a controlled eza stand-in so this test proves alias behavior without
# making the contributor machine install an optional presentation tool.
fixture_bin=$(mktemp -d)
trap 'rm -rf "$fixture_bin"' EXIT
cat >"$fixture_bin/eza" <<'EOF'
#!/usr/bin/env sh
printf '%s\n' "$*"
EOF
chmod +x "$fixture_bin/eza"
PATH="$fixture_bin:$PATH"
shopt -s expand_aliases
# Reproduce Ubuntu's default interactive profile. Defining a same-named shell
# function while this alias is active used to make Bash reject the integration.
alias ls='ls --color=auto'

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
[[ $PS1 != *'\w'* ]]
! grep -qF 'PROMPT_DIRTRIM' "$root/shell-integration/bash/automexia.bash"
! grep -qF '__automexia_git_segment' "$root/shell-integration/bash/automexia.bash"
grep -qF '133;A;aid=%s\a \n' "$root/shell-integration/bash/automexia.bash"
grep -qF '133;P;k=c;aid=%s\a' "$root/shell-integration/bash/automexia.bash"
grep -qF '38;2;72;167;255m%s' "$root/shell-integration/bash/automexia.bash"
[[ $PS1 != *'PWD'* ]]
grep -qF '__automexia_prompt_is_active=0' "$root/shell-integration/bash/automexia.bash"
! grep -Eiq 'alias (docker|kubectl)=|function (ax|kgp)' "$root/shell-integration/bash/automexia.bash"
[[ $(type -t ls) == function ]]
[[ $(type -t ll) == function ]]
[[ $(ls -ll) == '--icons=auto --color=auto --group-directories-first --header --group --time-style=long-iso -ll' ]]
[[ $(ll) == '--icons=auto --color=auto --group-directories-first --header --group --time-style=long-iso -lah --git' ]]
[[ $EZA_COLORS == reset:* ]]
[[ $EZA_COLORS == *'ur=38;5;81'* ]]
[[ $EZA_COLORS == *'uw=38;5;220'* ]]
[[ $EZA_COLORS == *'ux=38;5;114'* ]]
[[ $EZA_COLORS == *'hd=1;38;5;117'* ]]
[[ $EZA_COLORS == *'di=1;38;5;39'* ]]
[[ $EZA_COLORS == *'ex=38;5;252'* ]]

unset -f ls l ll la lA tree __automexia_eza
AUTOMEXIA_PLAIN_LS=1
# shellcheck source=/dev/null
source "$root/shell-integration/bash/automexia.bash" >/dev/null
[[ $(type -t ls) != function ]]

echo 'PASS: Bash integration is active, idempotent, status-preserving, UTF-8-safe, three-row prompt-identified, full-path, resize-safe, command-neutral, and readable icon-listing aware'
