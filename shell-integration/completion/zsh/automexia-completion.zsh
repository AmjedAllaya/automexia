# Automexia CP1 Zsh completion adapter. It never invokes compinit and native
# `_comps` registrations remain authoritative.
[[ ${AUTOMEXIA_COMPLETION_ADAPTER_ZSH_LOADED:-0} == 1 ]] && return 0
typeset -gx AUTOMEXIA_COMPLETION_ADAPTER_ZSH_LOADED=1
typeset -g __automexia_completion_root=${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/generated/completion
typeset -g __automexia_completion_collisions=''
typeset -g __automexia_completion_loaded=''

__automexia_completion_directory_safe() {
  local generated_dir=${__automexia_completion_root:h}
  local config_dir=${generated_dir:h}
  local shell_dir="$__automexia_completion_root/zsh"
  local directory
  for directory in "$config_dir" "$generated_dir" "$__automexia_completion_root" "$shell_dir"; do
    [[ ! -h $directory && ( ! -e $directory || -d $directory ) ]] || return 1
  done
}

typeset -g __automexia_completion_path_safe=0
__automexia_completion_directory_safe && __automexia_completion_path_safe=1

__automexia_completion_hash() {
  local file=$1
  if (( $+commands[sha256sum] )); then
    command sha256sum -- "$file" | command awk '{print $1}'
  elif (( $+commands[shasum] )); then
    command shasum -a 256 -- "$file" | command awk '{print $1}'
  elif (( $+commands[openssl] )); then
    command openssl dgst -sha256 "$file" | command awk '{print $NF}'
  else
    return 127
  fi
}

__automexia_load_completion() {
  local target=$1 file expected actual
  file="$__automexia_completion_root/zsh/$target.zsh"
  [[ -f $file && ! -h $file && -f $file.sha256 && ! -h $file.sha256 ]] || return 0
  [[ $(wc -c <"$file") -le 1114112 && $(wc -c <"$file.sha256") -le 128 ]] || return 0
  if (( ${+_comps[$target]} )); then
    __automexia_completion_collisions="${__automexia_completion_collisions}${__automexia_completion_collisions:+,}$target"
    return 0
  fi
  (( $+functions[compdef] )) || return 0
  IFS= read -r expected <"$file.sha256" || return 0
  [[ $expected =~ '^[0-9a-f]{64}$' ]] || return 0
  actual=$(__automexia_completion_hash "$file") || return 0
  [[ $actual == "$expected" ]] || return 0
  source "$file"
  __automexia_completion_loaded="${__automexia_completion_loaded}${__automexia_completion_loaded:+,}$target"
}

automexia_completion_health() {
  local state=enabled
  if [[ $__automexia_completion_path_safe != 1 ]]; then
    state=unsafe-path/native-fallback
  elif [[ ${AUTOMEXIA_COMPLETION_DISABLED:-0} == 1 || -f $__automexia_completion_root/.disabled ]]; then
    state=disabled
  fi
  print -r -- "shell=zsh" "version=$ZSH_VERSION" "state=$state" \
    "loaded=${__automexia_completion_loaded:-none}" \
    "collisions=${__automexia_completion_collisions:-none}"
}

if [[ $__automexia_completion_path_safe == 1 && ${AUTOMEXIA_COMPLETION_DISABLED:-0} != 1 && ! -f $__automexia_completion_root/.disabled ]]; then
  for __automexia_completion_target in docker kubectl oc helm; do
    __automexia_load_completion "$__automexia_completion_target"
  done
  unset __automexia_completion_target
fi
