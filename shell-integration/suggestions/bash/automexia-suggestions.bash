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

__automexia_suggestions_request() {
  [[ $__automexia_suggestion_state == ready ]] || return 0
  local fd=$AUTOMEXIA_SUGGESTION_REQUEST_FD line=$READLINE_LINE point=$READLINE_POINT
  local byte_length point_byte prefix old_lc=${LC_ALL-}
  prefix=${line:0:point}
  LC_ALL=C
  byte_length=${#line}
  point_byte=${#prefix}
  LC_ALL=$old_lc
  if ((byte_length > 16384)); then
    __automexia_suggestion_reason=buffer-limit
    return 0
  fi
  ((__automexia_suggestion_generation += 1))
  # The session helper supplies route identity/capability and performs schema-1
  # framing. This record contains only bounded editor-owned state.
  printf 'AXSH\001\000\001' >&"$fd" || {
    automexia_suggestions_disable
    __automexia_suggestion_reason=helper-disconnected
    return 0
  }
  __automexia_suggestion_u32 "$((4 + byte_length + 4 + 4 + 4))" "$fd"
  __automexia_suggestion_u32 "$byte_length" "$fd"
  printf '%s' "$line" >&"$fd"
  __automexia_suggestion_u32 "$point_byte" "$fd"
  __automexia_suggestion_u32 "$__automexia_suggestion_generation" "$fd"
  __automexia_suggestion_u32 0 "$fd"
}

automexia_suggestions_enable() {
  local chord=${1-} existing
  automexia_suggestions_disable
  if ((BASH_VERSINFO[0] < 5)); then
    __automexia_suggestion_reason=unsupported-bash
    return 1
  fi
  if [[ ${AUTOMEXIA_SUGGESTION_PREVIEW-0} != 1 ||
        ! ${AUTOMEXIA_SUGGESTION_REQUEST_FD-} =~ ^[0-9]+$ ]]; then
    __automexia_suggestion_reason=preview-disabled
    return 1
  fi
  if ! : >&"$AUTOMEXIA_SUGGESTION_REQUEST_FD" 2>/dev/null; then
    __automexia_suggestion_reason=helper-unavailable
    return 1
  fi
  __automexia_suggestion_state=ready-unbound
  __automexia_suggestion_reason=none
  [[ -n $chord ]] || return 0
  while IFS= read -r existing; do
    if [[ $existing == *"\"$chord\""* ]]; then
      __automexia_suggestion_reason=binding-collision
      return 1
    fi
  done < <(bind -P; bind -X; bind -S)
  bind -x "\"$chord\":__automexia_suggestions_request" || {
    __automexia_suggestion_reason=binding-failed
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
