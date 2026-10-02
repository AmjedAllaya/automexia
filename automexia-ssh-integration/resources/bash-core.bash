# Automexia SSH protocol-1 optional, session-only Bash core. Not a launch grant.
# Preserve native prompt text/hooks. No DEBUG/EXIT trap, aliases or editor hooks.
[[ $- == *i* && -t 2 ]] || return 0
(( BASH_VERSINFO[0] > 5 || (BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] >= 1) )) || return 0
shopt -q restricted_shell && return 0
shopt -q promptvars || return 0
[[ $- != *e* ]] || return 0
# Refuse namespace collisions; repeated sourcing is inert.
[[ -z $(builtin compgen -A variable __amx_ssh_) && -z $(builtin compgen -A function __amx_ssh_) && -z $(builtin compgen -A alias __amx_ssh_) ]] || return 0
case "$(declare -p PS1 2>/dev/null)" in
    'declare -- PS1='*|'declare -x PS1='*|'') ;;
    *) return 0 ;;
esac
case "$(declare -p PS0 2>/dev/null)" in
    'declare -- PS0='*|'declare -x PS0='*|'') ;;
    *) return 0 ;;
esac
case "$(declare -p PS2 2>/dev/null)" in
    'declare -- PS2='*|'declare -x PS2='*|'') ;;
    *) return 0 ;;
esac
case "$(declare -p PROMPT_COMMAND 2>/dev/null)" in
    'declare -- PROMPT_COMMAND='*|'declare -x PROMPT_COMMAND='*|'declare -a PROMPT_COMMAND='*|'declare -ax PROMPT_COMMAND='*|'') ;;
    *) return 0 ;;
esac
[[ ${PS1:-} != *']133;'* && ${PS0:-} != *']133;'* && ${PS2:-} != *']133;'* ]] || return 0
[[ -R COLORTERM || -R TERM_PROGRAM ||
   ( -v COLORTERM && ${COLORTERM@a} == *[raA]* ) ||
   ( -v TERM_PROGRAM && ${TERM_PROGRAM@a} == *[raA]* ) ]] && return 0
