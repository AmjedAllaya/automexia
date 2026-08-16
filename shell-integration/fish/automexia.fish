# Automexia Fish integration. Fish retains ownership of its prompt, editor,
# history, autosuggestions, key bindings, and candidate presentation.
set -l automexia_term_program ''
set -l automexia_shell_integration ''
set -q TERM_PROGRAM; and set automexia_term_program "$TERM_PROGRAM"
set -q AUTOMEXIA_SHELL_INTEGRATION; and set automexia_shell_integration "$AUTOMEXIA_SHELL_INTEGRATION"
if test "$automexia_term_program" != Automexia; and test "$automexia_shell_integration" != 1
    return 0
end
set -q AUTOMEXIA_FISH_INTEGRATION_LOADED; and return 0
set -gx AUTOMEXIA_FISH_INTEGRATION_LOADED 1
set -gx AUTOMEXIA_SHELL_INTEGRATION 1
set -gx TERM_PROGRAM Automexia
set -gx COLORTERM truecolor

function __automexia_fish_prompt --on-event fish_prompt
    set -l encoded_path (string replace -a ' ' '%20' -- "$PWD")
    printf '\e]7;file://localhost%s\a' "$encoded_path"
    printf '\e]1337;SetUserVar=automexia_shell=MQ==\a'
    printf '\e]1337;SetUserVar=automexia_shell_name=ZmlzaA==\a'
end

function __automexia_fish_preexec --on-event fish_preexec
    printf '\e]133;C\a'
end

function __automexia_fish_postexec --on-event fish_postexec
    # fish_postexec receives the expanded command line, not the exit code. Capture
    # $status before any Fish builtin can overwrite it.
    set -l status_code $status
    printf '\e]133;D;%s\a' "$status_code"
end

set -l integration_dir (path dirname (status filename))
set -l completion_adapter "$integration_dir/automexia-completion.fish"
if not test -r "$completion_adapter"
    set completion_adapter "$integration_dir/../completion/fish/automexia-completion.fish"
end
test -r "$completion_adapter"; and source "$completion_adapter"
