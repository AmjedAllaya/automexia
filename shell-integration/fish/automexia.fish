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
set -g AUTOMEXIA_FISH_INTEGRATION_LOADED 1
set -gx AUTOMEXIA_SHELL_INTEGRATION 1
set -gx TERM_PROGRAM Automexia
set -gx COLORTERM truecolor
if not set -q AUTOMEXIA_AMX; or test "$AUTOMEXIA_AMX" != 0
    if not functions -q amx
        function amx --description 'Automexia commands, including google search'
            set -l binary (command -s amx)
            if test (count $binary) -gt 0
                command "$binary" $argv
                return $status
            end
            if set -q AUTOMEXIA_CLI
                set binary "$AUTOMEXIA_CLI"
            end
            if test (count $binary) -eq 0
                set binary (command -s automexia)
            end
            if test (count $binary) -eq 0; or not test -x "$binary"
                printf '%s\n' 'amx: Automexia command unavailable; reopen a current Automexia session.' >&2
                return 127
            end
            if string match -q '*.exe' -- "$binary"; and set -q WSL_DISTRO_NAME
                command "$binary" --amx-wsl-distribution "$WSL_DISTRO_NAME" --amx-wsl-cwd "$PWD" --amx-wsl-path (string join ':' -- $PATH) --amx-wsl-home "$HOME" $argv
            else
                command "$binary" $argv
            end
        end
    end
end
set -g __automexia_fish_prompt_generation 0

function __automexia_hint_encode --argument-names value
    test -n "$value"; or return 0
    test (string length -- "$value") -le 4096; or return 1
    string match -rq '[\x00-\x1f\x7f-\x9f]' -- "$value"; and return 1
    command -q base64; and command -q tr; or return 1
    set -l encoded (printf '%s' "$value" | command base64 2>/dev/null | command tr -d '\r\n')
    contains -- 1 $pipestatus; and return 1
    # Fish counts Unicode scalars. Base64 length plus padding independently
    # bounds the original UTF-8 payload to 4096 bytes on older Fish versions.
    set -l size (string length -- "$encoded")
    test $size -gt 0; and test $size -le 5464; or return 1
    if test $size -eq 5464; and not string match -q '*==' -- "$encoded"
        return 1
    end
    printf '%s' "$encoded"
end

function __automexia_publish_location_hints
    set -l hint_home "$HOME"
    set -l hint_config ''
    if set -q -x KUBECONFIG
        if not set -q KUBECONFIG[2]
            set hint_config "$KUBECONFIG"
        else if set -q --path KUBECONFIG
            set hint_config (string join ':' -- $KUBECONFIG | string collect --allow-empty)
        else
            set hint_config (string join ' ' -- $KUBECONFIG | string collect --allow-empty)
        end
    end
    if test "$AUTOMEXIA_CONTEXT_PATH_HINTS" = 0; or \
            test (string length -- "$hint_home") -gt 4096; or \
            test (string length -- "$hint_config") -gt 4096
        set hint_home ''; set hint_config ''
    end
    if not set -q __automexia_location_ready; or \
            test "$hint_home" != "$__automexia_location_home"; or \
            test "$hint_config" != "$__automexia_location_config"
        set -g __automexia_location_home "$hint_home"
        set -g __automexia_location_config "$hint_config"
        set -g __automexia_home_encoded (__automexia_hint_encode "$hint_home")
        set -l home_result $status
        set -g __automexia_config_encoded (__automexia_hint_encode "$hint_config")
        set -l config_result $status
        if test $home_result -ne 0; or test $config_result -ne 0
            set -g __automexia_home_encoded ''; set -g __automexia_config_encoded ''
        end
        set -g __automexia_location_ready 1
    end
    printf '\e]1337;SetUserVar=automexia_env_HOME=%s\a' "$__automexia_home_encoded"
    printf '\e]1337;SetUserVar=automexia_env_KUBECONFIG=%s\a' "$__automexia_config_encoded"
end

# Fish owns these existing identity fields too. Cache their encoding once, then
# restore the parent identity after nested Bash/Zsh/WSL sessions on every prompt.
set -g __automexia_fish_distro (__automexia_hint_encode "$WSL_DISTRO_NAME")
set -g __automexia_fish_user (__automexia_hint_encode "$USER")
set -g __automexia_fish_shell (__automexia_hint_encode (status fish-path))

