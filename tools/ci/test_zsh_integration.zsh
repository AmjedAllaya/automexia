#!/usr/bin/env zsh
set -eu

root=${0:A:h:h:h}
export TERM_PROGRAM=Automexia
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
export AUTOMEXIA_CONFIG_HOME="$fixture/config"
completion_root="$AUTOMEXIA_CONFIG_HOME/generated/completion/zsh"
mkdir -p "$completion_root"
print -r -- 'compdef _gnu_generic kubectl' >"$completion_root/kubectl.zsh"
kubectl_digest=$(sha256sum "$completion_root/kubectl.zsh" | awk '{print $1}')
# Simulate an interrupted refresh: the candidate digest may be the second entry.
printf '%064d\n%s\n' 0 "$kubectl_digest" >"$completion_root/kubectl.zsh.sha256"
print -r -- 'compdef _gnu_generic docker' >"$completion_root/docker.zsh"
sha256sum "$completion_root/docker.zsh" | awk '{print $1}' >"$completion_root/docker.zsh.sha256"
autoload -Uz compinit
compinit -D
compdef _files docker
alias_generation=$(
  python3 "$root/tools/ci/create_cp31_alias_fixture.py" \
    --config-root "$AUTOMEXIA_CONFIG_HOME" --alias axt --value first
)
[[ $alias_generation =~ '^[0-9a-f]{64}$' ]]
source "$root/shell-integration/zsh/automexia.zsh" >/dev/null
source "$root/shell-integration/zsh/automexia.zsh" >/dev/null
__automexia_precmd >/dev/null

(( ${precmd_functions[(I)__automexia_precmd]} == 1 ))
(( ${preexec_functions[(I)__automexia_preexec]} == 1 ))
[[ $PROMPT == *$'\xCE\xBB'* ]]
[[ $PROMPT != *'%d'* ]]
failure_marker=$(set +e; false; __automexia_precmd)
[[ $failure_marker == *$'\e]133;D;1\a'* ]]
[[ $failure_marker == *$'\e]133;A;aid='* ]]
preexec_marker=$(__automexia_preexec)
[[ $preexec_marker == *$'\e]133;C\a'* ]]
[[ $preexec_marker == *$'automexia_prompt_active=MA=='* ]]
grep -qF '__automexia_print_colored_path "$PWD"' "$root/shell-integration/zsh/automexia.zsh"
! grep -Eiq 'alias (docker|kubectl)=|function (ax|kgp)' "$root/shell-integration/zsh/automexia.zsh"
sample_path='/srv/cloud project/region/production'
expected_path=$'\e[38;2;88;113;141m/\e[38;2;98;176;255msrv\e[38;2;88;113;141m/\e[38;2;80;213;255mcloud project\e[38;2;88;113;141m/\e[38;2;167;139;250mregion\e[38;2;88;113;141m/\e[38;2;184;243;107mproduction\e[0m'
[[ "$(__automexia_print_colored_path "$sample_path")" == "$expected_path" ]]
automexia_aliases_health | grep -qF 'state=ready'
[[ $(axt) == first ]]

zmodload zsh/datetime
typeset -a alias_samples
for iteration in {1..25}; do
  started=$EPOCHREALTIME
  automexia_aliases_reload >/dev/null
  elapsed=$(( EPOCHREALTIME - started ))
  (( iteration > 5 )) && alias_samples+=("$elapsed")
done
zsh_alias_reload_p95=$(printf '%s\n' "${alias_samples[@]}" | sort -n | sed -n '19p')
(( zsh_alias_reload_p95 <= 0.050 ))

second_alias_generation=$(
  python3 "$root/tools/ci/create_cp31_alias_fixture.py" \
    --config-root "$AUTOMEXIA_CONFIG_HOME" --alias ayt --value second
)
[[ $second_alias_generation != "$alias_generation" ]]
function ayt { printf '%s' native }
if automexia_aliases_reload; then exit 1; fi
[[ $(axt) == first ]]
[[ $(ayt) == native ]]
automexia_aliases_health | grep -qF 'state=collision'
unfunction ayt
automexia_aliases_reload
(( ! ${+aliases[axt]} ))
[[ $(ayt) == second ]]
current_alias_generation=$(<"$AUTOMEXIA_CONFIG_HOME/generated/aliases/current")
printf '%s\n' '# tampered' >>"$AUTOMEXIA_CONFIG_HOME/generated/aliases/generations/$current_alias_generation/zsh/automexia-aliases.zsh"
if automexia_aliases_reload; then exit 1; fi
[[ $(ayt) == second ]]
automexia_aliases_health | grep -qF 'state=tampered'
printf '%s\n' disabled >"$AUTOMEXIA_CONFIG_HOME/generated/aliases/current"
chmod 600 "$AUTOMEXIA_CONFIG_HOME/generated/aliases/current"
AUTOMEXIA_TEST_ROOT="$root" zsh -f -c '
  source "$AUTOMEXIA_TEST_ROOT/shell-integration/zsh/automexia.zsh" >/dev/null
  (( ! ${+aliases[ayt]} ))
  automexia_aliases_health | grep -qF "state=disabled"
