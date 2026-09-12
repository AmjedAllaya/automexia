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
if [[ ${AUTOMEXIA_AMX:-1} != 0 ]] && ! builtin alias amx >/dev/null 2>&1 && ! declare -F amx >/dev/null; then
  amx() {
    # Resolve external-name collisions only on use, not on WSL's startup path.
    local binary
    if binary=$(type -P amx 2>/dev/null); then
      command "$binary" "$@"
      return $?
    fi
    binary=${AUTOMEXIA_CLI:-}
    [[ -n $binary ]] || binary=$(type -P automexia 2>/dev/null)
    if [[ -z $binary || ! -x $binary ]]; then
      printf '%s\n' 'amx: Automexia command unavailable; reopen a current Automexia session.' >&2
      return 127
    fi
    if [[ $binary == *.exe && -n ${WSL_DISTRO_NAME:-} ]]; then
      command "$binary" --amx-wsl-distribution "$WSL_DISTRO_NAME" --amx-wsl-cwd "$PWD" --amx-wsl-path "$PATH" --amx-wsl-home "$HOME" "$@"
    else
      command "$binary" "$@"
    fi
  }
fi
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
__automexia_identity_frame=$(__automexia_publish_static_metadata)
printf '%s' "$__automexia_identity_frame"
unset -f __automexia_publish_static_metadata

