# Session-only remote hooks. Native prompt/profile/editor ownership stays in Zsh.
[[ -o interactive && -t 2 && ! -o restricted ]] || return 0
[[ -z ${(k)parameters[(I)__amx_ssh_*]} && -z ${(k)functions[(I)__amx_ssh_*]} && -z ${(k)aliases[(I)__amx_ssh_*]} ]] || return 0
[[ ${(t)PROMPT} != *readonly* && $PROMPT != *']133;'* ]] || return 0
[[ ${(t)COLORTERM} == (|scalar|scalar-export) && ${(t)TERM_PROGRAM} == (|scalar|scalar-export) ]] || return 0
export COLORTERM=truecolor TERM_PROGRAM=Automexia
typeset -g __amx_ssh_pending=0 __amx_ssh_aid=0 __amx_ssh_active=1
typeset -g __amx_ssh_cwd='' __amx_ssh_frame='' __amx_ssh_prefix='' __amx_ssh_suffix=$'%{\e]133;B\a%}'
typeset -g __amx_ssh_can_cwd=0
typeset -g __amx_ssh_context_cache='' __amx_ssh_context_frame='' __amx_ssh_user_frame=''
(( $+commands[base64] )) && __amx_ssh_can_cwd=1
function __amx_ssh_user {
    emulate -L zsh
    if [[ -n $__amx_ssh_user_frame ]]; then
        builtin printf '%s' "$__amx_ssh_user_frame" >&2
        return 0
    fi
    local LC_ALL=C
    local value=${USERNAME-${USER-${LOGNAME-}}} encoded=''
    if (( __amx_ssh_can_cwd )) && [[ -n $value && ${#value} -le 256 && $value != *[[:cntrl:]]* ]]; then
        encoded=$(builtin printf 'AMXSSHUSER1|@@PANE@@|@@GENERATION@@|%s' "$value" | command base64 2>/dev/null) || encoded=''
        encoded=${encoded//$'\n'/}
        encoded=${encoded//$'\r'/}
    fi
    [[ -n $encoded && $encoded != *[^a-zA-Z0-9+/=]* && ${#encoded} -le 416 ]] || encoded='@@CLEAR_USER@@'
    builtin printf -v __amx_ssh_user_frame '\e]1337;SetUserVar=automexia_ssh_user=%s\a' "$encoded"
    builtin printf '%s' "$__amx_ssh_user_frame" >&2
}
function __amx_ssh_context {
    emulate -L zsh
    if (( $+functions[__amx_ssh_helper_prompt] )); then
        __amx_ssh_helper_prompt
        return 0
    fi
    local LC_ALL=C
    local body=$'AMXSSHCTX1|@@PANE@@|@@GENERATION@@|\n' encoded='' value i
    local -a names=(git_branch kubernetes_context kubernetes_namespace docker_context terraform_workspace environment aws_profile azure_cloud gcp_project)
    local -a values=("${GIT_BRANCH-}" "${KUBECONTEXT-${KUBE_CONTEXT-}}" "${KUBE_NAMESPACE-}" "${DOCKER_CONTEXT-}" "${TF_WORKSPACE-}" "${AUTOMEXIA_ENV-${ENVIRONMENT-${APP_ENV-${NODE_ENV-}}}}" "${AWS_PROFILE-${AWS_DEFAULT_PROFILE-}}" "${AZURE_CLOUD_NAME-}" "${CLOUDSDK_CORE_PROJECT-}")
    for (( i=1; i<=9; i++ )); do
        value=$values[$i]
        if [[ ${#value} -gt 256 || $value == *[[:cntrl:]]* || $value == *$'\xc2'[$'\x80'-$'\x9f']* || $value == *$'\xd8\x9c'* || $value == *$'\xe2\x80'[$'\x8e'$'\x8f'$'\xaa'-$'\xae']* || $value == *$'\xe2\x81'[$'\xa6'-$'\xa9']* ]]; then
            value=''
        fi
        body+="$names[$i]=$value"$'\n'
    done
    if [[ $body == "$__amx_ssh_context_cache" ]]; then
        builtin printf '%s' "$__amx_ssh_context_frame" >&2
        return 0
    fi
    __amx_ssh_context_cache=$body
    if (( __amx_ssh_can_cwd )); then
        encoded=$(builtin printf '%s' "$body" | command base64 2>/dev/null) || encoded=''
        encoded=${encoded//$'\n'/}
        encoded=${encoded//$'\r'/}
    fi
    [[ -n $encoded && $encoded != *[^a-zA-Z0-9+/=]* && ${#encoded} -le 4096 ]] || encoded='@@CLEAR_CONTEXT@@'
    builtin printf -v __amx_ssh_context_frame '\e]1337;SetUserVar=automexia_ssh_context=%s\a' "$encoded"
    builtin printf '%s' "$__amx_ssh_context_frame" >&2
}
function __amx_ssh_preexec {
    [[ -t 2 && $__amx_ssh_active == 1 ]] || return 0
    __amx_ssh_pending=1
    builtin printf '\e]133;C\a' >&2
}
function __amx_ssh_precmd {
    local __amx_status=$?
    emulate -L zsh
    local __amx_path=$PWD __amx_encoded=''
    local LC_ALL=C
    [[ -t 2 && $__amx_ssh_active == 1 ]] || return 0
    if [[ -n $__amx_ssh_prefix && $PROMPT == "$__amx_ssh_prefix"*"$__amx_ssh_suffix" ]]; then
        PROMPT=${PROMPT#"$__amx_ssh_prefix"}
        PROMPT=${PROMPT%"$__amx_ssh_suffix"}
    elif [[ $PROMPT == *']133;'* || ${(t)PROMPT} == *readonly* ]]; then
        __amx_ssh_active=0
        builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@REVOKE_RECEIPT@@\a\e]1337;SetUserVar=automexia_ssh_cwd=@@CLEAR_CWD@@\a' >&2
        return 0
    fi
    if (( __amx_ssh_pending )); then
        builtin printf '\e]133;D;%s\a' "$__amx_status" >&2
        __amx_ssh_pending=0
    fi
    __amx_ssh_user
    __amx_ssh_context
    if (( __amx_ssh_can_cwd )); then
        [[ ${#__amx_path} -le @@PATH_LIMIT@@ && $__amx_path == /* && $__amx_path != *[[:cntrl:]]* ]] || __amx_path=''
        if [[ $__amx_path != "$__amx_ssh_cwd" || -z $__amx_ssh_frame ]]; then
            __amx_encoded=$(builtin printf 'AMXSSHCWD1|@@PANE@@|@@GENERATION@@|%s' "$__amx_path" | command base64 2>/dev/null) || __amx_encoded=''
            __amx_encoded=${__amx_encoded//$'\n'/}
            __amx_encoded=${__amx_encoded//$'\r'/}
            if [[ -n $__amx_encoded && $__amx_encoded != *[^a-zA-Z0-9+/=]* && ${#__amx_encoded} -le 5464 ]]; then
                __amx_ssh_cwd=$__amx_path
                builtin printf -v __amx_ssh_frame '\e]1337;SetUserVar=automexia_ssh_cwd=%s\a' "$__amx_encoded"
            else
                __amx_ssh_can_cwd=0
                __amx_ssh_frame=''
                builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@DOWNGRADE_RECEIPT@@\a\e]1337;SetUserVar=automexia_ssh_cwd=@@CLEAR_CWD@@\a' >&2
            fi
        fi
        [[ -z $__amx_ssh_frame ]] || builtin printf '%s' "$__amx_ssh_frame" >&2
    fi
    (( __amx_ssh_aid += 1 ))
    builtin printf '\e]133;A;aid=%s\a \n' "$__amx_ssh_aid" >&2
    builtin printf -v __amx_ssh_prefix '%%{\e]133;P;k=c;aid=%s\a%%}' "$__amx_ssh_aid"
    PROMPT=$__amx_ssh_prefix$PROMPT$__amx_ssh_suffix
    return 0
}
preexec_functions+=(__amx_ssh_preexec)
precmd_functions+=(__amx_ssh_precmd)
if (( __amx_ssh_can_cwd )); then
    builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@RECEIPT@@\a' >&2
else
    builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@PROMPT_RECEIPT@@\a' >&2
fi
__amx_ssh_user
