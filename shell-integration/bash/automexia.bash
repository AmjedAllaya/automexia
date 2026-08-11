# Automexia Bash/WSL shell integration. Prompt metadata + editor colors only.
# No wrapper commands or aliases are defined.

# WSLENV itself is inherited into WSL even on systems where an individual
# variable import is misconfigured. Treat the presence of Automexia's /u entry
# as a reliable activation hint, but remain inert outside Automexia.
case "${TERM_PROGRAM:-}|${AUTOMEXIA_SHELL_INTEGRATION:-}|${WSLENV:-}" in
  Automexia*|*'|1|'*|*'AUTOMEXIA_SHELL_INTEGRATION/u'*) ;;
  *) return 0 ;;
esac

export COLORTERM=truecolor
export TERM_PROGRAM=Automexia
export AUTOMEXIA_SHELL_INTEGRATION=1
# Keep long project paths readable without allowing the prompt to wrap early.
PROMPT_DIRTRIM=${PROMPT_DIRTRIM:-3}

# Publish distro/version once. OSC 1337 SetUserVar is metadata only; the
# terminal never executes it as a command. Metadata is encoded once at shell
# startup with portable base64 output normalization (GNU/BSD compatible).
__automexia_set_user_var() {
  local name=$1 value=$2 encoded
  [[ -n $value ]] || return 0
  if command -v base64 >/dev/null 2>&1 && command -v tr >/dev/null 2>&1; then
    # Portable across GNU/BSD base64: remove any implementation-specific
    # line wrapping instead of relying on GNU-only `-w0`.
    encoded=$(printf '%s' "$value" | base64 2>/dev/null | tr -d '\r\n') || return 0
    [[ -n $encoded ]] && printf '\e]1337;SetUserVar=%s=%s\a' "$name" "$encoded"
  fi
}

__automexia_publish_static_metadata() {
  local os_version=''
  if [[ -r /etc/os-release ]]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    os_version=${VERSION_ID:-}
  fi
  __automexia_set_user_var automexia_distro "${WSL_DISTRO_NAME:-}"
  __automexia_set_user_var automexia_os_version "$os_version"
  # base64("1") is constant; publish the activation marker without spawning
  # another encoder process on shell startup.
  printf '\e]1337;SetUserVar=automexia_shell=MQ==\a'
}
__automexia_publish_static_metadata
unset -f __automexia_publish_static_metadata

__automexia_osc7() {
  local p=${PWD// /%20}
  printf '\e]7;file://%s%s\a' "${HOSTNAME:-localhost}" "$p"
}

__automexia_title() {
  # Keep the terminal's raw title useful for passive WSL/session detection.
  printf '\e]2;%s@%s: %s\a' "${USER:-user}" "${HOSTNAME:-host}" "$PWD"
}

__automexia_pre_prompt() {
  # Preserve the status presented to subsequent prompt expansion/hooks. Because
  # this hook is appended after existing PROMPT_COMMAND entries, those hooks see
  # the real command status first; returning the captured status avoids turning
  # it into 0 just because Automexia emitted metadata.
  local status=$?
  printf '\e[0m\e]133;D\a'
  __automexia_osc7
  __automexia_title
  return "$status"
}

# Preserve a user's PROMPT_COMMAND rather than replacing it. Bash 5 may expose
# it as either a string or array; normalize only the common string case and do
# no external work on the prompt hot path.
case "$(declare -p PROMPT_COMMAND 2>/dev/null)" in
  'declare -a'*)
  __automexia_has_pc=0
  for __automexia_pc in "${PROMPT_COMMAND[@]}"; do
    [[ $__automexia_pc == __automexia_pre_prompt ]] && __automexia_has_pc=1
  done
  # Append rather than prepend: existing hooks see the real command `$?`
  # before Automexia emits metadata.
  (( __automexia_has_pc )) || PROMPT_COMMAND+=(__automexia_pre_prompt)
  unset __automexia_has_pc __automexia_pc
;;
  *)
  case ";${PROMPT_COMMAND:-};" in
    *';__automexia_pre_prompt;'*) ;;
    ';;') PROMPT_COMMAND='__automexia_pre_prompt' ;;
    *)
      # Avoid creating `;;` when an existing string hook already ends in a
      # semicolon. `eval "$PROMPT_COMMAND"` must remain valid Bash syntax.
      __automexia_pc_string=${PROMPT_COMMAND%';'}
      PROMPT_COMMAND="${__automexia_pc_string};__automexia_pre_prompt"
      unset __automexia_pc_string
      ;;
  esac
  ;;
esac

# Two semantic rows, matching Automexia's reference UX:
# 1. an intentionally blank Prompt row that the native renderer decorates with
#    Docker/Git/K8s/cloud context;
# 2. a PromptContinuation row containing U+03BB + editable command input.
# PS0 resets SGR before child output and emits OSC 133 C, so magenta command
# text cannot bleed into program output.
PS1='\[\e]1337;SetUserVar=automexia_prompt_active=MQ==\a\]\[\e]133;A\a\]\n\[\e]133;P;k=c\a\]\[\e[38;2;72;167;255m\]\w\[\e[0m\] \[\e[38;2;97;231;255m\]'$'\xCE\xBB''\[\e[0m\] \[\e]133;B\a\]\[\e[38;2;181;140;255m\]'
PS0='\[\e[0m\]\[\e]1337;SetUserVar=automexia_prompt_active=MA==\a\]\[\e]133;C\a\]'
