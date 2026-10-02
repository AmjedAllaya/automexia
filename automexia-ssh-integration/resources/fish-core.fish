# Session-only remote hooks; preserve the user's prompt and Fish event handlers.
status is-interactive; and isatty stderr; or return 0
string match -q '__amx_ssh_*' (set --names); and return 0
string match -q '__amx_ssh_*' (functions --names --all); and return 0
functions -q fish_prompt; or return 0
functions --copy fish_prompt __amx_ssh_native_prompt; or return 0
set -gx COLORTERM truecolor
set -gx TERM_PROGRAM Automexia
set -g __amx_ssh_aid 0
set -g __amx_ssh_new 1
set -g __amx_ssh_can_cwd 0
set -g __amx_ssh_cwd ''
set -g __amx_ssh_frame ''
set -g __amx_ssh_context_cache ''
set -g __amx_ssh_context_frame ''
set -g __amx_ssh_user_frame ''
type -q base64; and set -g __amx_ssh_can_cwd 1
function __amx_ssh_bytes
    set -l escaped (string escape --style=url -- "$argv[1]")
    set -l percent (string match --all --regex '%' -- "$escaped")
    math (string length -- "$escaped") - 2 \* (count $percent)
end
function __amx_ssh_user
    if test -n "$__amx_ssh_user_frame"
        builtin printf '%s' "$__amx_ssh_user_frame" >&2
        return 0
    end
    set -l value "$USER"
    set -l encoded ''
    if test $__amx_ssh_can_cwd = 1; and test -n "$value"; and test (string length -- "$value") -le 256; and test (__amx_ssh_bytes "$value") -le 256; and not string match -rq '[[:cntrl:]]' -- "$value"
        set encoded (builtin printf 'AMXSSHUSER1|@@PANE@@|@@GENERATION@@|%s' "$value" | command base64 2>/dev/null)
        if test $status != 0
            set encoded ''
        else
            set encoded (string join '' -- $encoded)
        end
    end
    if test -z "$encoded"; or not string match -rq '^[a-zA-Z0-9+/=]+$' -- "$encoded"; or test (string length -- "$encoded") -gt 416
        set encoded '@@CLEAR_USER@@'
    end
    set -g __amx_ssh_user_frame (builtin printf '\e]1337;SetUserVar=automexia_ssh_user=%s\a' "$encoded")
    builtin printf '%s' "$__amx_ssh_user_frame" >&2
end
function __amx_ssh_value
    set -l value ''
    for name in $argv
        if set -q $name
            set value "$$name"
            break
        end
    end
    if test (string length -- "$value") -gt 256; or test (__amx_ssh_bytes "$value") -gt 256; or string match -rq '[[:cntrl:]\x{061c}\x{200e}\x{200f}\x{202a}-\x{202e}\x{2066}-\x{2069}]' -- "$value"
        set value ''
    end
    builtin printf '%s' "$value"
