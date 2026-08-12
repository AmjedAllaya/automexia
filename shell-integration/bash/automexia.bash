# Automexia Bash/WSL shell integration. Prompt metadata, editor colors, and
# an optional icon-aware directory-listing experience.

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
__automexia_prompt_generation=${__automexia_prompt_generation:-0}
__automexia_prompt_is_active=0

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
  __automexia_set_user_var automexia_shell_user "${USER:-}"
  __automexia_set_user_var automexia_shell_path "${BASH:-$(command -v bash 2>/dev/null)}"
  # base64("1") is constant; publish the activation marker without spawning
  # another encoder process on shell startup.
  printf '\e]1337;SetUserVar=automexia_shell=MQ==\a'
  printf '\e]1337;SetUserVar=automexia_shell_name=YmFzaA==\a'
}
__automexia_publish_static_metadata
unset -f __automexia_publish_static_metadata

# Match the liquid-hacker reference experience without parsing or rewriting
# terminal output. eza owns the listing and emits Nerd Font codepoints before
# the bytes reach the PTY, which keeps copy/paste, selection, pipes, and
# scrollback honest. Long listings gain a labeled, color-separated table only
# on the interactive path. `command ls` remains an explicit escape hatch, and
# AUTOMEXIA_PLAIN_LS=1 disables the presentation layer before this file loads.
if [[ ${AUTOMEXIA_PLAIN_LS:-0} != 1 ]] && command -v eza >/dev/null 2>&1; then
  # Each metadata column has a stable visual role: cyan read bits, gold write
  # bits, green execute bits, violet ownership, blue groups, orange sizes and
  # muted teal dates. DrvFs executable filenames remain neutral because WSL
  # commonly marks every Windows-hosted file executable. User colors win.
  if [[ -z ${EZA_COLORS+x} ]]; then
    export EZA_COLORS='reset:ur=38;5;81:uw=38;5;220:ux=38;5;114:ue=38;5;114:gr=38;5;81:gw=38;5;220:gx=38;5;114:tr=38;5;81:tw=38;5;220:tx=38;5;114:su=1;38;5;203:sf=1;38;5;203:sn=1;38;5;215:sb=38;5;180:uu=1;38;5;141:uR=1;38;5;203:un=38;5;177:gu=38;5;75:gR=38;5;203:gn=38;5;75:lc=38;5;245:lm=38;5;250:da=38;5;109:hd=1;38;5;117:xx=38;5;240:di=1;38;5;39:fi=38;5;252:ex=38;5;252:ln=38;5;45:or=1;38;5;203:pi=38;5;214:so=38;5;171:bd=38;5;214:cd=38;5;214:sp=38;5;214:mp=38;5;39:sc=38;5;252:bu=38;5;252:do=38;5;252:cr=38;5;203:co=38;5;214:tm=38;5;109:cm=38;5;109:im=38;5;171:vi=38;5;171:mu=38;5;171:lo=38;5;171:ga=38;5;114:gm=38;5;220:gd=38;5;203:gv=38;5;81:gt=38;5;215:gi=38;5;245:gc=1;38;5;203:*Dockerfile=38;5;39:*docker-compose*.yml=38;5;39:*docker-compose*.yaml=38;5;39'
  fi

  # Ubuntu and many user profiles define `ls` aliases before this managed
  # block. Bash expands an alias while parsing a same-named `ls()` function,
  # producing invalid syntax, so clear only the shortcuts Automexia replaces.
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

__automexia_osc7() {
  local p=${PWD// /%20}
  printf '\e]7;file://%s%s\a' "${HOSTNAME:-localhost}" "$p"
}

__automexia_title() {
  # Keep the terminal's raw title useful for passive WSL/session detection.
  printf '\e]2;%s@%s: %s\a' "${USER:-user}" "${HOSTNAME:-host}" "$PWD"
}

__automexia_print_colored_path() {
  local remaining=$1 component separators color
  local first_component=1
  local root_color=$'\e[38;2;98;176;255m'
  local parent_color=$'\e[38;2;72;167;255m'
  local leaf_color=$'\e[38;2;45;212;191m'
  local separator_color=$'\e[38;2;88;113;141m'
  local reset_color=$'\e[0m'

  # Builtins only: path styling must never add a process to the prompt hot
  # path. Separator runs are emitted literally, so copied text remains exact.
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
        color=$parent_color
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

__automexia_pre_prompt() {
  # Preserve the status presented to subsequent prompt expansion/hooks. Because
  # this hook is appended after existing PROMPT_COMMAND entries, those hooks see
  # the real command status first; returning the captured status avoids turning
  # it into 0 just because Automexia emitted metadata.
  local status=$?
  printf '\e[0m\e]133;D;%s\a' "$status"
  __automexia_osc7
  __automexia_title
  ((__automexia_prompt_generation += 1))
  __automexia_prompt_is_active=1
  # Automexia owns the stable context spacer and complete path rows. Readline
  # owns only the lambda, editable command, and cursor row, so its delayed
  # SIGWINCH repaint cannot erase or duplicate the terminal-owned path.
  printf '\e]1337;SetUserVar=automexia_prompt_active=MQ==\a\e]133;A;aid=%s\a \n' \
    "$__automexia_prompt_generation"
  printf '\e]133;P;k=c;aid=%s\a' "$__automexia_prompt_generation"
  __automexia_print_colored_path "$PWD"
  printf '\n'
  printf '\e]133;P;k=c;aid=%s\a' "$__automexia_prompt_generation"
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

# Readline owns only the short lambda/input row. The path was already emitted
# by `__automexia_pre_prompt` as terminal-owned content.
PS1='\[\e[38;2;97;231;255m\]'$'\xCE\xBB''\[\e[0m\] \[\e]133;B\a\]\[\e[38;2;238;247;242m\]'
PS0='\[\e[0;$((__automexia_prompt_is_active=0))m\]\[\e]1337;SetUserVar=automexia_prompt_active=MA==\a\]\[\e]133;C\a\]'
