#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
export TERM_PROGRAM=Automexia
PROMPT_COMMAND='printf user-hook >/dev/null'

# Use a controlled eza stand-in so this test proves alias behavior without
# making the contributor machine install an optional presentation tool.
fixture_bin=$(mktemp -d)
trap 'rm -rf "$fixture_bin"' EXIT
export AUTOMEXIA_CONFIG_HOME="$fixture_bin/config"
completion_root="$AUTOMEXIA_CONFIG_HOME/generated/completion/bash"
mkdir -p "$completion_root"
printf '%s\n' 'complete -W "managed-candidate" kubectl' >"$completion_root/kubectl.bash"
kubectl_digest=$(sha256sum "$completion_root/kubectl.bash" | awk '{print $1}')
# Simulate an interrupted refresh: the candidate digest may be the second entry.
printf '%064d\n%s\n' 0 "$kubectl_digest" >"$completion_root/kubectl.bash.sha256"
printf '%s\n' 'complete -W "must-not-load" docker' >"$completion_root/docker.bash"
sha256sum "$completion_root/docker.bash" | awk '{print $1}' >"$completion_root/docker.bash.sha256"
complete -W 'native-candidate' docker
cat >"$fixture_bin/eza" <<'EOF'
#!/usr/bin/env sh
printf '%s\n' "$*"
EOF
chmod +x "$fixture_bin/eza"
PATH="$fixture_bin:$PATH"
alias_generation=$(
  python3 "$root/tools/ci/create_cp31_alias_fixture.py" \
    --config-root "$AUTOMEXIA_CONFIG_HOME" --alias axt --value first
)
[[ $alias_generation =~ ^[0-9a-f]{64}$ ]]
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
complete -p kubectl | grep -qF 'managed-candidate'
complete -p docker | grep -qF 'native-candidate'
! complete -p docker | grep -qF 'must-not-load'
automexia_completion_health | grep -qF 'loaded=kubectl'
automexia_completion_health | grep -qF 'collisions=docker'
automexia_aliases_health | grep -qF 'state=ready'
[[ $(axt) == first ]]

alias_samples=''
for iteration in {1..25}; do
  sample=$(
    TIMEFORMAT='%R'
    { time automexia_aliases_reload >/dev/null; } 2>&1
  )
  (( iteration > 5 )) && alias_samples+="$sample"$'\n'
done
bash_alias_reload_p95=$(printf '%s' "$alias_samples" | sort -n | sed -n '19p')
awk -v p95="$bash_alias_reload_p95" 'BEGIN { exit !(p95 <= 0.050) }'

second_alias_generation=$(
  python3 "$root/tools/ci/create_cp31_alias_fixture.py" \
    --config-root "$AUTOMEXIA_CONFIG_HOME" --alias ayt --value second
)
[[ $second_alias_generation != "$alias_generation" ]]
ayt() { printf '%s' native; }
if automexia_aliases_reload; then exit 1; fi
[[ $(axt) == first ]]
[[ $(ayt) == native ]]
automexia_aliases_health | grep -qF 'state=collision'
unset -f ayt
automexia_aliases_reload
[[ -z $(type -t axt 2>/dev/null || true) ]]
[[ $(ayt) == second ]]
wrong_compiler_generation=$(
  python3 "$root/tools/ci/create_cp31_alias_fixture.py" \
    --config-root "$AUTOMEXIA_CONFIG_HOME" --alias azt --value wrong \
    --generator automexia-devops/999.0.0
)
[[ $wrong_compiler_generation != "$second_alias_generation" ]]
if automexia_aliases_reload; then exit 1; fi
[[ $(ayt) == second ]]
[[ -z $(type -t azt 2>/dev/null || true) ]]
automexia_aliases_health | grep -qF 'state=tampered'
printf '%s\n' "$second_alias_generation" >"$AUTOMEXIA_CONFIG_HOME/generated/aliases/current"
chmod 600 "$AUTOMEXIA_CONFIG_HOME/generated/aliases/current"

