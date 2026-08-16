#!/bin/sh
set -eu
umask 077

quiet=0
force=0
for argument in "$@"; do
  case "$argument" in
    --quiet) quiet=1 ;;
    --force) force=1 ;;
    *) printf 'install-unix.sh: unknown argument: %s\n' "$argument" >&2; exit 2 ;;
  esac
done

if [ -z "${HOME:-}" ]; then
  printf 'install-unix.sh: HOME is unavailable\n' >&2
  exit 1
fi

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH='' cd -- "$script_dir/.." && pwd)
config_root=${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-"$HOME/.config"}/automexia}
fish_conf_root=${XDG_CONFIG_HOME:-"$HOME/.config"}/fish/conf.d
state_file=$config_root/install-state-unix.sha256
marker_start='# >>> AUTOMEXIA SHELL INTEGRATION >>>'
marker_end='# <<< AUTOMEXIA SHELL INTEGRATION <<<'
temporary_suffix=.automexia-$$.tmp

say() {
  if [ "$quiet" -eq 0 ]; then printf '%s\n' "$1"; fi
}

hash_stream() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 | awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 | awk '{print $NF}'
  else
    printf 'install-unix.sh: sha256sum, shasum, or openssl is required\n' >&2
    return 1
  fi
}

source_fingerprint=$(
  printf '%s\n' 'schema=2'
  for source in \
    "$script_dir/install-unix.sh" \
    "$script_dir/bash/automexia.bash" \
    "$script_dir/zsh/automexia.zsh" \
    "$script_dir/fish/automexia.fish" \
    "$script_dir/completion/bash/automexia-completion.bash" \
    "$script_dir/completion/zsh/automexia-completion.zsh" \
    "$script_dir/completion/fish/automexia-completion.fish" \
    "$script_dir/posix/automexia-eza-filter.pl" \
    "$repository_root/packaging/linux/automexia.terminfo"; do
    [ -f "$source" ] || {
      printf 'install-unix.sh: required source is missing: %s\n' "$source" >&2
      exit 1
    }
    printf '%s\n' "$source"
    hash_stream <"$source"
  done
  printf 'config=%s\n' "$config_root"
) || exit 1
source_fingerprint=$(printf '%s' "$source_fingerprint" | hash_stream)

integration_is_current() {
  [ -f "$state_file" ] || return 1
  [ ! -L "$state_file" ] || return 1
  [ "$(wc -c <"$state_file")" -le 256 ] || return 1
  [ "$(cat "$state_file")" = "$source_fingerprint" ] || return 1
  cmp -s "$script_dir/bash/automexia.bash" "$config_root/shell-integration.bash" || return 1
  cmp -s "$script_dir/zsh/automexia.zsh" "$config_root/shell-integration.zsh" || return 1
  cmp -s "$script_dir/completion/bash/automexia-completion.bash" "$config_root/automexia-completion.bash" || return 1
  cmp -s "$script_dir/completion/zsh/automexia-completion.zsh" "$config_root/automexia-completion.zsh" || return 1
  cmp -s "$script_dir/fish/automexia.fish" "$fish_conf_root/automexia.fish" || return 1
  cmp -s "$script_dir/completion/fish/automexia-completion.fish" "$fish_conf_root/automexia-completion.fish" || return 1
  cmp -s "$script_dir/posix/automexia-eza-filter.pl" "$config_root/automexia-eza-filter.pl" || return 1
  grep -Fq "$marker_start" "$HOME/.bashrc" 2>/dev/null || return 1
  grep -Fq "$marker_start" "$HOME/.zshrc" 2>/dev/null || return 1
}

if [ "$force" -eq 0 ] && integration_is_current; then
  say 'Automexia shell integration is already current.'
  exit 0
fi

mkdir -p "$config_root" "$fish_conf_root"
[ ! -L "$config_root" ] || { printf 'install-unix.sh: config root must not be a symbolic link\n' >&2; exit 1; }
[ ! -L "$fish_conf_root" ] || { printf 'install-unix.sh: Fish config root must not be a symbolic link\n' >&2; exit 1; }
cleanup() {
  rm -f \
    "$config_root/shell-integration.bash$temporary_suffix" \
    "$config_root/shell-integration.zsh$temporary_suffix" \
    "$config_root/automexia-completion.bash$temporary_suffix" \
    "$config_root/automexia-completion.zsh$temporary_suffix" \
    "$fish_conf_root/automexia.fish$temporary_suffix" \
    "$fish_conf_root/automexia-completion.fish$temporary_suffix" \
    "$config_root/automexia-eza-filter.pl$temporary_suffix" \
    "$state_file$temporary_suffix"
}
trap cleanup EXIT HUP INT TERM

