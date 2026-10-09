# One-shot adapter after native Bash login startup. Compatible with Bash 3.2.
# A profile that replaces PROMPT_COMMAND owns that choice and may explicitly
# source automexia.bash. Never rewrite profiles or emulate /etc/profile here.
__automexia_login_bootstrap='builtin source "$AUTOMEXIA_SHELL_INTEGRATION_ROOT/bash/login-session.bash"; if declare -F __automexia_legacy_install >/dev/null; then __automexia_legacy_install; fi'
# shellcheck disable=SC2178 # Native Bash supports string and array prompt hooks.
case "$(declare -p PROMPT_COMMAND 2>/dev/null)" in
  'declare -a '*)
    for __automexia_login_index in "${!PROMPT_COMMAND[@]}"; do
      PROMPT_COMMAND[$__automexia_login_index]=${PROMPT_COMMAND[$__automexia_login_index]//"$__automexia_login_bootstrap"/:}
    done
    ;;
  *) PROMPT_COMMAND=${PROMPT_COMMAND//"$__automexia_login_bootstrap"/:} ;;
esac
unset __automexia_login_index __automexia_login_bootstrap
[[ $- == *i* && ${AUTOMEXIA_SHELL_INTEGRATION:-} != 0 ]] || return 0
if ! declare -F __automexia_pre_prompt >/dev/null &&
   [[ -r ${AUTOMEXIA_SHELL_INTEGRATION_ROOT}/bash/automexia.bash ]]; then
  # shellcheck disable=SC1091
  . "${AUTOMEXIA_SHELL_INTEGRATION_ROOT}/bash/automexia.bash"
  __automexia_pre_prompt
fi
