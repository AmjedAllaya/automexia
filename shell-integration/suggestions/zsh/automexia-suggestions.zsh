# Automexia CP5.5 preview adapter. Session-only; never source from a profile.

typeset -g __automexia_suggestion_state=disabled
typeset -g __automexia_suggestion_reason=preview-disabled
typeset -g __automexia_suggestion_chord=
typeset -gi __automexia_suggestion_generation=0

automexia_suggestions_health() {
  local bound=no
  [[ -n $__automexia_suggestion_chord ]] && bound=yes
  print -r -- "state=$__automexia_suggestion_state reason=$__automexia_suggestion_reason bound=$bound fallback=zle-native"
}

__automexia_suggestion_u32() {
  local -i value=$1 fd=$2
  local o0 o1 o2 o3
  printf -v o0 '%03o' $((value & 255))
  printf -v o1 '%03o' $(((value >> 8) & 255))
  printf -v o2 '%03o' $(((value >> 16) & 255))
  printf -v o3 '%03o' $(((value >> 24) & 255))
  printf '%b' "\\$o0\\$o1\\$o2\\$o3" >&$fd
}

__automexia_suggestion_u64() {
  __automexia_suggestion_u32 "$(( $1 & 4294967295 ))" "$2"
  __automexia_suggestion_u32 "$(( ($1 >> 32) & 4294967295 ))" "$2"
}

