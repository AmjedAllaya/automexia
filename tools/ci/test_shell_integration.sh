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
if grep -qF 'PROMPT_DIRTRIM' "$root/shell-integration/bash/automexia.bash"; then exit 1; fi
if grep -qF '__automexia_git_segment' "$root/shell-integration/bash/automexia.bash"; then exit 1; fi
grep -qF '133;A;aid=%s\a \n' "$root/shell-integration/bash/automexia.bash"
grep -qF '133;P;k=c;aid=%s\a' "$root/shell-integration/bash/automexia.bash"
# shellcheck disable=SC2016 # Search for the literal integration contract.
grep -qF '__automexia_print_colored_path "$PWD"' "$root/shell-integration/bash/automexia.bash"
[[ $PS1 != *'PWD'* ]]
grep -qF '__automexia_prompt_is_active=0' "$root/shell-integration/bash/automexia.bash"
if grep -Eiq 'alias (docker|kubectl)=|function (ax|kgp)' "$root/shell-integration/bash/automexia.bash"; then exit 1; fi
sample_path='/srv/cloud project/region/production'
expected_path=$'\e[38;2;88;113;141m/\e[38;2;98;176;255msrv\e[38;2;88;113;141m/\e[38;2;80;213;255mcloud project\e[38;2;88;113;141m/\e[38;2;167;139;250mregion\e[38;2;88;113;141m/\e[38;2;184;243;107mproduction\e[0m'
[[ $(__automexia_print_colored_path "$sample_path") == "$expected_path" ]]
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
for category_color in \
  '*secret=1;38;5;203' '*config=38;5;214' '*logs=38;5;220' \
  '*src=38;5;81' '*docs=38;5;114' '*tests=38;5;177' \
  '*target=38;5;209' '*assets=38;5;211' '*data=38;5;105' \
  '*cache=38;5;245' '*infra=38;5;39' '*packaging=38;5;214'; do
  [[ $EZA_COLORS == *"$category_color"* ]]
done

badge_filter="$root/shell-integration/posix/automexia-eza-filter.pl"
[[ -r $badge_filter ]]
badge_input=$'\e[38;5;39m\uE5FF \e[1mconfig\e[0m  \e[38;5;39m\uE5FF \e[1mrio-vt\e[0m  \e[38;5;39m\uE5FF \e[1mordinary\e[0m'
badge_output=$(printf '%s\n' "$badge_input" | perl -CS "$badge_filter")
[[ $badge_output == *$'\U000F107F \e[1mconfig'* ]]
[[ $badge_output == *$'\U000F19F6 \e[1mrio-vt'* ]]
[[ $badge_output == *$'\uE5FF \e[1mordinary'* ]]
[[ $badge_output == *$'\e[38;2;255;176;32m'* ]]
[[ $badge_output == *$'\e[38;2;80;213;255m'* ]]

unset -f ls l ll la lA tree __automexia_eza __automexia_run_eza
export AUTOMEXIA_PLAIN_LS=1
# shellcheck source=/dev/null
source "$root/shell-integration/bash/automexia.bash" >/dev/null
[[ $(type -t ls) != function ]]

echo 'PASS: Bash integration is active, idempotent, status-preserving, UTF-8-safe, semantically path-colored, composite-folder-aware, three-row prompt-identified, full-path, resize-safe, command-neutral, and readable icon-listing aware'
