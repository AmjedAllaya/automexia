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
    set -l o0 (printf '%03o' (math --scale=0 "$value % 256"))
    set -l o1 (printf '%03o' (math --scale=0 "floor($value / 256) % 256"))
    set -l o2 (printf '%03o' (math --scale=0 "floor($value / 65536) % 256"))
    set -l o3 (printf '%03o' (math --scale=0 "floor($value / 16777216) % 256"))
    test "$fd" = 3; or return 1
    printf '%b' "\\$o0\\$o1\\$o2\\$o3" >&3
end

function __automexia_suggestion_u64 --argument-names value fd
    __automexia_suggestion_u32 (math --scale=0 "$value % 4294967296") $fd
    __automexia_suggestion_u32 (math --scale=0 "floor($value / 4294967296)") $fd
end

function __automexia_suggestion_utf8_length --argument-names value
    set -l escaped (string escape --style=url -- "$value")
    set -l percent_count (string match -ar '%' -- "$escaped" | count)
    set -l escaped_length (string length -- "$escaped")
    math "$escaped_length - 2 * $percent_count"
end

function __automexia_suggestion_decode_hex --argument-names hex
    string match -qr '^[0-9A-F]{2,2048}$' -- "$hex"; or return 1
    set -l hex_length (string length -- "$hex")
    test (math "$hex_length % 2") -eq 0; or return 1
    set -l pairs (string match -ar '..' -- "$hex")
    set -l bytes
    for pair in $pairs
        set -a bytes (math "0x$pair")
    end
    set -l escaped
    set -l index 1
    while test $index -le (count $bytes)
        set -l b0 $bytes[$index]
        set -l b1 0
        set -l b2 0
        set -l b3 0
        set -l width 0
        if test $b0 -ge 32 -a $b0 -le 126
            set width 1
        else if test $b0 -ge 194 -a $b0 -le 223 -a (math "$index + 1") -le (count $bytes)
            set b1 $bytes[(math "$index + 1")]
            test $b1 -ge 128 -a $b1 -le 191; or return 1
            test $b0 -ne 194 -o $b1 -gt 159; or return 1
            test $b0 -ne 216 -o $b1 -ne 156; or return 1
            set width 2
        else if test $b0 -ge 224 -a $b0 -le 239 -a (math "$index + 2") -le (count $bytes)
            set b1 $bytes[(math "$index + 1")]
            set b2 $bytes[(math "$index + 2")]
            test $b2 -ge 128 -a $b2 -le 191; or return 1
            if test $b0 -eq 224
                test $b1 -ge 160 -a $b1 -le 191; or return 1
            else if test $b0 -eq 237
                test $b1 -ge 128 -a $b1 -le 159; or return 1
            else
                test $b1 -ge 128 -a $b1 -le 191; or return 1
            end
            if test $b0 -eq 226 -a $b1 -eq 128
                test $b2 -ne 142 -a $b2 -ne 143; or return 1
                test $b2 -lt 170 -o $b2 -gt 174; or return 1
            else if test $b0 -eq 226 -a $b1 -eq 129
                test $b2 -lt 166 -o $b2 -gt 169; or return 1
            end
            set width 3
        else if test $b0 -ge 240 -a $b0 -le 244 -a (math "$index + 3") -le (count $bytes)
            set b1 $bytes[(math "$index + 1")]
            set b2 $bytes[(math "$index + 2")]
            set b3 $bytes[(math "$index + 3")]
            test $b2 -ge 128 -a $b2 -le 191 -a $b3 -ge 128 -a $b3 -le 191; or return 1
            if test $b0 -eq 240
                test $b1 -ge 144 -a $b1 -le 191; or return 1
            else if test $b0 -eq 244
                test $b1 -ge 128 -a $b1 -le 143; or return 1
            else
                test $b1 -ge 128 -a $b1 -le 191; or return 1
            end
            set width 4
        else
            return 1
        end
        for offset in (seq 0 (math "$width - 1"))
            set -l pair_index (math "$index + $offset")
            set escaped "$escaped\\x$pairs[$pair_index]"
        end
        set index (math "$index + $width")
    end
    set -g __automexia_suggestion_decoded (printf '%b' "$escaped")
    test -n "$__automexia_suggestion_decoded"
end

function __automexia_suggestion_read_response \
        --argument-names original_line original_point generation span_start span_end native_start native_end
    set -l response
    set -l fish_read_limit 2176
    read --local --nchars 2176 response </dev/fd/4; or return 1
    test (__automexia_suggestion_utf8_length "$response") -le 2175; or return 1
    set -l fields (string split \t -- "$response")
    if test (count $fields) -eq 3 -a "$fields[1]" = AXSR1 -a "$fields[2]" = S; and string match -qr '^[1-6]$' -- "$fields[3]"
        set -g __automexia_suggestion_reason no-replacement
        return 0
    end
    test (count $fields) -eq 7; or return 1
    test "$fields[1]" = AXSR1 -a "$fields[2]" = R \
        -a "$fields[3]" = "$generation" -a "$fields[5]" = "$span_start" \
        -a "$fields[6]" = "$span_end"; or return 1
    string match -qr '^[1-9][0-9]*$' -- "$fields[4]"; or return 1
    __automexia_suggestion_decode_hex "$fields[7]"; or return 1
    test (commandline) = "$original_line" -a (commandline --cursor) -eq $original_point
    or begin
        set -g __automexia_suggestion_reason stale-editor-state
        return 0
    end
    set -l prefix (string sub --start 1 --length $native_start -- "$original_line")
    set -l suffix (string sub --start (math "$native_end + 1") -- "$original_line")
    set -l next "$prefix$__automexia_suggestion_decoded$suffix"
    commandline --replace "$next"
    set -l insertion_length (string length -- "$__automexia_suggestion_decoded")
    commandline --cursor (math "$native_start + $insertion_length")
    set -g __automexia_suggestion_reason accepted