# Only these two local paths cross the prompt metadata boundary. Cache bounded
# values and complete frames; unchanged prompts never launch an encoder. Replay
# both frames after every command so a nested shell cannot leave its paths behind.
__automexia_publish_location_hints() {
  local LC_ALL=C hint_home=${HOME:-} hint_config='' attributes
  if [[ -n ${KUBECONFIG:-} ]]; then
    if (( BASH_VERSINFO[0] > 4 || (BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] >= 4) )); then
      [[ ${KUBECONFIG@a} == *x* ]] && hint_config=$KUBECONFIG
    else
      # Bash 3.2 has no attribute expansion. This fallback runs only a builtin
      # in a subshell, and only when an override exists; it never evaluates data.
      attributes=$(declare -p KUBECONFIG 2>/dev/null)
      [[ $attributes == 'declare -'*x*' KUBECONFIG='* ]] && hint_config=$KUBECONFIG
    fi
  fi
  if [[ ${AUTOMEXIA_CONTEXT_PATH_HINTS:-1} == 0 ||
        ${#hint_home} -gt 4096 || ${#hint_config} -gt 4096 ||
        $hint_home == *[[:cntrl:]]* || $hint_config == *[[:cntrl:]]* ]]; then
    hint_home='' hint_config=''
  fi
  if [[ ${__automexia_location_ready:-0} != 1 ||
        $hint_home != "${__automexia_location_home:-}" ||
        $hint_config != "${__automexia_location_config:-}" ]]; then
    __automexia_location_home=$hint_home
    __automexia_location_config=$hint_config
    __automexia_home_frame=$(__automexia_set_user_var automexia_env_HOME "$hint_home")
    __automexia_config_frame=$(__automexia_set_user_var automexia_env_KUBECONFIG "$hint_config")
    if [[ ( -n $hint_home && -z $__automexia_home_frame ) ||
          ( -n $hint_config && -z $__automexia_config_frame ) ]]; then
      __automexia_home_frame='' __automexia_config_frame=''
    fi
    __automexia_location_ready=1
  fi
  printf '%s' "${__automexia_home_frame:-$'\e]1337;SetUserVar=automexia_env_HOME=\a'}"
  printf '%s' "${__automexia_config_frame:-$'\e]1337;SetUserVar=automexia_env_KUBECONFIG=\a'}"
}

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
    export EZA_COLORS='reset:ur=38;5;81:uw=38;5;220:ux=38;5;114:ue=38;5;114:gr=38;5;81:gw=38;5;220:gx=38;5;114:tr=38;5;81:tw=38;5;220:tx=38;5;114:su=1;38;5;203:sf=1;38;5;203:sn=1;38;5;215:sb=38;5;180:uu=1;38;5;141:uR=1;38;5;203:un=38;5;177:gu=38;5;75:gR=38;5;203:gn=38;5;75:lc=38;5;245:lm=38;5;250:da=38;5;109:hd=1;38;5;117:xx=38;5;240:di=1;38;5;39:fi=38;5;252:ex=38;5;252:ln=38;5;45:or=1;38;5;203:pi=38;5;214:so=38;5;171:bd=38;5;214:cd=38;5;214:sp=38;5;214:mp=38;5;39:sc=38;5;252:bu=38;5;252:do=38;5;252:cr=38;5;203:co=38;5;214:tm=38;5;109:cm=38;5;109:im=38;5;171:vi=38;5;171:mu=38;5;171:lo=38;5;171:ga=38;5;114:gm=38;5;220:gd=38;5;203:gv=38;5;81:gt=38;5;215:gi=38;5;245:gc=1;38;5;203:*secret=1;38;5;203:*secrets=1;38;5;203:*private=1;38;5;203:*credentials=1;38;5;203:*vault=1;38;5;203:*.env=1;38;5;203:*.env.*=1;38;5;203:*.key=1;38;5;203:*.pem=1;38;5;203:*config=38;5;214:*configs=38;5;214:*settings=38;5;214:*log=38;5;220:*logs=38;5;220:*.log=38;5;220:*.trace=38;5;220:*apps=38;5;81:*src=38;5;81:*source=38;5;81:*lib=38;5;81:*librio*=38;5;81:*rio-*=38;5;81:*corcovado=38;5;81:*sugarloaf=38;5;81:*teletypewriter=38;5;81:*docs=38;5;114:*documentation=38;5;114:*guides=38;5;114:*test=38;5;177:*tests=38;5;177:*specs=38;5;177:*fuzz=38;5;177:*fixtures=38;5;177:*target=38;5;209:*build=38;5;209:*dist=38;5;209:*out=38;5;209:*changes=38;5;209:*coverage=38;5;209:*assets=38;5;211:*public=38;5;211:*static=38;5;211:*media=38;5;211:*images=38;5;211:*icons=38;5;211:*fonts=38;5;211:*node_modules=38;5;141:*vendor=38;5;141:*packages=38;5;141:*deps=38;5;141:*.cargo=38;5;141:*.git=38;5;177:*.github=38;5;177:*scripts=38;5;80:*tools=38;5;80:*shell-integration=38;5;80:*ci=38;5;80:*data=38;5;105:*db=38;5;105:*database=38;5;105:*storage=38;5;105:*migrations=38;5;105:*.db=38;5;105:*.sqlite=38;5;105:*.cache=38;5;245:*cache=38;5;245:*tmp=38;5;245:*temp=38;5;245:*backups=38;5;245:*infra=38;5;39:*infrastructure=38;5;39:*terraform=38;5;141:*k8s=38;5;39:*kubernetes=38;5;39:*helm=38;5;39:*docker=38;5;39:*packaging=38;5;214:*Dockerfile=38;5;39:*docker-compose*.yml=38;5;39:*docker-compose*.yaml=38;5;39'
  fi

  # Ubuntu and many user profiles define `ls` aliases before this managed
  # block. Bash expands an alias while parsing a same-named `ls()` function,
  # producing invalid syntax, so clear only the shortcuts Automexia replaces.
  unalias ls l ll la lA tree 2>/dev/null || true
  __automexia_integration_dir=${BASH_SOURCE[0]%/*}
  [[ $__automexia_integration_dir != "${BASH_SOURCE[0]}" ]] || __automexia_integration_dir=.
  __automexia_eza_filter_path="$__automexia_integration_dir/../posix/automexia-eza-filter.pl"
  [[ -r $__automexia_eza_filter_path ]] || \
    __automexia_eza_filter_path="$__automexia_integration_dir/automexia-eza-filter.pl"

  __automexia_run_eza() {
    if [[ -t 1 && -r $__automexia_eza_filter_path ]] && command -v perl >/dev/null 2>&1; then
      # Force presentation while eza writes into the badge filter. The wrapper
      # itself is still TTY-gated, so pipes and redirects retain plain output.
      command eza --icons=always --color=always --width="${COLUMNS:-80}" "$@" |
        perl -CS "$__automexia_eza_filter_path"
      local eza_status=${PIPESTATUS[0]}
      return "$eza_status"
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

__automexia_osc7() {
  local p=${PWD// /%20}
  printf '\e]7;file://%s%s\a' "${HOSTNAME:-localhost}" "$p"
}

__automexia_title() {
  # Keep the terminal's raw title useful for passive WSL/session detection.
  printf '\e]2;%s@%s: %s\a' "${USER:-user}" "${HOSTNAME:-host}" "$PWD"
}

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

__automexia_pre_prompt() {
  # Preserve the status presented to subsequent prompt expansion/hooks. Because
  # this hook is appended after existing PROMPT_COMMAND entries, those hooks see
  # the real command status first; returning the captured status avoids turning
  # it into 0 just because Automexia emitted metadata.
  local status=$?
  printf '\e]1337;SetUserVar=automexia_env_pending=MQ==\a'
  printf '%s' "$__automexia_identity_frame"
  __automexia_publish_location_hints
  printf '\e]1337;SetUserVar=automexia_env_pending=MA==\a'
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
# shellcheck disable=SC2178 # The two runtime variants are handled explicitly.
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
# shellcheck disable=SC2016 # Readline evaluates the arithmetic at prompt time.
PS0='\[\e[0;$((__automexia_prompt_is_active=0))m\]\[\e]1337;SetUserVar=automexia_prompt_active=MA==\a\]\[\e]133;C\a\]'

# CP1 completion remains a separate managed adapter so disabling or removing it
# cannot affect prompt, history, listing, or editor ownership.
__automexia_completion_adapter="${BASH_SOURCE[0]%/*}/automexia-completion.bash"
[[ -r $__automexia_completion_adapter ]] || \
  __automexia_completion_adapter="${BASH_SOURCE[0]%/*}/../completion/bash/automexia-completion.bash"
if [[ -r $__automexia_completion_adapter ]]; then
  # shellcheck source=/dev/null
  . "$__automexia_completion_adapter"
fi
unset __automexia_completion_adapter
# CP3.1 persistent aliases are loaded from one immutable, content-addressed
# generation. The hook never regenerates state and never executes a provider.
if [[ -z ${__automexia_alias_loader_initialized+x} ]]; then
  __automexia_alias_loader_initialized=1
  if [[ -n ${AUTOMEXIA_CONFIG_HOME:-} ]]; then
    __automexia_alias_config_root=$AUTOMEXIA_CONFIG_HOME
  elif [[ $(uname -s 2>/dev/null) == Darwin ]]; then
    __automexia_alias_config_root="$HOME/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal"
  else
    __automexia_alias_config_root=${XDG_CONFIG_HOME:-$HOME/.config}/automexia
  fi
  __automexia_alias_root=$__automexia_alias_config_root/generated/aliases
  __automexia_alias_state=uninitialized
  __automexia_alias_reason=
  __automexia_alias_generation=
  __automexia_alias_loaded_path=
  __automexia_alias_records=
  __automexia_alias_collisions=

  __automexia_alias_hash_stream() {
    local output
    if command -v sha256sum >/dev/null 2>&1; then
      output=$(command sha256sum) || return 1
      printf '%s\n' "${output%% *}"
    elif command -v shasum >/dev/null 2>&1; then
      output=$(command shasum -a 256) || return 1
      printf '%s\n' "${output%% *}"
    elif command -v openssl >/dev/null 2>&1; then
      output=$(command openssl dgst -sha256) || return 1
      printf '%s\n' "${output##* }"
    else
      return 127
    fi
  }

  __automexia_alias_hash_file() {
    local file=$1 output
    if command -v sha256sum >/dev/null 2>&1; then
      output=$(command sha256sum -- "$file") || return 1
      printf '%s\n' "${output%% *}"
    elif command -v shasum >/dev/null 2>&1; then
      output=$(command shasum -a 256 -- "$file") || return 1
      printf '%s\n' "${output%% *}"
    elif command -v openssl >/dev/null 2>&1; then
      output=$(command openssl dgst -sha256 "$file") || return 1
      printf '%s\n' "${output##* }"
    else
      return 127
    fi
  }

  __automexia_alias_real_private_directories() {
    local output path mode count=0
    for path in "$@"; do
      [[ -d $path && ! -L $path ]] || return 1
    done
    output=$(command stat -c '%a' "$@" 2>/dev/null) ||
      output=$(command stat -f '%Lp' "$@" 2>/dev/null) || return 1
    while IFS= read -r mode; do
      [[ $mode =~ ^[0-7]{3,4}$ ]] || return 1
      (( (8#$mode & 077) == 0 )) || return 1
      count=$((count + 1))
    done <<<"$output"
    (( count == $# ))
  }

  __automexia_alias_real_private_directory() {
    __automexia_alias_real_private_directories "$1"
  }

  __automexia_alias_real_private_file() {
    local path=$1 maximum=$2 output mode size extra
    [[ -f $path && ! -L $path ]] || return 1
    output=$(command stat -c '%a %s' "$path" 2>/dev/null) ||
      output=$(command stat -f '%Lp %z' "$path" 2>/dev/null) || return 1
    read -r mode size extra <<<"$output"
    [[ -z $extra && $mode =~ ^[0-7]{3,4}$ && $size =~ ^[0-9]+$ ]] || return 1
    (( (8#$mode & 077) == 0 && size <= maximum ))
  }

  __automexia_alias_consent() {
    local requested=$1 entry name fingerprint
    REPLY=
    local old_ifs=$IFS
    IFS=,
    for entry in $__automexia_alias_candidate_overrides; do
      IFS=:
      read -r name fingerprint extra <<<"$entry"
      IFS=,
      if [[ $name == "$requested" && -z $extra &&
            $fingerprint =~ ^[0-9a-f]{64}$ ]]; then
        REPLY=$fingerprint
        IFS=$old_ifs
        return 0
      fi
    done
    IFS=$old_ifs
    return 1
  }

  __automexia_alias_current_definition_digest() {
    local kind=$1 name=$2 definition
    case $kind in
      A) definition=$(builtin alias -- "$name" 2>/dev/null) || return 1 ;;
      F) definition=$(declare -f -- "$name" 2>/dev/null) || return 1 ;;
      *) return 1 ;;
    esac
    printf '%s' "$definition" | __automexia_alias_hash_stream
  }

  __automexia_alias_owned_public() {
    local requested=$1 kind name expected actual
    while IFS='|' read -r kind name expected; do
      [[ $kind == A && $name == "$requested" ]] || continue
      actual=$(__automexia_alias_current_definition_digest "$kind" "$name") || return 1
      [[ $actual == "$expected" ]]
      return
    done <<<"$__automexia_alias_records"
    return 1
  }

  __automexia_alias_runtime_fingerprint() {
    local requested=$1 kind path
    kind=$(builtin type -t -- "$requested" 2>/dev/null) || return 1
    case $kind in
      file)
        path=$(builtin type -P -- "$requested" 2>/dev/null) || return 1
        [[ -f $path && ! -L $path ]] || return 1
        __automexia_alias_hash_file "$path"
        ;;
      builtin)
        printf '%s' "bash|Builtin|$requested|shell-builtin" |
          __automexia_alias_hash_stream
        ;;
      *)
        return 1
        ;;
    esac
  }

  __automexia_alias_prepare() {
    local pointer manifest manifest_hash line count=0 tag extra
    local manifest_lines=0 manifest_valid=1
    local name expected actual kind old_ifs

    __automexia_alias_reason=
    __automexia_alias_collisions=
    __automexia_alias_candidate_generation=
    __automexia_alias_candidate_path=
    __automexia_alias_candidate_names=
    __automexia_alias_candidate_overrides=

    [[ $__automexia_alias_config_root == /* &&
       ${#__automexia_alias_config_root} -le 4096 ]] || {
      __automexia_alias_state=unsafe-path
      __automexia_alias_reason="config-root"
      return 1
    }
    __automexia_alias_real_private_directories \
      "$__automexia_alias_config_root/generated" \
      "$__automexia_alias_root" "$__automexia_alias_root/generations" || {
      __automexia_alias_state=unsafe-permissions
      __automexia_alias_reason=directory
      return 1
    }

    pointer=$__automexia_alias_root/current
    __automexia_alias_real_private_file "$pointer" 80 || {
      __automexia_alias_state=uninitialized
      __automexia_alias_reason=current
      return 1
    }
    IFS= read -r __automexia_alias_candidate_generation <"$pointer" || {
      __automexia_alias_state=tampered
      __automexia_alias_reason="current-read"
      return 1
    }
    [[ $(command wc -l <"$pointer") -eq 1 ]] || {
      __automexia_alias_state=tampered
      __automexia_alias_reason="current-lines"
      return 1
    }
    if [[ $__automexia_alias_candidate_generation == disabled ]]; then
      __automexia_alias_state=disabled
      return 1
    fi
    [[ $__automexia_alias_candidate_generation =~ ^[0-9a-f]{64}$ ]] || {
      __automexia_alias_state=tampered
      __automexia_alias_reason="current-format"
      return 1
    }

    __automexia_alias_candidate_directory="$__automexia_alias_root/generations/$__automexia_alias_candidate_generation"
    __automexia_alias_real_private_directory "$__automexia_alias_candidate_directory" || {
      __automexia_alias_state=tampered
      __automexia_alias_reason="generation-directory"
      return 1
    }
    manifest=$__automexia_alias_candidate_directory/generation.manifest
    __automexia_alias_real_private_file "$manifest" 65536 || {
      __automexia_alias_state=tampered
      __automexia_alias_reason="manifest-file"
      return 1
    }
    manifest_hash=$(__automexia_alias_hash_file "$manifest") || {
      __automexia_alias_state=unavailable
      __automexia_alias_reason=sha256
      return 1
    }
    [[ $manifest_hash == "$__automexia_alias_candidate_generation" ]] || {
      __automexia_alias_state=tampered
      __automexia_alias_reason="manifest-digest"
      return 1
    }
    while IFS= read -r line; do
      manifest_lines=$((manifest_lines + 1))
      case $manifest_lines in
        1) [[ $line == automexia-alias-generation-v1 ]] || manifest_valid=0 ;;
        2) [[ $line == schema=1 ]] || manifest_valid=0 ;;
        3) [[ $line =~ ^source-revision=(0|[1-9][0-9]*)$ ]] || manifest_valid=0 ;;
        4) [[ $line =~ ^source-digest=[0-9a-f]{64}$ ]] || manifest_valid=0 ;;
        5) [[ $line == generator=automexia-devops/0.4.0 ]] || manifest_valid=0 ;;
        6) [[ $line == 'shell=powershell|'* ]] || manifest_valid=0 ;;
        7)
          [[ $line == 'shell=bash|'* ]] || manifest_valid=0
          count=$((count + 1))
          old_ifs=$IFS
          IFS='|'
          read -r tag __automexia_alias_candidate_file \
            __automexia_alias_candidate_sha __automexia_alias_candidate_artifact \
            __automexia_alias_candidate_ready __automexia_alias_candidate_decisions \
            __automexia_alias_candidate_names __automexia_alias_candidate_overrides extra <<<"$line"
          IFS=$old_ifs
          ;;
        8) [[ $line == 'shell=zsh|'* ]] || manifest_valid=0 ;;
        9) [[ $line == 'shell=fish|'* ]] || manifest_valid=0 ;;
        10) [[ $line == 'shell=cmd|'* ]] || manifest_valid=0 ;;
        *) manifest_valid=0 ;;
      esac
    done <"$manifest"
    [[ $manifest_lines -eq 10 && $manifest_valid -eq 1 &&
       $count -eq 1 && $tag == shell=bash &&
       $extra == '' &&
       $__automexia_alias_candidate_file == automexia-aliases.bash &&
       $__automexia_alias_candidate_sha =~ ^[0-9a-f]{64}$ &&
       $__automexia_alias_candidate_artifact =~ ^[0-9a-f]{64}$ &&
       $__automexia_alias_candidate_ready =~ ^[0-9]+$ &&
       $__automexia_alias_candidate_decisions =~ ^[0-9]+$ ]] || {
      __automexia_alias_state=tampered
      __automexia_alias_reason="manifest-shell"
      return 1
    }

    __automexia_alias_candidate_shell_directory="$__automexia_alias_candidate_directory/bash"
    __automexia_alias_real_private_directory "$__automexia_alias_candidate_shell_directory" || {
      __automexia_alias_state=tampered
      __automexia_alias_reason="shell-directory"
      return 1
    }
    __automexia_alias_candidate_path="$__automexia_alias_candidate_shell_directory/$__automexia_alias_candidate_file"
    __automexia_alias_real_private_file "$__automexia_alias_candidate_path" 1114112 || {
      __automexia_alias_state=tampered
      __automexia_alias_reason="artifact-file"
      return 1
    }
    actual=$(__automexia_alias_hash_file "$__automexia_alias_candidate_path") || {
      __automexia_alias_state=unavailable
      __automexia_alias_reason=sha256
      return 1
    }
    [[ $actual == "$__automexia_alias_candidate_sha" ]] || {
      __automexia_alias_state=tampered
      __automexia_alias_reason="artifact-digest"
      return 1
    }

    count=0
    old_ifs=$IFS
    IFS=,
    for name in $__automexia_alias_candidate_names; do
      [[ $name =~ ^[a-z][a-z0-9-]{1,31}$ ]] || {
        IFS=$old_ifs
        __automexia_alias_state=tampered
        __automexia_alias_reason="alias-name"
        return 1
      }
      count=$((count + 1))
      if builtin type -t -- "$name" >/dev/null 2>&1 &&
         ! __automexia_alias_owned_public "$name"; then
        if ! __automexia_alias_consent "$name"; then
          __automexia_alias_collisions="${__automexia_alias_collisions}${__automexia_alias_collisions:+,}$name"
          continue
        fi
        expected=$REPLY
        actual=$(__automexia_alias_runtime_fingerprint "$name") || actual=
        if [[ $actual != "$expected" ]]; then
          __automexia_alias_collisions="${__automexia_alias_collisions}${__automexia_alias_collisions:+,}$name"
        fi
      fi
    done
    IFS=$old_ifs
    [[ $count -eq $__automexia_alias_candidate_ready ]] || {
      __automexia_alias_state=tampered
      __automexia_alias_reason="binding-count"
      return 1
    }
    [[ -z $__automexia_alias_collisions ]] || {
      __automexia_alias_state=collision
      __automexia_alias_reason="native-wins"
      return 1
    }
    return 0
  }

  __automexia_alias_activate_candidate() {
    local name definition digest target old_ifs
    # shellcheck source=/dev/null
    . "$__automexia_alias_candidate_path" || return 1
    __automexia_alias_records=
    old_ifs=$IFS
    IFS=,
    for name in $__automexia_alias_candidate_names; do
      definition=$(builtin alias -- "$name" 2>/dev/null) || {
        IFS=$old_ifs
        return 1
      }
      digest=$(printf '%s' "$definition" | __automexia_alias_hash_stream) || {
        IFS=$old_ifs
        return 1
      }
      __automexia_alias_records="${__automexia_alias_records}${__automexia_alias_records:+
}A|$name|$digest"
      target=${definition#*=}
      target=${target#\'}
      target=${target%\'}
      if [[ $target =~ ^_automexia_cp3_[0-9a-f]{16}$ ]] &&
         declare -f -- "$target" >/dev/null 2>&1; then
        digest=$(__automexia_alias_current_definition_digest F "$target") || {
          IFS=$old_ifs
          return 1
        }
        __automexia_alias_records="${__automexia_alias_records}
F|$target|$digest"
      fi
    done
    IFS=$old_ifs
    __automexia_alias_generation=$__automexia_alias_candidate_generation
    __automexia_alias_loaded_path=$__automexia_alias_candidate_path
    __automexia_alias_state=ready
    __automexia_alias_reason=
    return 0
  }

  automexia_aliases_reload() {
    local old_generation=$__automexia_alias_generation
    local old_path=$__automexia_alias_loaded_path
    local old_records=$__automexia_alias_records
    local kind name expected actual

    if ! __automexia_alias_prepare; then
      [[ -n $old_generation ]] && __automexia_alias_reason="reload-$__automexia_alias_state-lkg"
      return 1
    fi
    [[ $__automexia_alias_candidate_generation != "$old_generation" ]] || {
      __automexia_alias_state=ready
      return 0
    }
    while IFS='|' read -r kind name expected; do
      [[ -n $kind ]] || continue
      actual=$(__automexia_alias_current_definition_digest "$kind" "$name") || {
        __automexia_alias_state=reload-conflict
        __automexia_alias_reason="definition-changed"
        return 1
      }
      [[ $actual == "$expected" ]] || {
        __automexia_alias_state=reload-conflict
        __automexia_alias_reason="definition-changed"
        return 1
      }
    done <<<"$old_records"

    while IFS='|' read -r kind name expected; do
      case $kind in
        A) builtin unalias -- "$name" ;;
        F) unset -f -- "$name" ;;
      esac
    done <<<"$old_records"
    if __automexia_alias_activate_candidate; then
      return 0
    fi

    # The private generation path was already bounded and digest-verified.
    # shellcheck disable=SC1090
    [[ -n $old_path && -f $old_path && ! -L $old_path ]] && . "$old_path"
    __automexia_alias_generation=$old_generation
    __automexia_alias_loaded_path=$old_path
    __automexia_alias_records=$old_records
    __automexia_alias_state=reload-failed-lkg
    __automexia_alias_reason=activation
    return 1
  }

  automexia_aliases_health() {
    printf 'state=%s generation=%s collisions=%s reason=%s\n' \
      "$__automexia_alias_state" \
      "${__automexia_alias_generation:-none}" \
      "${__automexia_alias_collisions:-none}" \
      "${__automexia_alias_reason:-none}"
  }

  if __automexia_alias_prepare; then
    __automexia_alias_activate_candidate || {
      __automexia_alias_state=activation-failed
      __automexia_alias_reason=source
    }
  fi
fi
