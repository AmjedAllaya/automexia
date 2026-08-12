#!/usr/bin/env zsh
set -eu

root=${0:A:h:h:h}
export TERM_PROGRAM=Automexia
source "$root/shell-integration/zsh/automexia.zsh" >/dev/null
source "$root/shell-integration/zsh/automexia.zsh" >/dev/null
__automexia_precmd >/dev/null

(( ${precmd_functions[(I)__automexia_precmd]} == 1 ))
(( ${preexec_functions[(I)__automexia_preexec]} == 1 ))
[[ $PROMPT == *$'\xCE\xBB'* ]]
[[ $PROMPT != *'%d'* ]]
grep -qF '__automexia_print_colored_path "$PWD"' "$root/shell-integration/zsh/automexia.zsh"
! grep -Eiq 'alias (docker|kubectl)=|function (ax|kgp)' "$root/shell-integration/zsh/automexia.zsh"
sample_path='/srv/cloud project/production'
expected_path=$'\e[38;2;88;113;141m/\e[38;2;98;176;255msrv\e[38;2;88;113;141m/\e[38;2;72;167;255mcloud project\e[38;2;88;113;141m/\e[38;2;45;212;191mproduction\e[0m'
[[ "$(__automexia_print_colored_path "$sample_path")" == "$expected_path" ]]
for category_color in \
  '*secret=1;38;5;203' '*config=38;5;214' '*logs=38;5;220' \
  '*src=38;5;81' '*docs=38;5;114' '*tests=38;5;177' \
  '*target=38;5;209' '*assets=38;5;211' '*data=38;5;105' \
  '*cache=38;5;245' '*infra=38;5;39' '*packaging=38;5;214'; do
  grep -qF "$category_color" "$root/shell-integration/zsh/automexia.zsh"
done
print 'PASS: Zsh integration is active, idempotent, UTF-8-safe, semantically path-colored, and command-neutral'
