# Sourced only by the opted-in helper, after the canonical core owns the prompt.
[[ ${__amx_ssh_active:-0} == 1 ]] || return 0
builtin declare -F __amx_ssh_helper_prompt >/dev/null && return 0
[[ ${AMX_SSH_HELPER_FD:-} =~ ^[1-9][0-9]{1,8}$ ]] || return 0
builtin printf '' 2>/dev/null 1>&"$AMX_SSH_HELPER_FD" || return 0
__amx_ssh_helper_fd=$AMX_SSH_HELPER_FD
builtin unset AMX_SSH_HELPER_FD
__amx_ssh_helper_revision=0
__amx_ssh_helper_disabled=0
__amx_ssh_helper_body=''

__amx_ssh_helper_response_fd=''
if [[ ${AMX_SSH_HELPER_RESPONSE_FD:-} =~ ^[1-9][0-9]{1,8}$ ]] &&
   builtin : 2>/dev/null <&"$AMX_SSH_HELPER_RESPONSE_FD"; then
    __amx_ssh_helper_response_fd=$AMX_SSH_HELPER_RESPONSE_FD
fi
builtin unset AMX_SSH_HELPER_RESPONSE_FD

# Only the short ASCII revision envelope is encoded here; no external encoder,
# filesystem access or provider process runs inside this prompt callback.
function __amx_ssh_helper_encode {
    local LC_ALL=C text=$1 alphabet=ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/
    local i a b c count=${#1}
    __amx_ssh_helper_encoded=''
    for ((i=0; i<count; i+=3)); do
        builtin printf -v a '%d' "'${text:i:1}"
        b=0 c=0
        ((i+1<count)) && builtin printf -v b '%d' "'${text:i+1:1}"
        ((i+2<count)) && builtin printf -v c '%d' "'${text:i+2:1}"
        __amx_ssh_helper_encoded+=${alphabet:$((a>>2)):1}${alphabet:$((((a&3)<<4)|(b>>4))):1}
        if ((i+1<count)); then
            __amx_ssh_helper_encoded+=${alphabet:$((((b&15)<<2)|(c>>6))):1}
        else
            __amx_ssh_helper_encoded+='='
        fi
        if ((i+2<count)); then
            __amx_ssh_helper_encoded+=${alphabet:$((c&63)):1}
        else
            __amx_ssh_helper_encoded+='='
        fi
    done
}

function __amx_ssh_helper_retire {
    __amx_ssh_helper_disabled=1
    builtin printf '\e]1337;SetUserVar=automexia_ssh_revision=\a' >&2
}

# Darwin's terminal writes may interleave under backpressure. Only this
# foreground prompt publishes helper metadata there. The inherited nonblocking
# response stream is read only after its complete-frame datagram notification;
# no subprocess, filesystem operation or wait runs in this callback.
function __amx_ssh_helper_response {
    local LC_ALL=C ack frame payload latest='' attempt
    [[ -n $__amx_ssh_helper_response_fd ]] || return 0
    for attempt in 1 2 3 4; do
        ack=''
        if ! builtin read -r -n 1 -u "$__amx_ssh_helper_fd" ack 2>/dev/null; then
            break
        fi
        frame=''
        if [[ $ack != 1 ]] ||
           ! builtin read -r -d "" -n 6145 -u "$__amx_ssh_helper_response_fd" frame 2>/dev/null ||
           (( ${#frame} > 6144 )); then
            __amx_ssh_helper_retire
            return 0
        fi
        case $frame in
            $'\e]1337;SetUserVar=automexia_ssh_context_v2='*$'\a')
                payload=${frame#$'\e]1337;SetUserVar=automexia_ssh_context_v2='}
                payload=${payload%$'\a'}
                if [[ -z $payload || $payload == *[^a-zA-Z0-9+/=]* ]]; then
                    __amx_ssh_helper_retire
                    return 0
                fi
                ;;
            $'\e]1337;SetUserVar=automexia_ssh_revision=\a') ;;
            *)
                __amx_ssh_helper_retire
                return 0
                ;;
        esac
        # Release one producer slot only after consuming the entire frame.
        # The writer coalesces idle refreshes while this acknowledgement waits.
        if ! builtin printf '1' 2>/dev/null 1>&"$__amx_ssh_helper_response_fd"; then
            __amx_ssh_helper_retire
            return 0
        fi
        latest=$frame
    done
    [[ -z $latest ]] || builtin printf '%s' "$latest" >&2
    return 0
}

function __amx_ssh_helper_prompt {
    local LC_ALL=C name value body='' invalid=0 limit header
    local -a names=(cwd HOME KUBECONFIG HOMEDRIVE HOMEPATH USERPROFILE DOCKER_CONTEXT DOCKER_HOST_PRESENT AWS_PROFILE AWS_DEFAULT_PROFILE AWS_REGION AWS_DEFAULT_REGION AZURE_CLOUD_NAME CLOUDSDK_ACTIVE_CONFIG_NAME CLOUDSDK_CORE_PROJECT CLOUDSDK_COMPUTE_REGION TF_WORKSPACE AUTOMEXIA_ENV ENVIRONMENT APP_ENV NODE_ENV GIT_BRANCH KUBECONTEXT KUBE_CONTEXT KUBE_NAMESPACE)
    [[ -t 2 && $__amx_ssh_helper_disabled == 0 ]] || return 0
    for name in "${names[@]}"; do
        limit=256
        case $name in
            cwd) value=${PWD-}; limit=4096 ;;
            HOME|KUBECONFIG|HOMEDRIVE|HOMEPATH|USERPROFILE) value=${!name-}; limit=4096 ;;
            DOCKER_HOST_PRESENT) value=0; [[ -n ${DOCKER_HOST-} ]] && value=1 ;;
            *) value=${!name-} ;;
        esac
        if [[ ${#value} -gt $limit || $value == *[[:cntrl:]]* ||
              $value == *$'\xc2'[$'\x80'-$'\x9f']* || $value == *$'\xd8\x9c'* ||
              $value == *$'\xe2\x80'[$'\x8e'$'\x8f'$'\xaa'-$'\xae']* ||
              $value == *$'\xe2\x81'[$'\xa6'-$'\xa9']* ]]; then
            invalid=1
            break
        fi
        body+="$name=$value"$'\n'
    done
    header=$'AMXREQ1|@@PANE@@|@@GENERATION@@|4294967295\n'
    if (( invalid || ${#header}+${#body}+2 > 16384 )); then
        body=''
        for name in "${names[@]}"; do body+="$name="$'\n'; done
    fi
    if [[ $body != "$__amx_ssh_helper_body" ]]; then
        if (( __amx_ssh_helper_revision >= 4294967295 )); then
            __amx_ssh_helper_retire
            return 0
        fi
        __amx_ssh_helper_body=$body
        __amx_ssh_helper_revision=$((__amx_ssh_helper_revision+1))
        __amx_ssh_helper_encode "AMXSSHREV1|@@PANE@@|@@GENERATION@@|$__amx_ssh_helper_revision"
    fi
    header="AMXREQ1|@@PANE@@|@@GENERATION@@|$__amx_ssh_helper_revision"$'\n'
    # Invalidate prior results before submitting the new bounded request.
    builtin printf '\e]1337;SetUserVar=automexia_ssh_revision=%s\a' "$__amx_ssh_helper_encoded" >&2
    if ! builtin printf '\0%s%s\0' "$header" "$body" 2>/dev/null 1>&"$__amx_ssh_helper_fd"; then
        __amx_ssh_helper_retire
    fi
    if [[ $__amx_ssh_helper_disabled == 0 ]]; then
        __amx_ssh_helper_response
    fi
    return 0
}

__amx_ssh_helper_prompt