end

function __automexia_suggestions_request
    test "$__automexia_suggestion_state" = ready; or return 0
    set -l fd $AUTOMEXIA_SUGGESTION_REQUEST_FD
    set -l line (commandline)
    set -l point (commandline --cursor)
    set -l token (commandline --current-token)
    set -l token_cursor (commandline --current-token --cursor)
    set -l native_start (math "$point - $token_cursor")
    set -l token_length (string length -- "$token")
    set -l native_end (math "$native_start + $token_length")
    if string match -qr "['\"\\\\]" -- "$token"
        set native_start $point
        set native_end $point
    end
    set -l prefix (string sub --start 1 --length $point -- "$line")
    set -l span_prefix (string sub --start 1 --length $native_start -- "$line")
    set -l byte_length (__automexia_suggestion_utf8_length "$line")
    set -l point_byte (__automexia_suggestion_utf8_length "$prefix")
    set -l span_start (__automexia_suggestion_utf8_length "$span_prefix")
    set -l span_end (__automexia_suggestion_utf8_length \
        (string sub --start 1 --length $native_end -- "$line"))
    test $byte_length -le 16384
    or begin
        set -g __automexia_suggestion_reason buffer-limit
        return 0
    end

    set -l candidates
    set -l candidate_lengths
    set -l payload_length (math "30 + $byte_length")
    set -l fish_read_limit 4096
    complete -C "$line" | while read --local --line completion
        set -l parts (string split -m1 \t -- "$completion")
        set -l candidate $parts[1]
        test -n "$candidate"; or continue
        string match -qr '[\x00-\x1f\x7f\x{061C}\x{200E}\x{200F}\x{202A}-\x{202E}\x{2066}-\x{2069}]' -- "$candidate"; and continue
        set -l candidate_length (__automexia_suggestion_utf8_length "$candidate")
        test $candidate_length -le 1024; or continue
        set -a candidates "$candidate"
        set -a candidate_lengths $candidate_length
        set payload_length (math "$payload_length + 4 + $candidate_length")
        test (count $candidates) -ge 512; and break
    end
    test $payload_length -le 524288; or begin
        set candidates
        set candidate_lengths
        set payload_length (math "30 + $byte_length")
    end

    if test $__automexia_suggestion_generation -ge 9007199254740991
        automexia_suggestions_disable
        set -g __automexia_suggestion_reason identity-exhausted
        return 0
    end
    set -g __automexia_suggestion_generation \
        (math --scale=0 "$__automexia_suggestion_generation + 1")
    printf 'AXSH\001\000\001' >&3
    or begin
        automexia_suggestions_disable
        set -g __automexia_suggestion_reason helper-disconnected
        return 0
    end
    __automexia_suggestion_u32 $payload_length $fd
    __automexia_suggestion_u32 $byte_length $fd
    printf '%s' "$line" >&3
    __automexia_suggestion_u32 $point_byte $fd
    __automexia_suggestion_u64 $__automexia_suggestion_generation $fd
    __automexia_suggestion_u32 $span_start $fd
    __automexia_suggestion_u32 $span_end $fd
    printf '\000\005' >&3
    __automexia_suggestion_u32 (count $candidates) $fd
    for index in (seq 1 (count $candidates))
        __automexia_suggestion_u32 $candidate_lengths[$index] $fd
        printf '%s' "$candidates[$index]" >&3
    end
    __automexia_suggestion_read_response "$line" "$point" \
        "$__automexia_suggestion_generation" "$span_start" "$span_end" \
        "$native_start" "$native_end"
    or begin
        automexia_suggestions_disable
        set -g __automexia_suggestion_reason helper-disconnected
    end
end

function automexia_suggestions_enable --argument-names chord
    automexia_suggestions_disable
    if not string match -qr '^([3-9]|[1-9][0-9]+)\.' -- "$version"
        set -g __automexia_suggestion_reason unsupported-fish
        return 1
    end
    if test "$AUTOMEXIA_SUGGESTION_PREVIEW" != 1; or \
            test "$AUTOMEXIA_SUGGESTION_REQUEST_FD" != 3; or \
            test "$AUTOMEXIA_SUGGESTION_RESPONSE_FD" != 4
        set -g __automexia_suggestion_reason preview-disabled
        return 1
    end
    if not test -w /dev/fd/3; or not test -r /dev/fd/4
        set -g __automexia_suggestion_reason helper-unavailable
        return 1
    end
    set -g __automexia_suggestion_state ready-unbound
    set -g __automexia_suggestion_reason none
    test -n "$chord"; or return 0
    # Ctrl+Space is a native Fish insertion binding and always collides.
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