install_source() {
  source_path=$1
  destination_path=$2
  mode=$3
  [ ! -L "$destination_path" ] || {
    printf 'install-unix.sh: refusing linked destination: %s\n' "$destination_path" >&2
    exit 1
  }
  cp "$source_path" "$destination_path$temporary_suffix"
  chmod "$mode" "$destination_path$temporary_suffix"
  mv -f "$destination_path$temporary_suffix" "$destination_path"
}

append_block() {
  profile_path=$1
  source_line=$2
  [ ! -L "$profile_path" ] || {
    printf 'install-unix.sh: refusing linked profile: %s\n' "$profile_path" >&2
    exit 1
  }
  profile_directory=$(dirname -- "$profile_path")
  mkdir -p "$profile_directory"
  [ ! -L "$profile_directory" ] || {
    printf 'install-unix.sh: refusing linked profile directory: %s\n' "$profile_directory" >&2
    exit 1
  }
  if [ -e "$profile_path" ] && [ ! -f "$profile_path" ]; then
    printf 'install-unix.sh: profile is not a regular file: %s\n' "$profile_path" >&2
    exit 1
  fi
  if [ -f "$profile_path" ] && [ "$(wc -c <"$profile_path")" -gt 1048576 ]; then
    printf 'install-unix.sh: profile exceeds the 1 MiB safety limit: %s\n' "$profile_path" >&2
    exit 1
  fi
  start_count=0
  end_count=0
  if [ -f "$profile_path" ]; then
    start_count=$(grep -Fxc "$marker_start" "$profile_path" 2>/dev/null || true)
    end_count=$(grep -Fxc "$marker_end" "$profile_path" 2>/dev/null || true)
    start_count=${start_count:-0}
    end_count=${end_count:-0}
  fi
  if [ "$start_count" -eq 1 ] && [ "$end_count" -eq 1 ]; then return 0; fi
  if [ "$start_count" -ne 0 ] || [ "$end_count" -ne 0 ]; then
    printf 'install-unix.sh: malformed Automexia markers in %s\n' "$profile_path" >&2
    exit 1
  fi
  profile_temporary="$profile_path$temporary_suffix"
  if [ -f "$profile_path" ]; then cp -p "$profile_path" "$profile_temporary"; else : >"$profile_temporary"; fi
  printf '\n%s\n%s\n%s\n' "$marker_start" "$source_line" "$marker_end" >>"$profile_temporary"
  mv -f "$profile_temporary" "$profile_path"
}

install_source "$script_dir/bash/automexia.bash" "$config_root/shell-integration.bash" 0644
install_source "$script_dir/zsh/automexia.zsh" "$config_root/shell-integration.zsh" 0644
install_source "$script_dir/completion/bash/automexia-completion.bash" "$config_root/automexia-completion.bash" 0644
install_source "$script_dir/completion/zsh/automexia-completion.zsh" "$config_root/automexia-completion.zsh" 0644
install_source "$script_dir/fish/automexia.fish" "$fish_conf_root/automexia.fish" 0644
install_source "$script_dir/completion/fish/automexia-completion.fish" "$fish_conf_root/automexia-completion.fish" 0644
install_source "$script_dir/posix/automexia-eza-filter.pl" "$config_root/automexia-eza-filter.pl" 0644
append_block "$HOME/.bashrc" '[ -r "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.bash" ] && . "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.bash"'
append_block "$HOME/.zshrc" '[ -r "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.zsh" ] && . "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.zsh"'

# User-local terminfo avoids requiring root. Missing tic is non-fatal because
# Automexia already falls back to xterm-256color when a custom entry is absent.
if command -v tic >/dev/null 2>&1; then
  mkdir -p "$HOME/.terminfo"
  if [ "$quiet" -eq 1 ]; then
    tic -x -o "$HOME/.terminfo" "$repository_root/packaging/linux/automexia.terminfo" >/dev/null 2>&1 ||
      tic -o "$HOME/.terminfo" "$repository_root/packaging/linux/automexia.terminfo" >/dev/null 2>&1 || true
  else
    tic -x -o "$HOME/.terminfo" "$repository_root/packaging/linux/automexia.terminfo" ||
      tic -o "$HOME/.terminfo" "$repository_root/packaging/linux/automexia.terminfo" || true
  fi
fi

printf '%s\n' "$source_fingerprint" >"$state_file$temporary_suffix"
mv -f "$state_file$temporary_suffix" "$state_file"
say 'Automexia Bash/Zsh/Fish integration, managed completion adapters, and user terminfo are ready.'
