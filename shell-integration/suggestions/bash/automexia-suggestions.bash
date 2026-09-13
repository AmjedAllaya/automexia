# Automexia CP5.5 preview adapter. Session-only; never source from a profile.

__automexia_suggestion_state=disabled
__automexia_suggestion_reason=preview-disabled
__automexia_suggestion_chord=
__automexia_suggestion_generation=0

automexia_suggestions_health() {
  printf 'state=%s reason=%s bound=%s fallback=readline-native\n' \
    "$__automexia_suggestion_state" "$__automexia_suggestion_reason" \
    "$([[ -n $__automexia_suggestion_chord ]] && printf yes || printf no)"
}

__automexia_suggestion_u32() {
  local value=$1 fd=$2 o0 o1 o2 o3
  printf -v o0 '%03o' $((value & 255))
  printf -v o1 '%03o' $(((value >> 8) & 255))
  printf -v o2 '%03o' $(((value >> 16) & 255))
  printf -v o3 '%03o' $(((value >> 24) & 255))
  printf '%b' "\\$o0\\$o1\\$o2\\$o3" >&"$fd"
}

__automexia_suggestion_u64() {
  __automexia_suggestion_u32 "$(( $1 & 4294967295 ))" "$2"
  __automexia_suggestion_u32 "$(( ($1 >> 32) & 4294967295 ))" "$2"
}