function __automexia_fish_prompt --on-event fish_prompt
    printf '\e]1337;SetUserVar=automexia_env_pending=MQ==\a'
    printf '\e]1337;SetUserVar=automexia_distro=%s\a' "$__automexia_fish_distro"
    printf '\e]1337;SetUserVar=automexia_shell_user=%s\a' "$__automexia_fish_user"
    printf '\e]1337;SetUserVar=automexia_shell_path=%s\a' "$__automexia_fish_shell"
    printf '\e]1337;SetUserVar=automexia_os_version=\a'
    __automexia_publish_location_hints
    printf '\e]1337;SetUserVar=automexia_env_pending=MA==\a'
    set -g __automexia_fish_prompt_generation \
        (math --scale 0 "$__automexia_fish_prompt_generation + 1")
    printf '\e]133;A;aid=%s\a' "$__automexia_fish_prompt_generation"
    set -l encoded_path (string replace -a ' ' '%20' -- "$PWD")
    printf '\e]7;file://localhost%s\a' "$encoded_path"
    printf '\e]1337;SetUserVar=automexia_shell=MQ==\a'
    printf '\e]1337;SetUserVar=automexia_shell_name=ZmlzaA==\a'
end

function __automexia_fish_preexec --on-event fish_preexec
    printf '\e]133;C\a'
end

function __automexia_fish_finish --argument-names status_code
    printf '\e]133;D;%s\a' "$status_code"
end

function __automexia_fish_postexec --on-event fish_postexec
    # Event arguments contain the expanded command line, not the exit code.
    # Capture $status before any Fish builtin can overwrite it.
    set -l status_code $status
    __automexia_fish_finish "$status_code"
end

# Fish 4.8+ publishes syntax failures separately. Older supported Fish versions
# accept the named handler and simply never emit the event.
function __automexia_fish_posterror --on-event fish_posterror
    set -l status_code $status
    __automexia_fish_finish "$status_code"
end

set -l integration_dir (path dirname (status filename))
set -l completion_adapter "$integration_dir/automexia-completion.fish"
if not test -r "$completion_adapter"
    set completion_adapter "$integration_dir/../completion/fish/automexia-completion.fish"
