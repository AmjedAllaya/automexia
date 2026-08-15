#!/bin/sh
set -eu

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
config_root=${XDG_CONFIG_HOME:-"$HOME/.config"}/automexia
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
  printf '%s\n' 'schema=1'
  for source in \
    "$script_dir/install-unix.sh" \
    "$script_dir/bash/automexia.bash" \
    "$script_dir/zsh/automexia.zsh" \
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
  [ "$(cat "$state_file")" = "$source_fingerprint" ] || return 1
  cmp -s "$script_dir/bash/automexia.bash" "$config_root/shell-integration.bash" || return 1
  cmp -s "$script_dir/zsh/automexia.zsh" "$config_root/shell-integration.zsh" || return 1
  cmp -s "$script_dir/posix/automexia-eza-filter.pl" "$config_root/automexia-eza-filter.pl" || return 1
  grep -Fq "$marker_start" "$HOME/.bashrc" 2>/dev/null || return 1
  grep -Fq "$marker_start" "$HOME/.zshrc" 2>/dev/null || return 1
}

if [ "$force" -eq 0 ] && integration_is_current; then
  say 'Automexia shell integration is already current.'
  exit 0
fi

mkdir -p "$config_root"
cleanup() {
  rm -f \
    "$config_root/shell-integration.bash$temporary_suffix" \
    "$config_root/shell-integration.zsh$temporary_suffix" \
    "$config_root/automexia-eza-filter.pl$temporary_suffix" \
    "$state_file$temporary_suffix"
}
trap cleanup EXIT HUP INT TERM

install_source() {
  source_path=$1
  destination_path=$2
  mode=$3
  cp "$source_path" "$destination_path$temporary_suffix"
  chmod "$mode" "$destination_path$temporary_suffix"
  mv -f "$destination_path$temporary_suffix" "$destination_path"
}

append_block() {
  profile_path=$1
  source_line=$2
  touch "$profile_path"
  if ! grep -Fq "$marker_start" "$profile_path" 2>/dev/null; then
    printf '\n%s\n%s\n%s\n' "$marker_start" "$source_line" "$marker_end" >>"$profile_path"
  fi
}

install_source "$script_dir/bash/automexia.bash" "$config_root/shell-integration.bash" 0644
install_source "$script_dir/zsh/automexia.zsh" "$config_root/shell-integration.zsh" 0644
install_source "$script_dir/posix/automexia-eza-filter.pl" "$config_root/automexia-eza-filter.pl" 0644
append_block "$HOME/.bashrc" '[ -r "${XDG_CONFIG_HOME:-$HOME/.config}/automexia/shell-integration.bash" ] && . "${XDG_CONFIG_HOME:-$HOME/.config}/automexia/shell-integration.bash"'
append_block "$HOME/.zshrc" '[ -r "${XDG_CONFIG_HOME:-$HOME/.config}/automexia/shell-integration.zsh" ] && . "${XDG_CONFIG_HOME:-$HOME/.config}/automexia/shell-integration.zsh"'

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
say 'Automexia Bash/Zsh integration and user terminfo are ready.'
