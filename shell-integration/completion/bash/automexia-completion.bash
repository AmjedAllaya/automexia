# Automexia CP1 Bash completion adapter. Native definitions always win.
[[ ${AUTOMEXIA_COMPLETION_ADAPTER_BASH_LOADED:-0} == 1 ]] && return 0
export AUTOMEXIA_COMPLETION_ADAPTER_BASH_LOADED=1

if [[ -n ${AUTOMEXIA_CONFIG_HOME:-} ]]; then
  __automexia_completion_config_root=$AUTOMEXIA_CONFIG_HOME
elif [[ ${OSTYPE:-} == darwin* ]]; then
  __automexia_completion_config_root="$HOME/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal"
else
  __automexia_completion_config_root=${XDG_CONFIG_HOME:-$HOME/.config}/automexia
fi
__automexia_completion_root=$__automexia_completion_config_root/generated/completion
__automexia_completion_collisions=''
__automexia_completion_loaded=''

__automexia_completion_directory_safe() {
  local generated_dir config_dir shell_dir directory
  [[ $__automexia_completion_config_root == /* ]] || return 1
  generated_dir=${__automexia_completion_root%/*}
  config_dir=${generated_dir%/*}
  shell_dir="$__automexia_completion_root/bash"
  for directory in "$config_dir" "$generated_dir" "$__automexia_completion_root" "$shell_dir"; do
    [[ ! -L $directory && ( ! -e $directory || -d $directory ) ]] || return 1
  done
}

__automexia_completion_path_safe=0
__automexia_completion_directory_safe && __automexia_completion_path_safe=1

__automexia_completion_hash() {
  local file=$1
  if command -v sha256sum >/dev/null 2>&1; then
    command sha256sum -- "$file" | command awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    command shasum -a 256 -- "$file" | command awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    command openssl dgst -sha256 "$file" | command awk '{print $NF}'
  else
    return 127
  fi
}

__automexia_load_completion() {
  local target=$1 file expected actual
  file="$__automexia_completion_root/bash/$target.bash"
  [[ -f $file && ! -L $file && -f $file.sha256 && ! -L $file.sha256 ]] || return 0
  [[ $(wc -c <"$file") -le 1114112 && $(wc -c <"$file.sha256") -le 128 ]] || return 0
  if complete -p "$target" >/dev/null 2>&1; then
    __automexia_completion_collisions="${__automexia_completion_collisions}${__automexia_completion_collisions:+,}$target"
    return 0
  fi
  IFS= read -r expected <"$file.sha256" || return 0
  [[ $expected =~ ^[0-9a-f]{64}$ ]] || return 0
  actual=$(__automexia_completion_hash "$file") || return 0
  [[ $actual == "$expected" ]] || return 0
  # shellcheck disable=SC1090 # The exact, user-private, digest-verified path is intentional.
  . "$file"
  __automexia_completion_loaded="${__automexia_completion_loaded}${__automexia_completion_loaded:+,}$target"
}

automexia_completion_health() {
  local state=enabled
  if [[ $__automexia_completion_path_safe != 1 ]]; then
    state=unsafe-path/native-fallback
  elif [[ ${AUTOMEXIA_COMPLETION_DISABLED:-0} == 1 || -f $__automexia_completion_root/.disabled ]]; then
    state=disabled
  fi
  printf 'shell=bash\nversion=%s\nstate=%s\nloaded=%s\ncollisions=%s\n' \
    "$BASH_VERSION" \
    "$state" \
    "${__automexia_completion_loaded:-none}" \
    "${__automexia_completion_collisions:-none}"
}

if [[ $__automexia_completion_path_safe == 1 && ${AUTOMEXIA_COMPLETION_DISABLED:-0} != 1 && ! -f $__automexia_completion_root/.disabled ]]; then
  for __automexia_completion_target in docker kubectl oc helm; do
    __automexia_load_completion "$__automexia_completion_target"
  done
  unset __automexia_completion_target
fi
