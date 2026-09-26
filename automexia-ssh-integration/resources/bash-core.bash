# Automexia SSH protocol-1 optional, session-only Bash core. Not a launch grant.
# PS1 remains the native prompt. No DEBUG/EXIT trap, PS0, aliases or editor hooks.
[[ $- == *i* && -t 2 ]] || return 0
(( BASH_VERSINFO[0] > 5 || (BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] >= 1) )) || return 0
shopt -q restricted_shell && return 0
[[ $- != *e* ]] || return 0
# Refuse collisions BEFORE touching globals, including readonly and nameref names.
# A repeated source is inert as the owned namespace is already populated.
[[ -z $(builtin compgen -A variable __amx_ssh_) && -z $(builtin compgen -A function __amx_ssh_) && -z $(builtin compgen -A alias __amx_ssh_) ]] || return 0
case "$(declare -p PS1 2>/dev/null)" in
    'declare -- PS1='*|'declare -x PS1='*|'') ;;
    *) return 0 ;;
esac
case "$(declare -p PROMPT_COMMAND 2>/dev/null)" in
    'declare -- PROMPT_COMMAND='*|'declare -x PROMPT_COMMAND='*|'declare -a PROMPT_COMMAND='*|'declare -ax PROMPT_COMMAND='*|'') ;;
    *) return 0 ;;
esac
[[ ${PS1:-} != *']133;'* ]] || return 0
__amx_ssh_core_loaded=1
__amx_ssh_prefix='\[\e]133;A\a\]'
__amx_ssh_suffix='\[\e]133;B\a\]'
__amx_ssh_cwd=''
__amx_ssh_cwd_frame=''
__amx_ssh_can_cwd=0
__amx_ssh_active=1
command -v base64 >/dev/null 2>&1 && __amx_ssh_can_cwd=1
function __amx_ssh_prompt {
    local __amx_status=$? __amx_path=${PWD:-} __amx_encoded=''
    local LC_ALL=C
    # Bash writes its prompt to stderr. Keep metadata on that same terminal;
    # stdout redirection must never capture control sequences from this hook.
    [[ -t 2 && $__amx_ssh_active == 1 ]] || return "$__amx_status"
    if [[ $PS1 == "$__amx_ssh_prefix"*"$__amx_ssh_suffix" ]]; then
        : # Already wrapped: no repeated prefix allocation on stable prompts.
    elif [[ $PS1 != *']133;'* && ${PS1@a} != *r* ]]; then
        PS1=$__amx_ssh_prefix$PS1$__amx_ssh_suffix
    else
        # A later native integration owns PS1. Do not double-wrap its markers.
        __amx_ssh_active=0
        builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@REVOKE_RECEIPT@@\a' >&2
        builtin printf '\e]1337;SetUserVar=automexia_ssh_cwd=@@CLEAR_CWD@@\a' >&2
        return "$__amx_status"
    fi
    if (( __amx_ssh_can_cwd )); then
        if [[ ${#__amx_path} -gt @@PATH_LIMIT@@ || $__amx_path != /* ||
              $__amx_path == *[[:cntrl:]]* ]]; then
            __amx_path=''
        fi
        if [[ $__amx_path != "$__amx_ssh_cwd" || -z $__amx_ssh_cwd_frame ]]; then
            # Reuse the installed Base64 utility only on CWD changes. No custom
            # encoder, provider scan or network operation runs at each prompt.
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
    return "$__amx_status"
}
# Run AFTER native hooks so dynamic prompt builders cannot erase our wrapping.
# The first native hook still receives the command's original exit status.
# Bash >=5.1 supports the array form. Sparse indices retain value order.
case "$(declare -p PROMPT_COMMAND 2>/dev/null)" in
    'declare -a PROMPT_COMMAND='*|'declare -ax PROMPT_COMMAND='*)
        PROMPT_COMMAND=("${PROMPT_COMMAND[@]}" __amx_ssh_prompt) ;;
    *)
        if [[ -n ${PROMPT_COMMAND:-} ]]; then
            PROMPT_COMMAND=("$PROMPT_COMMAND" __amx_ssh_prompt)
        else
            PROMPT_COMMAND=(__amx_ssh_prompt)
        fi ;;
esac
# Initial wrapping and one advisory readiness message; no command completion is
# fabricated for empty Enter. Live receipt routing remains disabled in the app.
__amx_ssh_prompt
if (( __amx_ssh_can_cwd )) && [[ -n $__amx_ssh_cwd_frame ]]; then
    builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@RECEIPT@@\a' >&2
else
    __amx_ssh_can_cwd=0
    builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@PROMPT_RECEIPT@@\a' >&2
fi
