# Automexia CP1 Fish completion adapter. `complete -c` inventories names only;
# it does not execute completion bodies. Existing Fish definitions win.
set -q AUTOMEXIA_COMPLETION_ADAPTER_FISH_LOADED; and return 0
set -g AUTOMEXIA_COMPLETION_ADAPTER_FISH_LOADED 1
set -g __automexia_completion_root ''
if set -q AUTOMEXIA_CONFIG_HOME; and test -n "$AUTOMEXIA_CONFIG_HOME"
    set __automexia_completion_root "$AUTOMEXIA_CONFIG_HOME"
else if test (command uname -s) = Darwin
    set __automexia_completion_root "$HOME/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal"
else if set -q XDG_CONFIG_HOME; and test -n "$XDG_CONFIG_HOME"
    set __automexia_completion_root "$XDG_CONFIG_HOME/automexia"
else
    set __automexia_completion_root "$HOME/.config/automexia"
end
set -g __automexia_completion_config_root "$__automexia_completion_root"
set __automexia_completion_root "$__automexia_completion_root/generated/completion"
set -g __automexia_completion_collisions
set -g __automexia_completion_loaded

function __automexia_completion_directory_safe
    string match -qr '^/' -- "$__automexia_completion_root"; or return 1
    test (string length -- "$__automexia_completion_config_root") -le 4096; or return 1
    set -l generated_dir (path dirname "$__automexia_completion_root")
    set -l config_dir (path dirname "$generated_dir")
    set -l shell_dir "$__automexia_completion_root/fish"
    for directory in "$config_dir" "$generated_dir" "$__automexia_completion_root" "$shell_dir"
        test -L "$directory"; and return 1
        if test -e "$directory"; and not test -d "$directory"
            return 1
        end
    end
    return 0
end

set -g __automexia_completion_path_safe 0
__automexia_completion_directory_safe; and set __automexia_completion_path_safe 1

function __automexia_completion_hash --argument-names file
    if type -q sha256sum
        command sha256sum -- "$file" | read -l digest remainder
        printf '%s\n' "$digest"
    else if type -q shasum
        command shasum -a 256 -- "$file" | read -l digest remainder
        printf '%s\n' "$digest"
    else if type -q openssl
        command openssl dgst -sha256 "$file" | read -l label digest
        printf '%s\n' "$digest"
    else
        return 127
    end
end

function __automexia_load_completion --argument-names target
    set -l file "$__automexia_completion_root/fish/$target.fish"
    test -f "$file"; and not test -L "$file"; or return 0
    test -f "$file.sha256"; and not test -L "$file.sha256"; or return 0
    test (command wc -c <"$file") -le 1114112; or return 0
    test (command wc -c <"$file.sha256") -le 192; or return 0
    set -l existing (complete -c "$target")
    if test (count $existing) -gt 0
        set -ga __automexia_completion_collisions $target
        return 0
    end
    set -l expected_digests
    while read -l expected
        string match -qr '^[0-9a-f]{64}$' -- "$expected"; or return 0
        set -a expected_digests "$expected"
        test (count $expected_digests) -le 2; or return 0
    end <"$file.sha256"
    test (count $expected_digests) -ge 1; or return 0
    set -l actual (__automexia_completion_hash "$file"); or return 0
    contains -- "$actual" $expected_digests; or return 0
    source "$file"
    set -ga __automexia_completion_loaded $target
end

function automexia_completion_health
    set -l state enabled
    if test $__automexia_completion_path_safe -ne 1
        set state unsafe-path/native-fallback
    else if set -q AUTOMEXIA_COMPLETION_DISABLED; and test "$AUTOMEXIA_COMPLETION_DISABLED" = 1
        set state disabled
    else if test -f "$__automexia_completion_root/.disabled"
        set state disabled
    end
    set -l loaded (string join , $__automexia_completion_loaded)
    set -l collisions (string join , $__automexia_completion_collisions)
    test -n "$loaded"; or set loaded none
    test -n "$collisions"; or set collisions none
    printf 'shell=fish\nversion=%s\nstate=%s\nloaded=%s\ncollisions=%s\n' \
        "$version" "$state" "$loaded" "$collisions"
end

set -l completion_disabled 0
if set -q AUTOMEXIA_COMPLETION_DISABLED; and test "$AUTOMEXIA_COMPLETION_DISABLED" = 1
    set completion_disabled 1
end
if test $__automexia_completion_path_safe -eq 1; and test $completion_disabled -eq 0; and not test -f "$__automexia_completion_root/.disabled"
    for target in docker kubectl oc helm
        __automexia_load_completion $target
    end
end