'

for category_color in \
  '*secret=1;38;5;203' '*config=38;5;214' '*logs=38;5;220' \
  '*src=38;5;81' '*docs=38;5;114' '*tests=38;5;177' \
  '*target=38;5;209' '*assets=38;5;211' '*data=38;5;105' \
  '*cache=38;5;245' '*infra=38;5;39' '*packaging=38;5;214'; do
  grep -qF "$category_color" "$root/shell-integration/zsh/automexia.zsh"
done
grep -qF 'automexia-eza-filter.pl' "$root/shell-integration/zsh/automexia.zsh"
[[ $_comps[kubectl] == _gnu_generic ]]
[[ $_comps[docker] == _files ]]
[[ $__automexia_completion_loaded == kubectl ]]
[[ $__automexia_completion_collisions == docker ]]
unset AUTOMEXIA_COMPLETION_ADAPTER_ZSH_LOADED
export AUTOMEXIA_COMPLETION_DISABLED=1
unset '_comps[kubectl]'
source "$root/shell-integration/completion/zsh/automexia-completion.zsh"
(( ! ${+_comps[kubectl]} ))
automexia_completion_health | grep -qF 'state=disabled'

zmodload zsh/datetime
typeset -a adapter_samples
for iteration in {1..25}; do
  unset AUTOMEXIA_COMPLETION_ADAPTER_ZSH_LOADED AUTOMEXIA_COMPLETION_DISABLED
  unset '_comps[kubectl]'
  started=$EPOCHREALTIME
  source "$root/shell-integration/completion/zsh/automexia-completion.zsh" >/dev/null
  elapsed=$(( EPOCHREALTIME - started ))
  (( iteration > 5 )) && adapter_samples+=("$elapsed")
done
zsh_adapter_p95=$(printf '%s\n' "${adapter_samples[@]}" | sort -n | sed -n '19p')
(( zsh_adapter_p95 <= 0.050 ))

unset AUTOMEXIA_COMPLETION_ADAPTER_ZSH_LOADED AUTOMEXIA_COMPLETION_DISABLED
unset '_comps[kubectl]'
unsafe_completion="$fixture/unsafe-zsh"
mkdir -p "$unsafe_completion"
print -r -- 'compdef _gnu_generic kubectl' >"$unsafe_completion/kubectl.zsh"
sha256sum "$unsafe_completion/kubectl.zsh" | awk '{print $1}' >"$unsafe_completion/kubectl.zsh.sha256"
rm -rf -- "$completion_root"
ln -s "$unsafe_completion" "$completion_root"
source "$root/shell-integration/completion/zsh/automexia-completion.zsh"
(( ! ${+_comps[kubectl]} ))
automexia_completion_health | grep -qF 'state=unsafe-path/native-fallback'

unset AUTOMEXIA_COMPLETION_ADAPTER_ZSH_LOADED
export AUTOMEXIA_CONFIG_HOME=relative-config-root
source "$root/shell-integration/completion/zsh/automexia-completion.zsh"
automexia_completion_health | grep -qF 'state=unsafe-path/native-fallback'

# The declared path ceiling must fail before any filesystem probe.
unset AUTOMEXIA_COMPLETION_ADAPTER_ZSH_LOADED
printf -v overlong_root '%*s' 4097 ''
overlong_root=${overlong_root// /x}
export AUTOMEXIA_CONFIG_HOME="/$overlong_root"
source "$root/shell-integration/completion/zsh/automexia-completion.zsh"
automexia_completion_health | grep -qF 'state=unsafe-path/native-fallback'
print "PASS: Zsh integration is prompt-safe, native-first, digest-verified, linked-parent-safe, disable-safe, idempotent, adapter-p95=${zsh_adapter_p95}s, alias-reload-p95=${zsh_alias_reload_p95}s, and command-neutral"
