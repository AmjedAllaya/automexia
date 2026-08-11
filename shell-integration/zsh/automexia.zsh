# Automexia Zsh/macOS/WSL shell integration. Metadata + prompt styling only.
# It defines no wrapper commands or aliases.
case "${TERM_PROGRAM:-}|${AUTOMEXIA_SHELL_INTEGRATION:-}|${WSLENV:-}" in
  Automexia*|*'|1|'*|*'AUTOMEXIA_SHELL_INTEGRATION/u'*) ;;
  *) return 0 ;;
esac

export COLORTERM=truecolor
export TERM_PROGRAM=Automexia
export AUTOMEXIA_SHELL_INTEGRATION=1

autoload -Uz add-zsh-hook

__automexia_set_user_var() {
  local name=$1 value=$2 encoded
  [[ -n $value ]] || return 0
  if (( $+commands[base64] )) && (( $+commands[tr] )); then
    # Portable across GNU/BSD base64 (including macOS).
    encoded=$(printf '%s' "$value" | base64 2>/dev/null | tr -d '\r\n') || return 0
    [[ -n $encoded ]] && printf '\e]1337;SetUserVar=%s=%s\a' "$name" "$encoded"
  fi
}

__automexia_publish_static_metadata() {
  local os_version=''
  if [[ -r /etc/os-release ]]; then
    source /etc/os-release
    os_version=${VERSION_ID:-}
  fi
  __automexia_set_user_var automexia_distro "${WSL_DISTRO_NAME:-}"
  __automexia_set_user_var automexia_os_version "$os_version"
  printf '\e]1337;SetUserVar=automexia_shell=MQ==\a'
}
__automexia_publish_static_metadata
unfunction __automexia_publish_static_metadata 2>/dev/null || true

__automexia_precmd() {
  printf '\e[0m\e]133;D\a'
  printf '\e]7;file://%s%s\a' "${HOST:-localhost}" "${PWD// /%20}"
  printf '\e]2;%s@%s: %s\a' "${USER:-user}" "${HOST:-host}" "$PWD"
}
__automexia_preexec() { printf '\e[0m\e]1337;SetUserVar=automexia_prompt_active=MA==\a\e]133;C\a'; }
add-zsh-hook precmd __automexia_precmd
add-zsh-hook preexec __automexia_preexec
setopt PROMPT_SUBST
PROMPT=$'%{\e]1337;SetUserVar=automexia_prompt_active=MQ==\a%}%{\e]133;A\a%}\n%{\e]133;P;k=c\a%}%F{blue}%3~%f %F{cyan}\xCE\xBB%f %{\e]133;B\a%}%F{magenta}'
