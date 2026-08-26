# Automexia Zsh/macOS/WSL shell integration. Metadata, prompt styling, and an
# optional icon-aware directory-listing experience.
case "${TERM_PROGRAM:-}|${AUTOMEXIA_SHELL_INTEGRATION:-}|${WSLENV:-}" in
  Automexia*|*'|1|'*|*'AUTOMEXIA_SHELL_INTEGRATION/u'*) ;;
  *) return 0 ;;
esac

[[ -n ${AUTOMEXIA_ZSH_INTEGRATION_LOADED:-} ]] && return 0
typeset -g AUTOMEXIA_ZSH_INTEGRATION_LOADED=1

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

# Completion is independently removable and never calls `compinit`; the user's
# existing compsys setup remains authoritative.
typeset __automexia_completion_adapter="${${(%):-%N}:A:h}/automexia-completion.zsh"
[[ -r $__automexia_completion_adapter ]] || \
  __automexia_completion_adapter="${${(%):-%N}:A:h}/../completion/zsh/automexia-completion.zsh"
[[ -r $__automexia_completion_adapter ]] && source "$__automexia_completion_adapter"
unset __automexia_completion_adapter
# CP3.1 persistent aliases use the same immutable generation contract as Bash,
# with Zsh-native collision and definition ownership checks.
if [[ -z ${__automexia_alias_loader_initialized+x} ]]; then
  typeset -g __automexia_alias_loader_initialized=1
  if [[ -n ${AUTOMEXIA_CONFIG_HOME:-} ]]; then
    typeset -g __automexia_alias_config_root=$AUTOMEXIA_CONFIG_HOME
  elif [[ $OSTYPE == darwin* ]]; then
    typeset -g __automexia_alias_config_root="$HOME/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal"
  else
    typeset -g __automexia_alias_config_root=${XDG_CONFIG_HOME:-$HOME/.config}/automexia
  fi
  typeset -g __automexia_alias_root=$__automexia_alias_config_root/generated/aliases
  typeset -g __automexia_alias_state=uninitialized
  typeset -g __automexia_alias_reason=
  typeset -g __automexia_alias_generation=
  typeset -g __automexia_alias_loaded_path=
  typeset -g __automexia_alias_records=
  typeset -g __automexia_alias_collisions=

  __automexia_alias_hash_stream() {
    if (( $+commands[sha256sum] )); then
      command sha256sum | command awk '{print $1}'
    elif (( $+commands[shasum] )); then
      command shasum -a 256 | command awk '{print $1}'
    elif (( $+commands[openssl] )); then
      command openssl dgst -sha256 | command awk '{print $NF}'
    else
      return 127
    fi
  }

  __automexia_alias_hash_file() {
    local file=$1
    if (( $+commands[sha256sum] )); then
      command sha256sum -- "$file" | command awk '{print $1}'
    elif (( $+commands[shasum] )); then
      command shasum -a 256 -- "$file" | command awk '{print $1}'
    elif (( $+commands[openssl] )); then
      command openssl dgst -sha256 "$file" | command awk '{print $NF}'
    else
      return 127
    fi
  }

  __automexia_alias_private_mode() {
    local file_path=$1
    local -a modes
    zmodload zsh/stat 2>/dev/null || return 1
    zstat -A modes +mode -- "$file_path" 2>/dev/null || return 1
    (( ${#modes} == 1 && (modes[1] & 63) == 0 ))
  }

  __automexia_alias_real_private_directory() {
    [[ -d $1 && ! -h $1 ]] && __automexia_alias_private_mode "$1"
  }

  __automexia_alias_real_private_file() {
    local file_path=$1 maximum=$2
    [[ -f $file_path && ! -h $file_path ]] || return 1
    __automexia_alias_private_mode "$file_path" || return 1
    [[ $(command wc -c <"$file_path") -le $maximum ]]
  }

  __automexia_alias_consent() {
    local requested=$1 entry name fingerprint extra
    REPLY=
    for entry in ${(s:,:)__automexia_alias_candidate_overrides}; do
      IFS=: read -r name fingerprint extra <<<"$entry"
      if [[ $name == "$requested" && -z $extra &&
            $fingerprint =~ '^[0-9a-f]{64}$' ]]; then
        REPLY=$fingerprint
        return 0
      fi
    done
    return 1
  }

  __automexia_alias_current_definition_digest() {
    local kind=$1 name=$2 definition
    case $kind in
      A) definition=$(builtin alias -- "$name" 2>/dev/null) || return 1 ;;
      F) definition=$(functions -- "$name" 2>/dev/null) || return 1 ;;
      *) return 1 ;;
    esac
    print -rn -- "$definition" | __automexia_alias_hash_stream
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

  __automexia_alias_name_exists() {
    local requested=$1
    whence -w "$requested" >/dev/null 2>&1
  }

  __automexia_alias_runtime_fingerprint() {
    local requested=$1 command_path classification
    classification=$(whence -w "$requested" 2>/dev/null) || return 1
    case $classification in
      "$requested: command")
        command_path=$(whence -p "$requested" 2>/dev/null) || return 1
        [[ -f $command_path && ! -h $command_path ]] || return 1
        __automexia_alias_hash_file "$command_path"
        ;;
      "$requested: builtin"|"$requested: reserved")
        print -rn -- "zsh|Builtin|$requested|shell-builtin" |
          __automexia_alias_hash_stream
        ;;
      *) return 1 ;;
    esac
  }

  __automexia_alias_prepare() {
    local pointer manifest manifest_hash line count=0 tag extra
    local manifest_lines=0 manifest_valid=1
    local name expected actual

    __automexia_alias_reason=
    __automexia_alias_collisions=
    __automexia_alias_candidate_generation=
    __automexia_alias_candidate_path=
    __automexia_alias_candidate_names=
    __automexia_alias_candidate_overrides=

    [[ $__automexia_alias_config_root == /* &&
       ${#__automexia_alias_config_root} -le 4096 ]] || {
      __automexia_alias_state=unsafe-path
      __automexia_alias_reason=config-root
      return 1
    }
    for line in "$__automexia_alias_config_root/generated" \
      "$__automexia_alias_root" "$__automexia_alias_root/generations"; do
      __automexia_alias_real_private_directory "$line" || {
        __automexia_alias_state=unsafe-permissions
        __automexia_alias_reason=directory
        return 1
      }
    done

    pointer=$__automexia_alias_root/current
    __automexia_alias_real_private_file "$pointer" 80 || {
      __automexia_alias_state=uninitialized
      __automexia_alias_reason=current
      return 1
    }
    IFS= read -r __automexia_alias_candidate_generation <"$pointer" || {
      __automexia_alias_state=tampered
      __automexia_alias_reason=current-read
      return 1
    }
    [[ $(command wc -l <"$pointer") -eq 1 ]] || {
      __automexia_alias_state=tampered
      __automexia_alias_reason=current-lines
      return 1
    }
    if [[ $__automexia_alias_candidate_generation == disabled ]]; then
      __automexia_alias_state=disabled
      return 1
    fi
    [[ $__automexia_alias_candidate_generation =~ '^[0-9a-f]{64}$' ]] || {
      __automexia_alias_state=tampered
      __automexia_alias_reason=current-format
      return 1
    }

    __automexia_alias_candidate_directory="$__automexia_alias_root/generations/$__automexia_alias_candidate_generation"
    __automexia_alias_real_private_directory "$__automexia_alias_candidate_directory" || {
      __automexia_alias_state=tampered
      __automexia_alias_reason=generation-directory
      return 1
    }
    manifest=$__automexia_alias_candidate_directory/generation.manifest
    __automexia_alias_real_private_file "$manifest" 65536 || {
      __automexia_alias_state=tampered
      __automexia_alias_reason=manifest-file
      return 1
    }
    manifest_hash=$(__automexia_alias_hash_file "$manifest") || {
      __automexia_alias_state=unavailable
      __automexia_alias_reason=sha256
      return 1
    }
    [[ $manifest_hash == "$__automexia_alias_candidate_generation" ]] || {
      __automexia_alias_state=tampered
      __automexia_alias_reason=manifest-digest
      return 1
    }
    while IFS= read -r line; do
      manifest_lines=$((manifest_lines + 1))
      case $manifest_lines in
        1) [[ $line == automexia-alias-generation-v1 ]] || manifest_valid=0 ;;
        2) [[ $line == schema=1 ]] || manifest_valid=0 ;;
        3) [[ $line =~ '^(source-revision=)(0|[1-9][0-9]*)$' ]] || manifest_valid=0 ;;
        4) [[ $line =~ '^source-digest=[0-9a-f]{64}$' ]] || manifest_valid=0 ;;
        5) [[ $line == generator=automexia-devops/0.4.0 ]] || manifest_valid=0 ;;
        6) [[ $line == 'shell=powershell|'* ]] || manifest_valid=0 ;;
        7) [[ $line == 'shell=bash|'* ]] || manifest_valid=0 ;;
        8)
          [[ $line == 'shell=zsh|'* ]] || manifest_valid=0
          count=$((count + 1))
          IFS='|' read -r tag __automexia_alias_candidate_file \
            __automexia_alias_candidate_sha __automexia_alias_candidate_artifact \
            __automexia_alias_candidate_ready __automexia_alias_candidate_decisions \
            __automexia_alias_candidate_names __automexia_alias_candidate_overrides extra <<<"$line"
          ;;
        9) [[ $line == 'shell=fish|'* ]] || manifest_valid=0 ;;
        10) [[ $line == 'shell=cmd|'* ]] || manifest_valid=0 ;;
        *) manifest_valid=0 ;;
      esac
    done <"$manifest"
    [[ $manifest_lines -eq 10 && $manifest_valid -eq 1 &&
       $count -eq 1 && $tag == shell=zsh && -z $extra &&
       $__automexia_alias_candidate_file == automexia-aliases.zsh &&
       $__automexia_alias_candidate_sha =~ '^[0-9a-f]{64}$' &&
       $__automexia_alias_candidate_artifact =~ '^[0-9a-f]{64}$' &&
       $__automexia_alias_candidate_ready =~ '^[0-9]+$' &&
       $__automexia_alias_candidate_decisions =~ '^[0-9]+$' ]] || {
      __automexia_alias_state=tampered
      __automexia_alias_reason=manifest-shell
      return 1
    }

    __automexia_alias_candidate_shell_directory="$__automexia_alias_candidate_directory/zsh"
    __automexia_alias_real_private_directory "$__automexia_alias_candidate_shell_directory" || {
      __automexia_alias_state=tampered
      __automexia_alias_reason=shell-directory
      return 1
    }
    __automexia_alias_candidate_path="$__automexia_alias_candidate_shell_directory/$__automexia_alias_candidate_file"
    __automexia_alias_real_private_file "$__automexia_alias_candidate_path" 1114112 || {
      __automexia_alias_state=tampered
      __automexia_alias_reason=artifact-file
      return 1
    }
    actual=$(__automexia_alias_hash_file "$__automexia_alias_candidate_path") || {
      __automexia_alias_state=unavailable
      __automexia_alias_reason=sha256
      return 1
    }
    [[ $actual == "$__automexia_alias_candidate_sha" ]] || {
      __automexia_alias_state=tampered
      __automexia_alias_reason=artifact-digest
      return 1
    }

    count=0
    for name in ${(s:,:)__automexia_alias_candidate_names}; do
      [[ $name =~ '^[a-z][a-z0-9-]{1,31}$' ]] || {
        __automexia_alias_state=tampered
        __automexia_alias_reason=alias-name
        return 1
      }
      count=$((count + 1))
      if __automexia_alias_name_exists "$name" &&
         ! __automexia_alias_owned_public "$name"; then
        if ! __automexia_alias_consent "$name"; then
          __automexia_alias_collisions="${__automexia_alias_collisions}${__automexia_alias_collisions:+,}$name"
          continue
        fi
        expected=$REPLY
        actual=$(__automexia_alias_runtime_fingerprint "$name") || actual=
        [[ $actual == "$expected" ]] ||
          __automexia_alias_collisions="${__automexia_alias_collisions}${__automexia_alias_collisions:+,}$name"
      fi
    done
    [[ $count -eq $__automexia_alias_candidate_ready ]] || {
      __automexia_alias_state=tampered
      __automexia_alias_reason=binding-count
      return 1
    }
    [[ -z $__automexia_alias_collisions ]] || {
      __automexia_alias_state=collision
      __automexia_alias_reason=native-wins
      return 1
    }
    return 0
  }

  __automexia_alias_activate_candidate() {
    local name definition digest target
    source "$__automexia_alias_candidate_path" || return 1
    __automexia_alias_records=
    for name in ${(s:,:)__automexia_alias_candidate_names}; do
      definition=$(builtin alias -- "$name" 2>/dev/null) || return 1
      digest=$(print -rn -- "$definition" | __automexia_alias_hash_stream) || return 1
      __automexia_alias_records="${__automexia_alias_records}${__automexia_alias_records:+
}A|$name|$digest"
      target=${definition#*=}
      target=${target#\'}
      target=${target%\'}
      if [[ $target =~ '^_automexia_cp3_[0-9a-f]{16}$' ]] &&
         [[ $(whence -w "$target" 2>/dev/null) == "$target: function" ]]; then
        digest=$(__automexia_alias_current_definition_digest F "$target") || return 1
        __automexia_alias_records="${__automexia_alias_records}
F|$target|$digest"
      fi
    done
    __automexia_alias_generation=$__automexia_alias_candidate_generation
    __automexia_alias_loaded_path=$__automexia_alias_candidate_path
    __automexia_alias_state=ready
    __automexia_alias_reason=
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
        __automexia_alias_reason=definition-changed
        return 1
      }
      [[ $actual == "$expected" ]] || {
        __automexia_alias_state=reload-conflict
        __automexia_alias_reason=definition-changed
        return 1
      }
    done <<<"$old_records"

    while IFS='|' read -r kind name expected; do
      case $kind in
        A) builtin unalias -- "$name" ;;
        F) unfunction -- "$name" ;;
      esac
    done <<<"$old_records"
    if __automexia_alias_activate_candidate; then
      return 0
    fi
    [[ -n $old_path && -f $old_path && ! -h $old_path ]] && source "$old_path"
    __automexia_alias_generation=$old_generation
    __automexia_alias_loaded_path=$old_path
    __automexia_alias_records=$old_records
    __automexia_alias_state=reload-failed-lkg
    __automexia_alias_reason=activation
    return 1
  }

  automexia_aliases_health() {
    print -r -- "state=$__automexia_alias_state generation=${__automexia_alias_generation:-none} collisions=${__automexia_alias_collisions:-none} reason=${__automexia_alias_reason:-none}"
  }

  if __automexia_alias_prepare; then
    __automexia_alias_activate_candidate || {
      __automexia_alias_state=activation-failed
      __automexia_alias_reason=source
    }
  fi
fi