__automexia_suggestion_utf8_length() {
  emulate -L zsh
  setopt nomultibyte
  local value=$1
  print -r -- ${#value}
}
__automexia_suggestion_decode_hex() {
  local hex=$1 pair byte output= chunk
  local -i b0 b1 b2 b3 width offset
  [[ -n $hex && ${#hex} -le 2048 && $((${#hex} % 2)) -eq 0 && $hex != *[^0-9A-F]* ]] || return 1
  while [[ -n $hex ]]; do
    pair=${hex[1,2]}
    b0=$((16#$pair))
    b1=0 b2=0 b3=0 width=0
    if ((b0 >= 32 && b0 <= 126)); then
      width=1
    elif ((b0 >= 194 && b0 <= 223 && ${#hex} >= 4)); then
      pair=${hex[3,4]}; b1=$((16#$pair))
      ((b1 >= 128 && b1 <= 191)) || return 1
      ((!(b0 == 194 && b1 <= 159) && !(b0 == 216 && b1 == 156))) || return 1
      width=2
    elif ((b0 >= 224 && b0 <= 239 && ${#hex} >= 6)); then
      pair=${hex[3,4]}; b1=$((16#$pair))
      pair=${hex[5,6]}; b2=$((16#$pair))
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
      pair=${hex[3,4]}; b1=$((16#$pair))
      pair=${hex[5,6]}; b2=$((16#$pair))
      pair=${hex[7,8]}; b3=$((16#$pair))
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
    chunk=${hex[1,width*2]}
    for ((offset = 1; offset <= width * 2; offset += 2)); do
      pair=${chunk[offset,offset+1]}
      printf -v byte '%b' "\\x$pair"
      output+=$byte
    done
    hex=${hex[width*2+1,-1]}
  done
  __automexia_suggestion_decoded=$output
}

__automexia_suggestion_read_line() {
  emulate -L zsh
  setopt localoptions nomultibyte
  local -i fd=$1 count=0
  local response
  if ! sysread -i $fd -s 2176 -t 30 -c count response; then
    return 1
  fi
  ((count > 0 && count <= 2176)) || return 1
  [[ $response == *$'\n' ]] || return 1
  response=${response%$'\n'}
  [[ $response != *$'\n'* && ${#response} -le 2175 ]] || return 1
  REPLY=$response
}
__automexia_suggestion_read_response() {
  local original_line=$1
  local -i original_cursor=$2 generation=$3 span_start=$4 span_end=$5
  local -i native_start=$6 native_end=$7 fd=$AUTOMEXIA_SUGGESTION_RESPONSE_FD
  local response
  __automexia_suggestion_read_line $fd || return 1
  response=$REPLY
  local -a fields
  fields=("${(@ps:\t:)response}")
  if (( ${#fields} == 3 )) && [[ $fields[1] == AXSR1 && $fields[2] == S &&
      $fields[3] == [1-6] ]]; then
    __automexia_suggestion_reason=no-replacement
    return 0
  fi
  (( ${#fields} == 7 )) || return 1
  [[ $fields[1] == AXSR1 && $fields[2] == R && $fields[3] == $generation &&
     $fields[4] == <1-> && $fields[5] == $span_start && $fields[6] == $span_end ]] || return 1
  __automexia_suggestion_decode_hex "$fields[7]" || return 1
  [[ $BUFFER == "$original_line" && $CURSOR -eq original_cursor ]] || {
    __automexia_suggestion_reason=stale-editor-state
    return 0
  }
  local prefix=${BUFFER[1,native_start]}
  local suffix=${BUFFER[native_end+1,-1]}
  BUFFER=$prefix$__automexia_suggestion_decoded$suffix
  CURSOR=$((native_start + ${#__automexia_suggestion_decoded}))
  __automexia_suggestion_reason=accepted
  zle redisplay
}

__automexia_suggestions_request() {
  [[ $__automexia_suggestion_state == ready ]] || return 0
  local -i fd=$AUTOMEXIA_SUGGESTION_REQUEST_FD point=$CURSOR byte_length point_byte
  local line=$BUFFER prefix=${BUFFER[1,CURSOR]}
  local word=${prefix##*[[:space:]]}
  local -i native_start=$((CURSOR - ${#word})) native_end=$CURSOR
  [[ $word == *[\'\"\\]* ]] && native_start=$CURSOR
  local span_prefix=${line[1,native_start]}
  byte_length=$(__automexia_suggestion_utf8_length "$line")
  point_byte=$(__automexia_suggestion_utf8_length "$prefix")
  local -i span_start=$(__automexia_suggestion_utf8_length "$span_prefix")
  local -i span_end=$point_byte
  if ((byte_length > 16384)); then
    __automexia_suggestion_reason=buffer-limit
    return 0
  fi
  ((__automexia_suggestion_generation += 1))
  printf 'AXSH\001\000\001' >&$fd || {
    automexia_suggestions_disable
    __automexia_suggestion_reason=helper-disconnected
    return 0
  }
  __automexia_suggestion_u32 "$((30 + byte_length))" "$fd"
  __automexia_suggestion_u32 "$byte_length" "$fd"
  printf '%s' "$line" >&$fd
  __automexia_suggestion_u32 "$point_byte" "$fd"
  __automexia_suggestion_u64 "$__automexia_suggestion_generation" "$fd"
  __automexia_suggestion_u32 "$span_start" "$fd"
  __automexia_suggestion_u32 "$span_end" "$fd"
  printf '\000\005' >&$fd
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
  autoload -Uz is-at-least
  if ! is-at-least 5.8 || ! zmodload zsh/system; then
    __automexia_suggestion_reason=unsupported-zsh
    return 1
  fi
  if [[ ${AUTOMEXIA_SUGGESTION_PREVIEW-0} != 1 ||
        ${AUTOMEXIA_SUGGESTION_REQUEST_FD-} != <-> ||
        ${AUTOMEXIA_SUGGESTION_RESPONSE_FD-} != <-> ]]; then
    __automexia_suggestion_reason=preview-disabled
    return 1
  fi
  if ! : >&$AUTOMEXIA_SUGGESTION_REQUEST_FD 2>/dev/null ||
     ! : <&$AUTOMEXIA_SUGGESTION_RESPONSE_FD 2>/dev/null; then
    __automexia_suggestion_reason=helper-unavailable
    return 1
  fi
  zle -N automexia-suggestions-request __automexia_suggestions_request
  __automexia_suggestion_state=ready-unbound
  __automexia_suggestion_reason=none
  [[ -n $chord ]] || return 0
  existing=$(bindkey "$chord" 2>/dev/null)
  if [[ -n $existing && $existing != *undefined-key* ]]; then
    __automexia_suggestion_reason=binding-collision
    return 1
  fi
  bindkey "$chord" automexia-suggestions-request || {
    __automexia_suggestion_reason=binding-failed
    return 1
  }
  __automexia_suggestion_chord=$chord
  __automexia_suggestion_state=ready
}

automexia_suggestions_disable() {
  local existing
  if [[ -n $__automexia_suggestion_chord ]]; then
    existing=$(bindkey "$__automexia_suggestion_chord" 2>/dev/null)
    [[ $existing == *automexia-suggestions-request* ]] && bindkey -r "$__automexia_suggestion_chord"
  fi
  zle -D automexia-suggestions-request 2>/dev/null || true
  __automexia_suggestion_chord=
  __automexia_suggestion_state=disabled
  [[ $__automexia_suggestion_reason != none ]] || __automexia_suggestion_reason=disabled
}
