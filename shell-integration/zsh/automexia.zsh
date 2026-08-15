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
  __automexia_set_user_var automexia_shell_user "${USER:-}"
  __automexia_set_user_var automexia_shell_path "${commands[zsh]:-${SHELL:-}}"
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
    typeset -gx EZA_COLORS='reset:ur=38;5;81:uw=38;5;220:ux=38;5;114:ue=38;5;114:gr=38;5;81:gw=38;5;220:gx=38;5;114:tr=38;5;81:tw=38;5;220:tx=38;5;114:su=1;38;5;203:sf=1;38;5;203:sn=1;38;5;215:sb=38;5;180:uu=1;38;5;141:uR=1;38;5;203:un=38;5;177:gu=38;5;75:gR=38;5;203:gn=38;5;75:lc=38;5;245:lm=38;5;250:da=38;5;109:hd=1;38;5;117:xx=38;5;240:di=1;38;5;39:fi=38;5;252:ex=38;5;252:ln=38;5;45:or=1;38;5;203:pi=38;5;214:so=38;5;171:bd=38;5;214:cd=38;5;214:sp=38;5;214:mp=38;5;39:sc=38;5;252:bu=38;5;252:do=38;5;252:cr=38;5;203:co=38;5;214:tm=38;5;109:cm=38;5;109:im=38;5;171:vi=38;5;171:mu=38;5;171:lo=38;5;171:ga=38;5;114:gm=38;5;220:gd=38;5;203:gv=38;5;81:gt=38;5;215:gi=38;5;245:gc=1;38;5;203:*secret=1;38;5;203:*secrets=1;38;5;203:*private=1;38;5;203:*credentials=1;38;5;203:*vault=1;38;5;203:*.env=1;38;5;203:*.env.*=1;38;5;203:*.key=1;38;5;203:*.pem=1;38;5;203:*config=38;5;214:*configs=38;5;214:*settings=38;5;214:*log=38;5;220:*logs=38;5;220:*.log=38;5;220:*.trace=38;5;220:*apps=38;5;81:*src=38;5;81:*source=38;5;81:*lib=38;5;81:*librio*=38;5;81:*rio-*=38;5;81:*corcovado=38;5;81:*sugarloaf=38;5;81:*teletypewriter=38;5;81:*docs=38;5;114:*documentation=38;5;114:*guides=38;5;114:*test=38;5;177:*tests=38;5;177:*specs=38;5;177:*fuzz=38;5;177:*fixtures=38;5;177:*target=38;5;209:*build=38;5;209:*dist=38;5;209:*out=38;5;209:*changes=38;5;209:*coverage=38;5;209:*assets=38;5;211:*public=38;5;211:*static=38;5;211:*media=38;5;211:*images=38;5;211:*icons=38;5;211:*fonts=38;5;211:*node_modules=38;5;141:*vendor=38;5;141:*packages=38;5;141:*deps=38;5;141:*.cargo=38;5;141:*.git=38;5;177:*.github=38;5;177:*scripts=38;5;80:*tools=38;5;80:*shell-integration=38;5;80:*ci=38;5;80:*data=38;5;105:*db=38;5;105:*database=38;5;105:*storage=38;5;105:*migrations=38;5;105:*.db=38;5;105:*.sqlite=38;5;105:*.cache=38;5;245:*cache=38;5;245:*tmp=38;5;245:*temp=38;5;245:*backups=38;5;245:*infra=38;5;39:*infrastructure=38;5;39:*terraform=38;5;141:*k8s=38;5;39:*kubernetes=38;5;39:*helm=38;5;39:*docker=38;5;39:*packaging=38;5;214:*Dockerfile=38;5;39:*docker-compose*.yml=38;5;39:*docker-compose*.yaml=38;5;39'
  fi

  # Profiles commonly create an `ls` alias before this managed block. Clear
  # only the shortcuts Automexia intentionally replaces before functions are
  # parsed, avoiding alias expansion of the function names.
  unalias ls l ll la lA tree 2>/dev/null || true
  typeset -g __automexia_integration_dir=${${(%):-%N}:A:h}
  typeset -g __automexia_eza_filter_path="$__automexia_integration_dir/../posix/automexia-eza-filter.pl"
  [[ -r $__automexia_eza_filter_path ]] || \
    __automexia_eza_filter_path="$__automexia_integration_dir/automexia-eza-filter.pl"

  __automexia_run_eza() {
    if [[ -t 1 && -r $__automexia_eza_filter_path ]] && (( $+commands[perl] )); then
      # eza writes presentation bytes into the badge filter only for a real
      # terminal. Piped and redirected listings keep eza's plain semantics.
      command eza --icons=always --color=always --width="${COLUMNS:-80}" "$@" |
        perl -CS "$__automexia_eza_filter_path"
      local eza_status=${pipestatus[1]}
      return $eza_status
    fi
    command eza --icons=auto --color=auto "$@"
  }

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
      __automexia_run_eza --group-directories-first \
        --header --group --time-style=long-iso "$@"
    else
      __automexia_run_eza --group-directories-first "$@"
    fi
  }
  function ls { __automexia_eza "$@"; }
  function l { __automexia_eza -l "$@"; }
  function ll { __automexia_eza -lah --git "$@"; }
  function la { __automexia_eza -la "$@"; }
  function lA { __automexia_eza -lA "$@"; }
  function tree { __automexia_run_eza --tree "$@"; }