__automexia_suggestion_decode_hex() {
  local hex=$1 pair output="" byte chunk
  local b0 b1 b2 b3 width offset
  [[ -n $hex && ${#hex} -le 2048 && $((${#hex} % 2)) -eq 0 && $hex != *[!0-9A-F]* ]] || return 1
  while [[ -n $hex ]]; do
    b0=$((16#${hex:0:2}))
    b1=0 b2=0 b3=0 width=0
    if ((b0 >= 32 && b0 <= 126)); then
      width=1
    elif ((b0 >= 194 && b0 <= 223 && ${#hex} >= 4)); then
      b1=$((16#${hex:2:2}))
      ((b1 >= 128 && b1 <= 191)) || return 1
      ((!(b0 == 194 && b1 <= 159) && !(b0 == 216 && b1 == 156))) || return 1
      width=2
    elif ((b0 >= 224 && b0 <= 239 && ${#hex} >= 6)); then
      b1=$((16#${hex:2:2}))
      b2=$((16#${hex:4:2}))
      ((b2 >= 128 && b2 <= 191)) || return 1
      if ((b0 == 224)); then
        ((b1 >= 160 && b1 <= 191)) || return 1
      elif ((b0 == 237)); then
        ((b1 >= 128 && b1 <= 159)) || return 1
      else
        ((b1 >= 128 && b1 <= 191)) || return 1
      fi
      if ((b0 == 226 && b1 == 128 &&
           (b2 == 142 || b2 == 143 || (b2 >= 170 && b2 <= 174)))); then
        return 1
      fi
      if ((b0 == 226 && b1 == 129 && b2 >= 166 && b2 <= 169)); then
        return 1
      fi
      width=3
    elif ((b0 >= 240 && b0 <= 244 && ${#hex} >= 8)); then
      b1=$((16#${hex:2:2}))
      b2=$((16#${hex:4:2}))
      b3=$((16#${hex:6:2}))
      ((b2 >= 128 && b2 <= 191 && b3 >= 128 && b3 <= 191)) || return 1
      if ((b0 == 240)); then
        ((b1 >= 144 && b1 <= 191)) || return 1
      elif ((b0 == 244)); then
        ((b1 >= 128 && b1 <= 143)) || return 1
      else
        ((b1 >= 128 && b1 <= 191)) || return 1
      fi
      width=4
    else
      return 1
    fi
    chunk=${hex:0:width*2}
    for ((offset = 0; offset < width * 2; offset += 2)); do
      pair=${chunk:offset:2}
      printf -v byte '%b' "\\x$pair"
      output+=$byte
    done
    hex=${hex:width*2}
  done
  __automexia_suggestion_decoded=$output
}

__automexia_suggestion_read_response() {
  local original_line=$1 original_point=$2 generation=$3 span_start=$4 span_end=$5
  local native_start=$6 native_end=$7 fd=$AUTOMEXIA_SUGGESTION_RESPONSE_FD response
  if ! LC_ALL=C IFS= read -r -t 30 -n 2176 -u "$fd" response; then
    return 1
  fi
  local old_lc=${LC_ALL-} response_bytes
  LC_ALL=C
  response_bytes=${#response}
  LC_ALL=$old_lc
  ((response_bytes <= 2175)) || return 1
  local -a fields
  IFS=$'	' read -r -a fields <<< "$response"
  if ((${#fields[@]} == 3)) && [[ ${fields[0]} == AXSR1 && ${fields[1]} == S &&
      ${fields[2]} =~ ^[1-6]$ ]]; then
    __automexia_suggestion_reason=no-replacement
    return 0
  fi
  ((${#fields[@]} == 7)) || return 1
  [[ ${fields[0]} == AXSR1 && ${fields[1]} == R && ${fields[2]} == "$generation" &&
     ${fields[3]} =~ ^[1-9][0-9]*$ && ${fields[4]} == "$span_start" &&
     ${fields[5]} == "$span_end" ]] || return 1
  __automexia_suggestion_decode_hex "${fields[6]}" || return 1
  [[ $READLINE_LINE == "$original_line" && $READLINE_POINT -eq original_point ]] || {
    __automexia_suggestion_reason=stale-editor-state
    return 0
  }
  local insertion=$__automexia_suggestion_decoded prefix suffix insertion_bytes
  LC_ALL=C
  prefix=${READLINE_LINE:0:span_start}
  suffix=${READLINE_LINE:span_end}
  insertion_bytes=${#insertion}
  READLINE_LINE=$prefix$insertion$suffix
  READLINE_POINT=$((span_start + insertion_bytes))
  LC_ALL=$old_lc
  __automexia_suggestion_reason=accepted
}
__automexia_suggestions_request() {
  [[ $__automexia_suggestion_state == ready ]] || return 0
  local fd=$AUTOMEXIA_SUGGESTION_REQUEST_FD line=$READLINE_LINE point=$READLINE_POINT
  local byte_length point_byte prefix word word_bytes span_start span_end native_start native_end
  local old_lc=${LC_ALL-}
  LC_ALL=C
  byte_length=${#line}
  prefix=${line:0:point}
  point_byte=${#prefix}
  LC_ALL=$old_lc
  word=${prefix##*[[:space:]]}
  LC_ALL=C
  word_bytes=${#word}
  LC_ALL=$old_lc
  span_start=$((point_byte - word_bytes))
  span_end=$point_byte
  if [[ $word == *"'"* || $word == *'"'* || $word == *\\* ]]; then
    span_start=$point_byte
  fi
  native_start=$span_start
  native_end=$span_end
  if ((byte_length > 16384)); then
    __automexia_suggestion_reason=buffer-limit
    return 0
  fi
  ((__automexia_suggestion_generation += 1))
  printf 'AXSH\001\000\001' >&"$fd" || {
    automexia_suggestions_disable
    __automexia_suggestion_reason=helper-disconnected
    return 0
  }
  __automexia_suggestion_u32 "$((30 + byte_length))" "$fd"
  __automexia_suggestion_u32 "$byte_length" "$fd"
  printf '%s' "$line" >&"$fd"
  __automexia_suggestion_u32 "$point_byte" "$fd"
  __automexia_suggestion_u64 "$__automexia_suggestion_generation" "$fd"
  __automexia_suggestion_u32 "$span_start" "$fd"
  __automexia_suggestion_u32 "$span_end" "$fd"
  printf '\000\005' >&"$fd" # no selection; shell-specific quoting
  __automexia_suggestion_u32 0 "$fd"
  __automexia_suggestion_read_response "$line" "$point" \
    "$__automexia_suggestion_generation" "$span_start" "$span_end" \
    "$native_start" "$native_end" || {
      automexia_suggestions_disable
      __automexia_suggestion_reason=helper-disconnected
    }
}

automexia_suggestions_enable() {
  local chord=${1-} existing
  automexia_suggestions_disable
  if ((BASH_VERSINFO[0] < 5)); then
    __automexia_suggestion_reason=unsupported-bash
    return 1
  fi
  if [[ ${AUTOMEXIA_SUGGESTION_PREVIEW-0} != 1 ||
        ! ${AUTOMEXIA_SUGGESTION_REQUEST_FD-} =~ ^[0-9]+$ ||
        ! ${AUTOMEXIA_SUGGESTION_RESPONSE_FD-} =~ ^[0-9]+$ ]]; then
    __automexia_suggestion_reason=preview-disabled
    return 1
  fi
  if ! { : >&"$AUTOMEXIA_SUGGESTION_REQUEST_FD"; } 2>/dev/null ||
     ! { : <&"$AUTOMEXIA_SUGGESTION_RESPONSE_FD"; } 2>/dev/null; then
    __automexia_suggestion_reason=helper-unavailable
    return 1
  fi
  __automexia_suggestion_state=ready-unbound
  __automexia_suggestion_reason=none
  [[ -n $chord ]] || return 0
  while IFS= read -r existing; do
    if [[ $existing == *"\"$chord\""* ]]; then
      __automexia_suggestion_reason="binding-collision"
      return 1
    fi
  done < <(bind -P; bind -X; bind -S)
  bind -x "\"$chord\":__automexia_suggestions_request" || {
    __automexia_suggestion_reason="binding-failed"
    return 1
  }
  __automexia_suggestion_chord=$chord
  __automexia_suggestion_state=ready
}

automexia_suggestions_disable() {
  local binding
  if [[ -n $__automexia_suggestion_chord ]]; then
    while IFS= read -r binding; do
      if [[ $binding == *"\"$__automexia_suggestion_chord\""* &&
            $binding == *__automexia_suggestions_request* ]]; then
        bind -r "$__automexia_suggestion_chord"
      fi
    done < <(bind -X)
  fi
  __automexia_suggestion_chord=
  __automexia_suggestion_state=disabled
  [[ $__automexia_suggestion_reason != none ]] || __automexia_suggestion_reason=disabled
}