end
function __amx_ssh_context
    set -l body (builtin printf 'AMXSSHCTX1|@@PANE@@|@@GENERATION@@|\ngit_branch=%s\nkubernetes_context=%s\nkubernetes_namespace=%s\ndocker_context=%s\nterraform_workspace=%s\nenvironment=%s\naws_profile=%s\nazure_cloud=%s\ngcp_project=%s\n' \
        (__amx_ssh_value GIT_BRANCH | string collect --allow-empty) (__amx_ssh_value KUBECONTEXT KUBE_CONTEXT | string collect --allow-empty) (__amx_ssh_value KUBE_NAMESPACE | string collect --allow-empty) \
        (__amx_ssh_value DOCKER_CONTEXT | string collect --allow-empty) (__amx_ssh_value TF_WORKSPACE | string collect --allow-empty) (__amx_ssh_value AUTOMEXIA_ENV ENVIRONMENT APP_ENV NODE_ENV | string collect --allow-empty) \
        (__amx_ssh_value AWS_PROFILE AWS_DEFAULT_PROFILE | string collect --allow-empty) (__amx_ssh_value AZURE_CLOUD_NAME | string collect --allow-empty) (__amx_ssh_value CLOUDSDK_CORE_PROJECT | string collect --allow-empty) | string collect)
    if test "$body" = "$__amx_ssh_context_cache"
        builtin printf '%s' "$__amx_ssh_context_frame" >&2
        return 0
    end
    set -g __amx_ssh_context_cache "$body"
    set -l encoded ''
    if test $__amx_ssh_can_cwd = 1
        set encoded (builtin printf '%s' "$body" | command base64 2>/dev/null)
        if test $status != 0
            set encoded ''
        else
            set encoded (string join '' -- $encoded)
        end
    end
    if test -z "$encoded"; or not string match -rq '^[a-zA-Z0-9+/=]+$' -- "$encoded"; or test (string length -- "$encoded") -gt 4096
        set encoded '@@CLEAR_CONTEXT@@'
    end
    set -g __amx_ssh_context_frame (builtin printf '\e]1337;SetUserVar=automexia_ssh_context=%s\a' "$encoded")
    builtin printf '%s' "$__amx_ssh_context_frame" >&2
end
function __amx_ssh_restore
    return $argv[1]
end
function __amx_ssh_preexec --on-event fish_preexec
    isatty stderr; or return 0
    builtin printf '\e]133;C\a' >&2
end
function __amx_ssh_postexec --on-event fish_postexec
    set -l saved $status
    set -g __amx_ssh_new 1
    isatty stderr; and builtin printf '\e]133;D;%s\a' $saved >&2
end
function fish_prompt
    set -l saved $status
    if not isatty stderr
        __amx_ssh_restore $saved
        __amx_ssh_native_prompt
        return
    end
    __amx_ssh_user
    __amx_ssh_context
    if test $__amx_ssh_can_cwd = 1
        set -l path "$PWD"
        if test (string length -- "$path") -gt @@PATH_LIMIT@@; or test (__amx_ssh_bytes "$path") -gt @@PATH_LIMIT@@; or string match -rq '[[:cntrl:]]' -- "$path"
            set path ''
        end
        if test "$path" != "$__amx_ssh_cwd"; or test -z "$__amx_ssh_frame"
            set -l encoded (builtin printf 'AMXSSHCWD1|@@PANE@@|@@GENERATION@@|%s' "$path" | command base64 2>/dev/null)
            if test $status != 0
                set encoded ''
            else
                set encoded (string join '' -- $encoded)
            end
            if test -n "$encoded"; and string match -rq '^[a-zA-Z0-9+/=]+$' -- "$encoded"; and test (string length -- "$encoded") -le 5464
                set -g __amx_ssh_cwd "$path"
                set -g __amx_ssh_frame (builtin printf '\e]1337;SetUserVar=automexia_ssh_cwd=%s\a' "$encoded")
            else
                set -g __amx_ssh_can_cwd 0
                set -g __amx_ssh_frame ''
                builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@DOWNGRADE_RECEIPT@@\a\e]1337;SetUserVar=automexia_ssh_cwd=@@CLEAR_CWD@@\a' >&2
            end
        end
        test -z "$__amx_ssh_frame"; or builtin printf '%s' "$__amx_ssh_frame" >&2
    end
    if test $__amx_ssh_new = 1
        set -g __amx_ssh_aid (math $__amx_ssh_aid + 1)
        set -g __amx_ssh_new 0
        builtin printf '\e]133;A;aid=%s\a \n' $__amx_ssh_aid >&2
    end
    builtin printf '\e]133;P;k=c;aid=%s\a' $__amx_ssh_aid
    __amx_ssh_restore $saved
    __amx_ssh_native_prompt
    builtin printf '\e]133;B\a'
end
if test $__amx_ssh_can_cwd = 1
    builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@RECEIPT@@\a' >&2
else
    builtin printf '\e]1337;SetUserVar=automexia_ssh_ready=@@PROMPT_RECEIPT@@\a' >&2
end
__amx_ssh_user