fi

__automexia_print_colored_path() {
  local remaining=$1 component separators color parent_index=0
  local first_component=1
  local root_color=$'\e[38;2;98;176;255m'
  local parent_cyan=$'\e[38;2;80;213;255m'
  local parent_violet=$'\e[38;2;167;139;250m'
  local parent_blue=$'\e[38;2;72;167;255m'
  local leaf_color=$'\e[38;2;184;243;107m'
  local separator_color=$'\e[38;2;88;113;141m'
  local reset_color=$'\e[0m'

  # Keep the path formatter allocation- and subprocess-free on ZLE's hot path.
  while [[ -n $remaining ]]; do
    if [[ $remaining == /* ]]; then
      separators=${remaining%%[^/]*}
      printf '%s%s' "$separator_color" "$separators"
      remaining=${remaining#"$separators"}
      continue
    fi

    if [[ $remaining == */* ]]; then
      component=${remaining%%/*}
      if (( first_component )); then
        color=$root_color
      else
        case $((parent_index % 3)) in
          0) color=$parent_cyan ;;
          1) color=$parent_violet ;;
          *) color=$parent_blue ;;
        esac
        ((parent_index += 1))
      fi
    else
      component=$remaining
      color=$leaf_color
    fi
    printf '%s%s' "$color" "$component"
    first_component=0
    remaining=${remaining#"$component"}
  done
  printf '%s' "$reset_color"
}

__automexia_precmd() {
  # `status` is a read-only special parameter in Zsh; use a private name so
  # the hook works under both interactive Zsh and the non-interactive tests.
  local exit_status=$?
  printf '\e[0m\e]133;D;%s\a' "$exit_status"
  printf '\e]7;file://%s%s\a' "${HOST:-localhost}" "${PWD// /%20}"
  printf '\e]2;%s@%s: %s\a' "${USER:-user}" "${HOST:-host}" "$PWD"
  ((__automexia_prompt_generation += 1))
  __automexia_prompt_is_active=1
  # Automexia owns the stable context spacer and complete path rows. ZLE owns
  # only the lambda, editable command, and cursor row, so its delayed SIGWINCH
  # repaint cannot erase or duplicate the terminal-owned path.
  printf '\e]1337;SetUserVar=automexia_prompt_active=MQ==\a\e]133;A;aid=%s\a \n' \
    "$__automexia_prompt_generation"
  printf '\e]133;P;k=c;aid=%s\a' "$__automexia_prompt_generation"
  __automexia_print_colored_path "$PWD"
  printf '\n'
  printf '\e]133;P;k=c;aid=%s\a' "$__automexia_prompt_generation"
  PROMPT=$'%F{cyan}\xCE\xBB%f %{\e]133;B\a%}%F{white}'
}
__automexia_preexec() {
  __automexia_prompt_is_active=0
  printf '\e[0m\e]1337;SetUserVar=automexia_prompt_active=MA==\a\e]133;C\a'
}
add-zsh-hook precmd __automexia_precmd
add-zsh-hook preexec __automexia_preexec
setopt PROMPT_SUBST

# `precmd` emits the stable context and complete path rows. ZLE owns only the
# editable lambda row.
PROMPT=''
