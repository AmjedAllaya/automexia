#!/usr/bin/env zsh
set -eu

root=${0:A:h:h:h}
export TERM_PROGRAM=Automexia
source "$root/shell-integration/zsh/automexia.zsh" >/dev/null
source "$root/shell-integration/zsh/automexia.zsh" >/dev/null

(( ${precmd_functions[(I)__automexia_precmd]} == 1 ))
(( ${preexec_functions[(I)__automexia_preexec]} == 1 ))
[[ $PROMPT == *$'\xCE\xBB'* ]]
! grep -Eiq 'alias (docker|kubectl)=|function (ax|kgp)' "$root/shell-integration/zsh/automexia.zsh"
print 'PASS: Zsh integration is active, idempotent, UTF-8-safe, and command-neutral'
