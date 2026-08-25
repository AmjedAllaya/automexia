# Automexia CP5.5 preview adapter. Session-only; never source from a profile.

set -g __automexia_suggestion_state disabled
set -g __automexia_suggestion_reason preview-disabled
set -g __automexia_suggestion_chord
set -g __automexia_suggestion_generation 0

function automexia_suggestions_health
    set -l bound no
    test -n "$__automexia_suggestion_chord"; and set bound yes
    printf 'state=%s reason=%s bound=%s fallback=fish-native\n' \
        "$__automexia_suggestion_state" "$__automexia_suggestion_reason" "$bound"
end

function __automexia_suggestion_u32 --argument-names value fd
    set -l o0 (printf '%03o' (math "bitand($value, 255)"))
    set -l o1 (printf '%03o' (math "bitand(bitshr($value, 8), 255)"))
    set -l o2 (printf '%03o' (math "bitand(bitshr($value, 16), 255)"))
    set -l o3 (printf '%03o' (math "bitand(bitshr($value, 24), 255)"))
    printf '%b' "\\$o0\\$o1\\$o2\\$o3" >&$fd
end

function __automexia_suggestions_request
    test "$__automexia_suggestion_state" = ready; or return 0
    set -l fd $AUTOMEXIA_SUGGESTION_REQUEST_FD
    set -l line (commandline)
    set -l point (commandline --cursor)
    # Fish does not expose a mutation-free byte-offset conversion primitive.
    # Keep the preview fail-closed for non-ASCII input; Fish's native Unicode
    # completion/autosuggestion remains fully available.
    string match -qr '^[\x20-\x7e]*$' -- "$line"
    or begin
        set -g __automexia_suggestion_reason unicode-native-fallback
        return 0
    end
    set -l byte_length (string length -- "$line")
    test $byte_length -le 16384
    or begin
        set -g __automexia_suggestion_reason buffer-limit
        return 0
    end
    set -g __automexia_suggestion_generation \
        (math "$__automexia_suggestion_generation + 1")
    printf 'AXSH\001\000\001' >&$fd
    or begin
        automexia_suggestions_disable
        set -g __automexia_suggestion_reason helper-disconnected
        return 0
    end
    __automexia_suggestion_u32 (math "4 + $byte_length + 4 + 4 + 4") $fd
    __automexia_suggestion_u32 $byte_length $fd
    printf '%s' "$line" >&$fd
    __automexia_suggestion_u32 $point $fd
    __automexia_suggestion_u32 $__automexia_suggestion_generation $fd
    __automexia_suggestion_u32 0 $fd
    commandline -f repaint
end

function automexia_suggestions_enable --argument-names chord
    automexia_suggestions_disable
    if not string match -qr '^([3-9]|[1-9][0-9]+)\.' -- "$version"
        set -g __automexia_suggestion_reason unsupported-fish
        return 1
    end
    if test "$AUTOMEXIA_SUGGESTION_PREVIEW" != 1; or \
            not string match -qr '^[0-9]+$' -- "$AUTOMEXIA_SUGGESTION_REQUEST_FD"
        set -g __automexia_suggestion_reason preview-disabled
        return 1
    end
    if not true >&$AUTOMEXIA_SUGGESTION_REQUEST_FD 2>/dev/null
        set -g __automexia_suggestion_reason helper-unavailable
        return 1
    end
    set -g __automexia_suggestion_state ready-unbound
    set -g __automexia_suggestion_reason none
    test -n "$chord"; or return 0
    # Ctrl+Space and every existing user/preset binding are collisions.
    if test "$chord" = '\c@'; or bind --user "$chord" >/dev/null 2>&1; or \
            bind --preset "$chord" >/dev/null 2>&1
        set -g __automexia_suggestion_reason binding-collision
        return 1
    end
    bind --user "$chord" __automexia_suggestions_request
    or begin
        set -g __automexia_suggestion_reason binding-failed
        return 1
    end
    set -g __automexia_suggestion_chord "$chord"
    set -g __automexia_suggestion_state ready
end

function automexia_suggestions_disable
    if test -n "$__automexia_suggestion_chord"
        set -l binding (bind --user "$__automexia_suggestion_chord" 2>/dev/null)
        string match -q '*__automexia_suggestions_request*' -- "$binding"; and \
            bind --user --erase "$__automexia_suggestion_chord"
    end
    set -g __automexia_suggestion_chord
    set -g __automexia_suggestion_state disabled
    test "$__automexia_suggestion_reason" != none; or \
        set -g __automexia_suggestion_reason disabled
end
