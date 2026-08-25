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

__automexia_suggestions_request() {
  [[ $__automexia_suggestion_state == ready ]] || return 0
  local -i fd=$AUTOMEXIA_SUGGESTION_REQUEST_FD point=$CURSOR byte_length point_byte
  local line=$BUFFER prefix=${BUFFER[1,CURSOR]}
  local old_lc=${LC_ALL-}
  LC_ALL=C
  byte_length=${#line}
  point_byte=${#prefix}
  LC_ALL=$old_lc
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
  __automexia_suggestion_u32 "$((4 + byte_length + 4 + 4 + 4))" "$fd"
  __automexia_suggestion_u32 "$byte_length" "$fd"
  printf '%s' "$line" >&$fd
  __automexia_suggestion_u32 "$point_byte" "$fd"
  __automexia_suggestion_u32 "$__automexia_suggestion_generation" "$fd"
  __automexia_suggestion_u32 0 "$fd"
  zle redisplay
}

automexia_suggestions_enable() {
  local chord=${1-} existing
  automexia_suggestions_disable
  autoload -Uz is-at-least
  if ! is-at-least 5.8; then
    __automexia_suggestion_reason=unsupported-zsh
    return 1
  fi
  if [[ ${AUTOMEXIA_SUGGESTION_PREVIEW-0} != 1 ||
        ${AUTOMEXIA_SUGGESTION_REQUEST_FD-} != <-> ]]; then
    __automexia_suggestion_reason=preview-disabled
    return 1
  fi
  if ! : >&$AUTOMEXIA_SUGGESTION_REQUEST_FD 2>/dev/null; then
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
