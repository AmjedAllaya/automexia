# Session-only startup for an ordinary interactive Bash launched by Automexia.
# Keep user rc side effects exactly once, including installed integration.
[[ $- == *i* ]] || return 0
# shellcheck disable=SC1090
[[ -r ~/.bashrc ]] && . ~/.bashrc
[[ ${AUTOMEXIA_SHELL_INTEGRATION:-} == 0 ]] && return 0
if ! declare -F __automexia_pre_prompt >/dev/null &&
   [[ -r ${AUTOMEXIA_SHELL_INTEGRATION_ROOT}/bash/automexia.bash ]]; then
  # shellcheck disable=SC1091
  . "${AUTOMEXIA_SHELL_INTEGRATION_ROOT}/bash/automexia.bash"
fi