end
test -r "$completion_adapter"; and source "$completion_adapter"
# CP3.1 persistent aliases consume one private immutable generation. Fish
# abbreviations/functions are fingerprinted so explicit reload is exact.
if not set -q __automexia_alias_loader_initialized
    set -g __automexia_alias_loader_initialized 1
    if set -q AUTOMEXIA_CONFIG_HOME; and test -n "$AUTOMEXIA_CONFIG_HOME"
        set -g __automexia_alias_config_root "$AUTOMEXIA_CONFIG_HOME"
    else if test (uname -s 2>/dev/null) = Darwin
        set -g __automexia_alias_config_root "$HOME/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal"
    else if set -q XDG_CONFIG_HOME; and test -n "$XDG_CONFIG_HOME"
        set -g __automexia_alias_config_root "$XDG_CONFIG_HOME/automexia"
    else
        set -g __automexia_alias_config_root "$HOME/.config/automexia"
    end
    set -g __automexia_alias_root "$__automexia_alias_config_root/generated/aliases"
    set -g __automexia_alias_state uninitialized
    set -g __automexia_alias_reason
    set -g __automexia_alias_generation
    set -g __automexia_alias_loaded_path
    set -g __automexia_alias_record_kinds
    set -g __automexia_alias_record_names
    set -g __automexia_alias_record_definitions
    set -g __automexia_alias_collisions

    function __automexia_alias_hash_stream
        if type -q sha256sum
            command sha256sum | read -l digest rest
            echo $digest
        else if type -q shasum
            command shasum -a 256 | read -l digest rest
            echo $digest
        else if type -q openssl
            command openssl dgst -sha256 | read -l label digest
            echo $digest
        else
            return 127
        end
    end

    function __automexia_alias_hash_file --argument-names file
        if type -q sha256sum
            command sha256sum -- "$file" | read -l digest rest
            echo $digest
        else if type -q shasum
            command shasum -a 256 -- "$file" | read -l digest rest
            echo $digest
        else if type -q openssl
            command openssl dgst -sha256 "$file" | read -l label digest
            echo $digest
        else
            return 127
        end
    end

    function __automexia_alias_stat_metadata
        set -g __automexia_alias_stat_reply
        set -g __automexia_alias_stat_reply (command stat -c '%a|%s' -- $argv 2>/dev/null)
        or set -g __automexia_alias_stat_reply (command stat -f '%Lp|%z' $argv 2>/dev/null)
        test (count $__automexia_alias_stat_reply) -eq (count $argv)
    end

    function __automexia_alias_verify_active_entries --argument-names pointer directory manifest shell_directory artifact
        set -l generated "$__automexia_alias_config_root/generated"
        set -l generations "$__automexia_alias_root/generations"
        for candidate in "$generated" "$__automexia_alias_root" "$generations" \
                "$directory" "$shell_directory"
            test -d "$candidate"; and not test -L "$candidate"; or return 1
        end
        for candidate in "$pointer" "$manifest" "$artifact"
            test -f "$candidate"; and not test -L "$candidate"; or return 1
        end
        set -l verified (command sh -c '
            if metadata=$(stat -c "%a|%s" -- "$@" 2>/dev/null); then
                :
            elif metadata=$(stat -f "%Lp|%z" "$@" 2>/dev/null); then
                :
            else
                exit 1
            fi
            printf "%s\n" "$metadata" | while IFS= read -r line; do
                printf "M|%s\n" "$line"
            done
            if command -v sha256sum >/dev/null 2>&1; then
                sha256sum -- "$7" "$8" | while IFS=" " read -r digest rest; do
                    printf "H|%s\n" "$digest"
                done
            elif command -v shasum >/dev/null 2>&1; then
                shasum -a 256 -- "$7" "$8" | while IFS=" " read -r digest rest; do
                    printf "H|%s\n" "$digest"
                done
            elif command -v openssl >/dev/null 2>&1; then
                openssl dgst -sha256 "$7" "$8" | while read -r label digest; do
                    printf "H|%s\n" "$digest"
                done
            else
                exit 127
            fi
        ' automexia-alias-verify "$generated" "$__automexia_alias_root" \
            "$generations" "$pointer" "$directory" "$shell_directory" \
            "$manifest" "$artifact"); or return 1
        test (count $verified) -eq 10; or return 1
        for index in 1 2 3 4 5 6 7 8
            set -l fields (string split '|' -- "$verified[$index]")
            test (count $fields) -eq 3; and test "$fields[1]" = M; and \
                string match -qr '^[0-7]{3,4}$' -- "$fields[2]"; and \
                test (string sub -s -2 -- "$fields[2]") = 00; and \
                string match -qr '^[0-9]+$' -- "$fields[3]"; or return 1
        end
        set -l pointer_fields (string split '|' -- "$verified[4]")
        set -l manifest_fields (string split '|' -- "$verified[7]")
        set -l artifact_fields (string split '|' -- "$verified[8]")
        test "$pointer_fields[3]" -le 80; and \
            test "$manifest_fields[3]" -le 65536; and \
            test "$artifact_fields[3]" -le 1114112; or return 1
        set -l manifest_hash (string split '|' -- "$verified[9]")
        set -l artifact_hash (string split '|' -- "$verified[10]")
        test (count $manifest_hash) -eq 2; and test "$manifest_hash[1]" = H; and \
            string match -qr '^[0-9a-f]{64}$' -- "$manifest_hash[2]"; and \
            test (count $artifact_hash) -eq 2; and test "$artifact_hash[1]" = H; and \
            string match -qr '^[0-9a-f]{64}$' -- "$artifact_hash[2]"; or return 1
        set -g __automexia_alias_hash_pair_reply "$manifest_hash[2]" "$artifact_hash[2]"
    end
    function __automexia_alias_private_root_entries --argument-names pointer
        set -l generated "$__automexia_alias_config_root/generated"
        set -l generations "$__automexia_alias_root/generations"
        for directory in "$generated" "$__automexia_alias_root" "$generations"
            test -d "$directory"; and not test -L "$directory"; or return 1
        end
        test -f "$pointer"; and not test -L "$pointer"; or return 1
        __automexia_alias_stat_metadata "$generated" "$__automexia_alias_root" \
            "$generations" "$pointer"; or return 1
        for index in 1 2 3
            set -l fields (string split '|' -- "$__automexia_alias_stat_reply[$index]")
            test (count $fields) -eq 2; and \
                string match -qr '^[0-7]{3,4}$' -- "$fields[1]"; and \
                test (string sub -s -2 -- "$fields[1]") = 00; or return 1
        end
        set -l pointer_fields (string split '|' -- "$__automexia_alias_stat_reply[4]")
        test (count $pointer_fields) -eq 2; and \
            string match -qr '^[0-7]{3,4}$' -- "$pointer_fields[1]"; and \
            test (string sub -s -2 -- "$pointer_fields[1]") = 00; and \
            string match -qr '^[0-9]+$' -- "$pointer_fields[2]"; and \
            test "$pointer_fields[2]" -le 80
    end

    function __automexia_alias_definition_text --argument-names kind name
        switch "$kind"
            case A
                abbr --show "$name" 2>/dev/null | string collect
            case F
                functions "$name" 2>/dev/null | string collect
            case '*'
                return 1
        end
    end

    function __automexia_alias_owned_public --argument-names requested
        set -l index (contains -i -- "$requested" $__automexia_alias_record_names)
        test -n "$index"; or return 1
        set -l actual (__automexia_alias_definition_text \
            "$__automexia_alias_record_kinds[$index]" "$requested")
        test -n "$actual"; and \
            test "$actual" = "$__automexia_alias_record_definitions[$index]"
    end

    function __automexia_alias_consent --argument-names requested
        set -g __automexia_alias_consent_reply
        for entry in (string split ',' -- "$__automexia_alias_candidate_overrides")
            set -l parts (string split ':' -- "$entry")
            if test (count $parts) -eq 2; and test "$parts[1]" = "$requested"; and \
                    string match -qr '^[0-9a-f]{64}$' -- "$parts[2]"
                set -g __automexia_alias_consent_reply "$parts[2]"
                return 0
            end
        end
        return 1
    end

    function __automexia_alias_runtime_fingerprint --argument-names requested
        set -l kind (type -t "$requested" 2>/dev/null)
        switch "$kind"
            case file
                set -l path (type -p "$requested" 2>/dev/null)
                test -f "$path"; and not test -L "$path"; or return 1
                __automexia_alias_hash_file "$path"
            case builtin
                printf '%s' "fish|Builtin|$requested|shell-builtin" |
                    __automexia_alias_hash_stream
            case '*'
                return 1
        end
    end

    function __automexia_alias_prepare
        set -g __automexia_alias_reason
        set -g __automexia_alias_collisions
        set -g __automexia_alias_candidate_generation
        set -g __automexia_alias_candidate_path
        set -g __automexia_alias_candidate_names
        set -g __automexia_alias_candidate_overrides

        string match -qr '^/' -- "$__automexia_alias_config_root"; and \
            test (string length -- "$__automexia_alias_config_root") -le 4096
        or begin
            set -g __automexia_alias_state unsafe-path
            set -g __automexia_alias_reason config-root
            return 1
        end
        set -l pointer "$__automexia_alias_root/current"
        test -f "$pointer"; and not test -L "$pointer"
        or begin
            set -g __automexia_alias_state uninitialized
            set -g __automexia_alias_reason current
            return 1
        end
        set -l candidate_generation
        set -l extra_pointer_data
        set -l pointer_read_ok 1
        begin
            read -f -n 81 candidate_generation; or set pointer_read_ok 0
            read -f -n 1 extra_pointer_data; and set pointer_read_ok 0
        end <"$pointer"
        if test $pointer_read_ok -ne 1
            set -g __automexia_alias_state tampered
            set -g __automexia_alias_reason current-lines
            return 1
        end
        set -g __automexia_alias_candidate_generation "$candidate_generation"
        if test "$__automexia_alias_candidate_generation" = disabled
            __automexia_alias_private_root_entries "$pointer"
            or begin
                set -g __automexia_alias_state unsafe-permissions
                set -g __automexia_alias_reason current
                return 1
            end
            set -g __automexia_alias_state disabled
            return 1
        end
        string match -qr '^[0-9a-f]{64}$' -- "$__automexia_alias_candidate_generation"
        or begin
            set -g __automexia_alias_state tampered
            set -g __automexia_alias_reason current-format
            return 1
        end

        set -g __automexia_alias_candidate_directory "$__automexia_alias_root/generations/$__automexia_alias_candidate_generation"
        set -l manifest "$__automexia_alias_candidate_directory/generation.manifest"
        set -g __automexia_alias_candidate_shell_directory "$__automexia_alias_candidate_directory/fish"
        set -g __automexia_alias_candidate_path "$__automexia_alias_candidate_shell_directory/automexia-aliases.fish"
        __automexia_alias_verify_active_entries "$pointer" \
            "$__automexia_alias_candidate_directory" "$manifest" \
            "$__automexia_alias_candidate_shell_directory" \
            "$__automexia_alias_candidate_path"
        or begin
            set -g __automexia_alias_state tampered
            set -g __automexia_alias_reason generation-directory
            return 1
        end
        set -l manifest_lines
        while read -l line
            set -a manifest_lines "$line"
        end <"$manifest"
        if test (count $manifest_lines) -ne 10; or \
                test "$manifest_lines[1]" != automexia-alias-generation-v1; or \
                test "$manifest_lines[2]" != schema=1; or \
                not string match -qr '^source-revision=(0|[1-9][0-9]*)$' -- "$manifest_lines[3]"; or \
                not string match -qr '^source-digest=[0-9a-f]{64}$' -- "$manifest_lines[4]"; or \
                test "$manifest_lines[5]" != generator=automexia-devops/0.4.0; or \
                not string match -q 'shell=powershell|*' -- "$manifest_lines[6]"; or \
                not string match -q 'shell=bash|*' -- "$manifest_lines[7]"; or \
                not string match -q 'shell=zsh|*' -- "$manifest_lines[8]"; or \
                not string match -q 'shell=fish|*' -- "$manifest_lines[9]"; or \
                not string match -q 'shell=cmd|*' -- "$manifest_lines[10]"
            set -g __automexia_alias_state tampered
            set -g __automexia_alias_reason manifest-header
            return 1
        end
        set -l fields (string split '|' -- "$manifest_lines[9]")
        if test (count $fields) -ne 8; or \
                test "$fields[1]" != shell=fish; or \
                test "$fields[2]" != automexia-aliases.fish; or \
                not string match -qr '^[0-9a-f]{64}$' -- "$fields[3]"; or \
                not string match -qr '^[0-9a-f]{64}$' -- "$fields[4]"; or \
                not string match -qr '^[0-9]+$' -- "$fields[5]"; or \
                not string match -qr '^[0-9]+$' -- "$fields[6]"
            set -g __automexia_alias_state tampered
            set -g __automexia_alias_reason manifest-shell
            return 1
        end
        set -g __automexia_alias_candidate_names "$fields[7]"
        set -g __automexia_alias_candidate_overrides "$fields[8]"

        if test "$__automexia_alias_hash_pair_reply[1]" != "$__automexia_alias_candidate_generation"
            set -g __automexia_alias_state tampered
            set -g __automexia_alias_reason manifest-digest
            return 1
        end
        if test "$__automexia_alias_hash_pair_reply[2]" != "$fields[3]"
            set -g __automexia_alias_state tampered
            set -g __automexia_alias_reason artifact-digest
            return 1
        end

        set -l names
        test -z "$__automexia_alias_candidate_names"; or \
            set names (string split ',' -- "$__automexia_alias_candidate_names")
        test (count $names) -eq "$fields[5]"
        or begin
            set -g __automexia_alias_state tampered
            set -g __automexia_alias_reason binding-count
            return 1
        end
        for name in $names
            string match -qr '^[a-z][a-z0-9-]{1,31}$' -- "$name"
            or begin
                set -g __automexia_alias_state tampered
                set -g __automexia_alias_reason alias-name
                return 1
            end
            if type -q "$name"; and not __automexia_alias_owned_public "$name"
                if not __automexia_alias_consent "$name"
                    set -a __automexia_alias_collisions "$name"
                    continue
                end
                set actual (__automexia_alias_runtime_fingerprint "$name")
                if test -z "$actual"; or test "$actual" != "$__automexia_alias_consent_reply"
                    set -a __automexia_alias_collisions "$name"
                end
            end
        end
        if test (count $__automexia_alias_collisions) -ne 0
            set -g __automexia_alias_state collision
            set -g __automexia_alias_reason native-wins
            return 1
        end
        return 0
    end

    function __automexia_alias_activate_candidate
        source "$__automexia_alias_candidate_path"; or return 1
        set -g __automexia_alias_record_kinds
        set -g __automexia_alias_record_names
        set -g __automexia_alias_record_definitions
        set -l names
        test -z "$__automexia_alias_candidate_names"; or \
            set names (string split ',' -- "$__automexia_alias_candidate_names")
        for name in $names
            set -l kind
            if abbr --query "$name"
                set kind A
            else if functions --query "$name"
                set kind F
            else
                return 1
            end
            set -l definition (__automexia_alias_definition_text "$kind" "$name")
            test -n "$definition"; or return 1
            set -a __automexia_alias_record_kinds "$kind"
            set -a __automexia_alias_record_names "$name"
            set -a __automexia_alias_record_definitions "$definition"
        end
        set -g __automexia_alias_generation "$__automexia_alias_candidate_generation"
        set -g __automexia_alias_loaded_path "$__automexia_alias_candidate_path"
        set -g __automexia_alias_state ready
        set -g __automexia_alias_reason
    end

    function automexia_aliases_reload
        set -l old_generation "$__automexia_alias_generation"
        set -l old_path "$__automexia_alias_loaded_path"
        set -l old_kinds $__automexia_alias_record_kinds
        set -l old_names $__automexia_alias_record_names
        set -l old_definitions $__automexia_alias_record_definitions
        if not __automexia_alias_prepare
            test -z "$old_generation"; or \
                set -g __automexia_alias_reason "reload-$__automexia_alias_state-lkg"
            return 1
        end
        if test "$__automexia_alias_candidate_generation" = "$old_generation"
            set -g __automexia_alias_state ready
            return 0
        end
        set -l record_index 1
        for name in $old_names
            set -l actual (__automexia_alias_definition_text \
                "$old_kinds[$record_index]" "$name")
            if test -z "$actual"; or test "$actual" != "$old_definitions[$record_index]"
                set -g __automexia_alias_state reload-conflict
                set -g __automexia_alias_reason definition-changed
                return 1
            end
            set record_index (math $record_index + 1)
        end
        set record_index 1
        for name in $old_names
            switch "$old_kinds[$record_index]"
                case A
                    abbr --erase "$name"
                case F
                    functions --erase "$name"
            end
            set record_index (math $record_index + 1)
        end
        __automexia_alias_activate_candidate; and return 0

        if test -n "$old_path"; and test -f "$old_path"; and not test -L "$old_path"
            source "$old_path"
        end
        set -g __automexia_alias_generation "$old_generation"
        set -g __automexia_alias_loaded_path "$old_path"
        set -g __automexia_alias_record_kinds $old_kinds
        set -g __automexia_alias_record_names $old_names
        set -g __automexia_alias_record_definitions $old_definitions
        set -g __automexia_alias_state reload-failed-lkg
        set -g __automexia_alias_reason activation
        return 1
    end

    function automexia_aliases_health
        set -l generation none
        set -l collisions none
        set -l reason none
        test -z "$__automexia_alias_generation"; or set generation "$__automexia_alias_generation"
        test (count $__automexia_alias_collisions) -eq 0; or \
            set collisions (string join , $__automexia_alias_collisions)
        test -z "$__automexia_alias_reason"; or set reason "$__automexia_alias_reason"
        printf 'state=%s generation=%s collisions=%s reason=%s\n' \
            "$__automexia_alias_state" "$generation" "$collisions" "$reason"
    end

    if __automexia_alias_prepare
        __automexia_alias_activate_candidate
        or begin
            set -g __automexia_alias_state activation-failed
            set -g __automexia_alias_reason source
        end
    end
end