current_alias_generation=$(<"$AUTOMEXIA_CONFIG_HOME/generated/aliases/current")
printf '%s\n' '# tampered' >>"$AUTOMEXIA_CONFIG_HOME/generated/aliases/generations/$current_alias_generation/bash/automexia-aliases.bash"
if automexia_aliases_reload; then exit 1; fi
[[ $(ayt) == second ]]
automexia_aliases_health | grep -qF 'state=tampered'
printf '%s\n' disabled >"$AUTOMEXIA_CONFIG_HOME/generated/aliases/current"
chmod 600 "$AUTOMEXIA_CONFIG_HOME/generated/aliases/current"
AUTOMEXIA_TEST_ROOT="$root" bash --noprofile --norc -c '
  shopt -s expand_aliases
  source "$AUTOMEXIA_TEST_ROOT/shell-integration/bash/automexia.bash" >/dev/null
  [[ -z $(type -t ayt 2>/dev/null || true) ]]
  automexia_aliases_health | grep -qF "state=disabled"
'

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

unset AUTOMEXIA_COMPLETION_ADAPTER_BASH_LOADED
export AUTOMEXIA_COMPLETION_DISABLED=1
complete -r kubectl
source "$root/shell-integration/completion/bash/automexia-completion.bash"
! complete -p kubectl >/dev/null 2>&1
automexia_completion_health | grep -qF 'state=disabled'

adapter_samples=''
for iteration in {1..25}; do
  sample=$(
    unset AUTOMEXIA_COMPLETION_ADAPTER_BASH_LOADED AUTOMEXIA_COMPLETION_DISABLED
    complete -r kubectl 2>/dev/null || true
    TIMEFORMAT='%R'
    { time source "$root/shell-integration/completion/bash/automexia-completion.bash" >/dev/null; } 2>&1
  )
  (( iteration > 5 )) && adapter_samples+="$sample"$'\n'
done
bash_adapter_p95=$(printf '%s' "$adapter_samples" | sort -n | sed -n '19p')
awk -v p95="$bash_adapter_p95" 'BEGIN { exit !(p95 <= 0.050) }'

unset AUTOMEXIA_COMPLETION_ADAPTER_BASH_LOADED AUTOMEXIA_COMPLETION_DISABLED
complete -r kubectl 2>/dev/null || true
unsafe_completion="$fixture_bin/unsafe-bash"
mkdir -p "$unsafe_completion"
printf '%s\n' 'complete -W "linked-candidate" kubectl' >"$unsafe_completion/kubectl.bash"
sha256sum "$unsafe_completion/kubectl.bash" | awk '{print $1}' >"$unsafe_completion/kubectl.bash.sha256"
rm -rf -- "$completion_root"
ln -s "$unsafe_completion" "$completion_root"
source "$root/shell-integration/completion/bash/automexia-completion.bash"
! complete -p kubectl >/dev/null 2>&1
automexia_completion_health | grep -qF 'state=unsafe-path/native-fallback'

unset AUTOMEXIA_COMPLETION_ADAPTER_BASH_LOADED
export AUTOMEXIA_CONFIG_HOME=relative-config-root
source "$root/shell-integration/completion/bash/automexia-completion.bash"
automexia_completion_health | grep -qF 'state=unsafe-path/native-fallback'

# The declared path ceiling must fail before any filesystem probe.
unset AUTOMEXIA_COMPLETION_ADAPTER_BASH_LOADED
printf -v overlong_root '%*s' 4097 ''
overlong_root=${overlong_root// /x}
export AUTOMEXIA_CONFIG_HOME="/$overlong_root"
source "$root/shell-integration/completion/bash/automexia-completion.bash"
automexia_completion_health | grep -qF 'state=unsafe-path/native-fallback'

echo "PASS: Bash integration is prompt-safe, native-first, digest-verified, linked-parent-safe, disable-safe, idempotent, adapter-p95=${bash_adapter_p95}s, alias-reload-p95=${bash_alias_reload_p95}s, and readable icon-listing aware"