__amx_ssh_core_loaded=1
builtin export COLORTERM=truecolor TERM_PROGRAM=Automexia
# shellcheck disable=SC2016
__amx_ssh_prefix='\[\e]133;P;k=c;aid=${__amx_ssh_prompt_id}\a\]'
__amx_ssh_suffix='\[\e]133;B\a\]'
# shellcheck disable=SC2016
__amx_ssh_secondary='\[\e]133;P;k=s;aid=${__amx_ssh_prompt_id}\a\]'
# PS0 executes only for real commands; preserve native PS0 first.
# shellcheck disable=SC2016
__amx_ssh_preexec='\[\e]133;C;amx=$((__amx_ssh_running=1))\a\]'
__amx_ssh_running=0
__amx_ssh_status=0
__amx_ssh_initializing=1
__amx_ssh_prompt_id=1
__amx_ssh_cwd=''
__amx_ssh_cwd_frame=''
__amx_ssh_can_cwd=0
__amx_ssh_active=1
command -v base64 >/dev/null 2>&1 && __amx_ssh_can_cwd=1
function __amx_ssh_valid_label {
    local LC_ALL=C
    [[ ${#1} -le 256 && $1 != *[[:cntrl:]]* && $1 != *$'\xc2'[$'\x80'-$'\x9f']* &&
       $1 != *$'\xd8\x9c'* && $1 != *$'\xe2\x80'[$'\x8e'-$'\x8f']* &&
       $1 != *$'\xe2\x80'[$'\xaa'-$'\xae']* && $1 != *$'\xe2\x81'[$'\xa6'-$'\xa9']* ]]
}
function __amx_ssh_identity {
    local LC_ALL=C __amx_user=${USER:-${LOGNAME:-}} __amx_encoded=''
    builtin printf -v __amx_ssh_user_frame '\e]1337;SetUserVar=automexia_ssh_user=@@CLEAR_USER@@\a'
    # Bounded remote text only; no identity lookup or hostnames.
    [[ -n $__amx_user && $__amx_ssh_can_cwd == 1 ]] && __amx_ssh_valid_label "$__amx_user" || return 0
    __amx_encoded=$(builtin printf 'AMXSSHUSER1|@@PANE@@|@@GENERATION@@|%s' "$__amx_user" | command base64 2>/dev/null) || return 0
    __amx_encoded=${__amx_encoded//$'\n'/}
    __amx_encoded=${__amx_encoded//$'\r'/}
    if [[ -n $__amx_encoded && $__amx_encoded != *[!a-zA-Z0-9+/=]* && ${#__amx_encoded} -le 512 ]]; then
        builtin printf -v __amx_ssh_user_frame '\e]1337;SetUserVar=automexia_ssh_user=%s\a' "$__amx_encoded"
    fi
}
# Resolve/encode the session identity once, then replay only the bounded frame.
__amx_ssh_identity
unset -f __amx_ssh_identity
__amx_ssh_context=''
__amx_ssh_context_frame=''
function __amx_ssh_context_update {
    if builtin declare -F __amx_ssh_helper_prompt >/dev/null; then
        __amx_ssh_helper_prompt
        return 0
    fi
    local __amx_snapshot=$'AMXSSHCTX1|@@PANE@@|@@GENERATION@@|\n' __amx_i __amx_value __amx_encoded
    local -a __amx_names=(git_branch kubernetes_context kubernetes_namespace docker_context terraform_workspace environment aws_profile azure_cloud gcp_project)
    local -a __amx_values=("${GIT_BRANCH:-}" "${KUBECONTEXT:-${KUBE_CONTEXT:-}}" "${KUBE_NAMESPACE:-}" "${DOCKER_CONTEXT:-}" "${TF_WORKSPACE:-}" "${AUTOMEXIA_ENV:-${ENVIRONMENT:-${APP_ENV:-${NODE_ENV:-}}}}" "${AWS_PROFILE:-${AWS_DEFAULT_PROFILE:-}}" "${AZURE_CLOUD_NAME:-}" "${CLOUDSDK_CORE_PROJECT:-}")
    for __amx_i in "${!__amx_names[@]}"; do
        __amx_value=${__amx_values[__amx_i]}
        __amx_ssh_valid_label "$__amx_value" || __amx_value=''
        __amx_snapshot+=${__amx_names[__amx_i]}=$__amx_value$'\n'
    done
    if [[ $__amx_snapshot != "$__amx_ssh_context" || -z $__amx_ssh_context_frame ]]; then
        __amx_ssh_context=$__amx_snapshot
        builtin printf -v __amx_ssh_context_frame '\e]1337;SetUserVar=automexia_ssh_context=@@CLEAR_CONTEXT@@\a'
        if (( __amx_ssh_can_cwd )); then
            __amx_encoded=$(builtin printf '%s' "$__amx_snapshot" | command base64 2>/dev/null) || __amx_encoded=''
            __amx_encoded=${__amx_encoded//$'\n'/}
            __amx_encoded=${__amx_encoded//$'\r'/}
            if [[ -n $__amx_encoded && $__amx_encoded != *[!a-zA-Z0-9+/=]* && ${#__amx_encoded} -le 4096 ]]; then
                builtin printf -v __amx_ssh_context_frame '\e]1337;SetUserVar=automexia_ssh_context=%s\a' "$__amx_encoded"
            fi
        fi
    fi
    builtin printf '%s' "$__amx_ssh_context_frame" >&2
}
function __amx_ssh_capture_status {
    __amx_ssh_status=$?
    return "$__amx_ssh_status"
}
function __amx_ssh_unwrap {
    # Remove only our exact framing. Never restore an older native prompt.
    if [[ ${PS1:-} == "$__amx_ssh_prefix"*"$__amx_ssh_suffix" && ${PS1@a} != *r* ]]; then
        PS1=${PS1#"$__amx_ssh_prefix"}
        PS1=${PS1%"$__amx_ssh_suffix"}
    fi
    if [[ ${PS2:-} == "$__amx_ssh_secondary"* && ${PS2@a} != *r* ]]; then
        PS2=${PS2#"$__amx_ssh_secondary"}
    fi
    if [[ ${PS0:-} == *"$__amx_ssh_preexec" && ${PS0@a} != *r* ]]; then
        PS0=${PS0%"$__amx_ssh_preexec"}
    fi
}
function __amx_ssh_prompt {
    local __amx_status=$__amx_ssh_status __amx_path=${PWD:-} __amx_encoded=''
    local __amx_ssh_path_limit='@@PATH_LIMIT@@'
    local LC_ALL=C
    # Metadata follows Bash's stderr prompt, never redirected output.
    if [[ ! -t 2 || $__amx_ssh_active != 1 ]]; then
        __amx_ssh_unwrap
        __amx_ssh_running=0
        return "$__amx_status"
    fi
    if ! shopt -q promptvars ||
        [[ -R PS0 || -R PS1 || -R PS2 ||
           ( -v PS0 && ${PS0@a} == *[raA]* ) ||
           ( -v PS1 && ${PS1@a} == *[raA]* ) ||
           ( -v PS2 && ${PS2@a} == *[raA]* ) ||
           ( ${PS1:-} != "$__amx_ssh_prefix"*"$__amx_ssh_suffix" && ${PS1:-} == *']133;'* ) ||
           ( ${PS2:-} != "$__amx_ssh_secondary"* && ${PS2:-} == *']133;'* ) ||
           ( ${PS0:-} != *"$__amx_ssh_preexec" && ${PS0:-} == *']133;'* ) ]]; then
        # Revoke on foreign ownership; unwrap only writable, owned markers.
        __amx_ssh_active=0
        __amx_ssh_unwrap
        builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@REVOKE_RECEIPT@@\a' >&2
        builtin printf '\e]1337;SetUserVar=automexia_ssh_cwd=@@CLEAR_CWD@@\a' >&2
        builtin printf '\e]1337;SetUserVar=automexia_ssh_user=@@CLEAR_USER@@\a' >&2
        builtin printf '\e]1337;SetUserVar=automexia_ssh_context=@@CLEAR_CONTEXT@@\a' >&2
        return "$__amx_status"
    fi
    [[ ${PS1:-} == "$__amx_ssh_prefix"*"$__amx_ssh_suffix" ]] || PS1=$__amx_ssh_prefix${PS1:-}$__amx_ssh_suffix
    [[ ${PS2:-} == "$__amx_ssh_secondary"* ]] || PS2=$__amx_ssh_secondary${PS2:-}
    [[ ${PS0:-} == *"$__amx_ssh_preexec" ]] || PS0=${PS0:-}$__amx_ssh_preexec
    if (( __amx_ssh_running )); then
        __amx_ssh_running=0
        builtin printf '\e]133;D;%s\a' "$__amx_status" >&2
        if (( __amx_ssh_prompt_id < 9223372036854775807 )); then
            ((__amx_ssh_prompt_id += 1))
        else
            __amx_ssh_prompt_id=1
        fi
    fi
    if (( __amx_ssh_can_cwd )); then
        if [[ ${#__amx_path} -gt $__amx_ssh_path_limit || $__amx_path != /* ||
              $__amx_path == *[[:cntrl:]]* ]]; then
            __amx_path=''
        fi
        if [[ $__amx_path != "$__amx_ssh_cwd" || -z $__amx_ssh_cwd_frame ]]; then
            # Encode only on changes; no provider scan or network operation.
            __amx_encoded=$(builtin printf 'AMXSSHCWD1|@@PANE@@|@@GENERATION@@|%s' "$__amx_path" | command base64 2>/dev/null) || __amx_encoded=''
            __amx_encoded=${__amx_encoded//$'\n'/}
            __amx_encoded=${__amx_encoded//$'\r'/}
            if [[ -n $__amx_encoded && $__amx_encoded != *[!a-zA-Z0-9+/=]* && ${#__amx_encoded} -le 5464 ]]; then
                __amx_ssh_cwd=$__amx_path
                builtin printf -v __amx_ssh_cwd_frame '\e]1337;SetUserVar=automexia_ssh_cwd=%s\a' "$__amx_encoded"
            else
                __amx_ssh_cwd='' __amx_ssh_cwd_frame=''
                __amx_ssh_can_cwd=0
                builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@DOWNGRADE_RECEIPT@@\a' >&2
                builtin printf '\e]1337;SetUserVar=automexia_ssh_cwd=@@CLEAR_CWD@@\a' >&2
            fi
        fi
        [[ -z $__amx_ssh_cwd_frame ]] || builtin printf '%s' "$__amx_ssh_cwd_frame" >&2
    fi
    builtin printf '%s' "$__amx_ssh_user_frame" >&2
    __amx_ssh_context_update
    # Separate context row; redraw/cancel/empty Enter retain the prompt identity.
    (( __amx_ssh_initializing )) || builtin printf '\e]133;A;aid=%s\a \n' "$__amx_ssh_prompt_id" >&2
    return "$__amx_status"
}
# Capture status before native hooks; wrap after dynamic prompt builders.
case "$(declare -p PROMPT_COMMAND 2>/dev/null)" in
    'declare -a PROMPT_COMMAND='*|'declare -ax PROMPT_COMMAND='*)
        PROMPT_COMMAND=(__amx_ssh_capture_status "${PROMPT_COMMAND[@]}" __amx_ssh_prompt) ;;
    *)
        if [[ -n ${PROMPT_COMMAND:-} ]]; then
            PROMPT_COMMAND=(__amx_ssh_capture_status "$PROMPT_COMMAND" __amx_ssh_prompt)
        else
            PROMPT_COMMAND=(__amx_ssh_capture_status __amx_ssh_prompt)
        fi ;;
esac
# Initial wrapping and readiness; empty Enter never invents command completion.
__amx_ssh_prompt
__amx_ssh_initializing=0
if (( __amx_ssh_can_cwd )) && [[ -n $__amx_ssh_cwd_frame ]]; then
    builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@RECEIPT@@\a' >&2
else
    __amx_ssh_can_cwd=0
    builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@PROMPT_RECEIPT@@\a' >&2
fi
