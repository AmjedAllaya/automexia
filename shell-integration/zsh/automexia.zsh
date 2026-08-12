# Automexia Zsh/macOS/WSL shell integration. Metadata, prompt styling, and an
# optional icon-aware directory-listing experience.
case "${TERM_PROGRAM:-}|${AUTOMEXIA_SHELL_INTEGRATION:-}|${WSLENV:-}" in
  Automexia*|*'|1|'*|*'AUTOMEXIA_SHELL_INTEGRATION/u'*) ;;
  *) return 0 ;;
esac

[[ -n ${AUTOMEXIA_ZSH_INTEGRATION_LOADED:-} ]] && return 0
typeset -gx AUTOMEXIA_ZSH_INTEGRATION_LOADED=1

export COLORTERM=truecolor
export TERM_PROGRAM=Automexia
export AUTOMEXIA_SHELL_INTEGRATION=1
typeset -gi __automexia_prompt_generation=${__automexia_prompt_generation:-0}
typeset -gi __automexia_prompt_is_active=0

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
  printf '\e]1337;SetUserVar=automexia_shell_name=enNo\a'
}
__automexia_publish_static_metadata
unfunction __automexia_publish_static_metadata 2>/dev/null || true

# Let eza emit the file-type glyphs seen in the liquid-hacker mockup. Keeping
# this at the shell layer preserves the terminal's PTY contract: Automexia does
# not guess which pieces of arbitrary command output happen to be filenames.
# Long listings gain a labeled, color-separated table only on the interactive
# path. `command ls` bypasses it, and AUTOMEXIA_PLAIN_LS=1 keeps stock commands.
if [[ ${AUTOMEXIA_PLAIN_LS:-0} != 1 ]] && (( $+commands[eza] )); then
  # Each metadata column has a stable visual role: cyan read bits, gold write
  # bits, green execute bits, violet ownership, blue groups, orange sizes and
  # muted teal dates. DrvFs executable filenames remain neutral because WSL
  # commonly marks every Windows-hosted file executable. User colors win.
  if (( ! ${+EZA_COLORS} )); then
    typeset -gx EZA_COLORS='reset:ur=38;5;81:uw=38;5;220:ux=38;5;114:ue=38;5;114:gr=38;5;81:gw=38;5;220:gx=38;5;114:tr=38;5;81:tw=38;5;220:tx=38;5;114:su=1;38;5;203:sf=1;38;5;203:sn=1;38;5;215:sb=38;5;180:uu=1;38;5;141:uR=1;38;5;203:un=38;5;177:gu=38;5;75:gR=38;5;203:gn=38;5;75:lc=38;5;245:lm=38;5;250:da=38;5;109:hd=1;38;5;117:xx=38;5;240:di=1;38;5;39:fi=38;5;252:ex=38;5;252:ln=38;5;45:or=1;38;5;203:pi=38;5;214:so=38;5;171:bd=38;5;214:cd=38;5;214:sp=38;5;214:mp=38;5;39:sc=38;5;252:bu=38;5;252:do=38;5;252:cr=38;5;203:co=38;5;214:tm=38;5;109:cm=38;5;109:im=38;5;171:vi=38;5;171:mu=38;5;171:lo=38;5;171:ga=38;5;114:gm=38;5;220:gd=38;5;203:gv=38;5;81:gt=38;5;215:gi=38;5;245:gc=1;38;5;203:*Dockerfile=38;5;39:*docker-compose*.yml=38;5;39:*docker-compose*.yaml=38;5;39'
  fi

  # Profiles commonly create an `ls` alias before this managed block. Clear
  # only the shortcuts Automexia intentionally replaces before functions are
  # parsed, avoiding alias expansion of the function names.
  unalias ls l ll la lA tree 2>/dev/null || true
  __automexia_eza() {
    local long_view=0 argument
    for argument in "$@"; do
      case $argument in
        --) break ;;
        --long) long_view=1 ;;
        --*) ;;
        -*) [[ ${argument#-} == *l* ]] && long_view=1 ;;
      esac
    done

    if (( long_view )); then
      command eza --icons=auto --color=auto --group-directories-first \
        --header --group --time-style=long-iso "$@"
    else
      command eza --icons=auto --color=auto --group-directories-first "$@"
    fi
  }
  function ls { __automexia_eza "$@"; }
  function l { __automexia_eza -l "$@"; }
  function ll { __automexia_eza -lah --git "$@"; }
  function la { __automexia_eza -la "$@"; }
  function lA { __automexia_eza -lA "$@"; }
  function tree { command eza --tree --icons=auto --color=auto "$@"; }
fi

__automexia_precmd() {
  local status=$?
  printf '\e[0m\e]133;D;%s\a' "$status"
  printf '\e]7;file://%s%s\a' "${HOST:-localhost}" "${PWD// /%20}"
  printf '\e]2;%s@%s: %s\a' "${USER:-user}" "${HOST:-host}" "$PWD"
  ((__automexia_prompt_generation += 1))
  __automexia_prompt_is_active=1
  # Only the renderer-owned context row is outside ZLE. `%d` is Zsh's full,
  # unabridged current directory, and keeping it in the multiline PROMPT lets
  # ZLE restore the whole path after every SIGWINCH redisplay.
  printf '\e]1337;SetUserVar=automexia_prompt_active=MQ==\a\e]133;A;aid=%s\a \n' \
    "$__automexia_prompt_generation"
  printf -v PROMPT '%%{\e]133;P;k=c;aid=%s\a%%}%%F{#48A7FF}%%d%%f\n%%{\e]133;P;k=c;aid=%s\a%%}%%F{cyan}' \
    "$__automexia_prompt_generation" "$__automexia_prompt_generation"
  PROMPT+=$'\xCE\xBB%f %{\e]133;B\a%}%F{white}'
}
__automexia_preexec() {
  __automexia_prompt_is_active=0
  printf '\e[0m\e]1337;SetUserVar=automexia_prompt_active=MA==\a\e]133;C\a'
}
add-zsh-hook precmd __automexia_precmd
add-zsh-hook preexec __automexia_preexec
setopt PROMPT_SUBST

# `precmd` emits the stable context row and builds the resize-owned multiline
# ZLE prompt. The full `%d` path is never abbreviated.
PROMPT=''
