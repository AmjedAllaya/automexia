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
system_name=$(uname -s)
if [ -n "${AUTOMEXIA_CONFIG_HOME:-}" ]; then
  config_root=$AUTOMEXIA_CONFIG_HOME
elif [ "$system_name" = Darwin ]; then
  config_root=$HOME/Library/Application\ Support/io.github.AmjedAllaya.AutomexiaTerminal
else
  config_root=${XDG_CONFIG_HOME:-"$HOME/.config"}/automexia
fi
fish_conf_root=${XDG_CONFIG_HOME:-"$HOME/.config"}/fish/conf.d
case "$config_root" in /*) ;; *) printf 'install-unix.sh: config root must be absolute\n' >&2; exit 1 ;; esac
case "$fish_conf_root" in /*) ;; *) printf 'install-unix.sh: Fish config root must be absolute\n' >&2; exit 1 ;; esac
state_file=$config_root/install-state-unix.sha256
marker_start='# >>> AUTOMEXIA SHELL INTEGRATION >>>'
marker_end='# <<< AUTOMEXIA SHELL INTEGRATION <<<'
if [ "$system_name" = Darwin ]; then
  bash_source_line='[ -r "${AUTOMEXIA_CONFIG_HOME:-$HOME/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal}/shell-integration.bash" ] && . "${AUTOMEXIA_CONFIG_HOME:-$HOME/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal}/shell-integration.bash"'
  zsh_source_line='[ -r "${AUTOMEXIA_CONFIG_HOME:-$HOME/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal}/shell-integration.zsh" ] && . "${AUTOMEXIA_CONFIG_HOME:-$HOME/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal}/shell-integration.zsh"'
else
  bash_source_line='[ -r "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.bash" ] && . "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.bash"'
  zsh_source_line='[ -r "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.zsh" ] && . "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.zsh"'
fi

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
  printf '%s\n' 'schema=3'
  printf 'system=%s\n' "$system_name"
  for source in \
    "$script_dir/install-unix.sh" \
    "$script_dir/bash/automexia.bash" \
    "$script_dir/zsh/automexia.zsh" \
    "$script_dir/fish/automexia.fish" \
    "$script_dir/completion/bash/automexia-completion.bash" \
    "$script_dir/completion/zsh/automexia-completion.zsh" \
    "$script_dir/completion/fish/automexia-completion.fish" \
    "$script_dir/posix/automexia-os.sh" \
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
  cmp -s "$script_dir/posix/automexia-os.sh" "$config_root/automexia-os.sh" || return 1
  cmp -s "$script_dir/posix/automexia-os.sh" "$fish_conf_root/automexia-os.sh" || return 1
  cmp -s "$script_dir/posix/automexia-eza-filter.pl" "$config_root/automexia-eza-filter.pl" || return 1
  grep -Fq "$marker_start" "$HOME/.bashrc" 2>/dev/null || return 1
  grep -Fq "$marker_start" "$HOME/.zshrc" 2>/dev/null || return 1
  grep -Fqx "$bash_source_line" "$HOME/.bashrc" 2>/dev/null || return 1
  grep -Fqx "$zsh_source_line" "$HOME/.zshrc" 2>/dev/null || return 1
}

if [ "$force" -eq 0 ] && integration_is_current; then
  say 'Automexia shell integration is already current.'
  exit 0
fi

mkdir -p "$config_root" "$fish_conf_root"
[ ! -L "$config_root" ] || { printf 'install-unix.sh: config root must not be a symbolic link\n' >&2; exit 1; }
[ ! -L "$fish_conf_root" ] || { printf 'install-unix.sh: Fish config root must not be a symbolic link\n' >&2; exit 1; }
stage_dir=
stage_path=
stage_identity=
directory_identity() {
  stat -c '%d:%i' "$1" 2>/dev/null || stat -f '%d:%i' "$1"
}
cleanup() {
  [ -n "$stage_dir" ] || return 0
  [ -d "$stage_dir" ] && [ ! -L "$stage_dir" ] || return 0
  # The pathname can be replaced after staging. Enter and verify the private
  # directory so a replacement directory's payload is never removed.
  (
    cd -P "$stage_dir" 2>/dev/null || exit 0
    [ "$(directory_identity .)" = "$stage_identity" ] || exit 0
    rm -f ./payload
  )
  [ "$(directory_identity "$stage_dir" 2>/dev/null)" = "$stage_identity" ] || return 0
  rmdir "$stage_dir"
}
trap cleanup EXIT HUP INT TERM

new_stage() {
  # mktemp creates an unpredictable, private directory in the destination's
  # filesystem. Noclobber below creates its payload exactly once inside it.
  stage_dir=$(mktemp -d "$1.automexia.XXXXXXXX")
  [ -d "$stage_dir" ] && [ ! -L "$stage_dir" ] || return 1
  stage_path=$stage_dir/payload
  stage_identity=$(directory_identity "$stage_dir")
}

publish_stage() {
  destination=$1
  case "$destination" in /*) ;; *) destination=$(pwd -P)/$destination ;; esac
  (
    cd -P "$stage_dir"
    [ "$(directory_identity .)" = "$stage_identity" ] || exit 1
    mv -f ./payload "$destination"
  )
  [ "$(directory_identity "$stage_dir" 2>/dev/null)" = "$stage_identity" ] || return 1
  rmdir "$stage_dir"
  stage_dir=
  stage_path=
  stage_identity=
}

install_source() {
  source_path=$1
  destination_path=$2
  mode=$3
  [ ! -L "$destination_path" ] || {
    printf 'install-unix.sh: refusing linked destination: %s\n' "$destination_path" >&2
    exit 1
  }
  new_stage "$destination_path"
  (set -C; cat "$source_path" >"$stage_path")
  chmod "$mode" "$stage_path"
  publish_stage "$destination_path"
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
  if [ "$start_count" -ne 0 ] || [ "$end_count" -ne 0 ]; then
    if [ "$start_count" -ne 1 ] || [ "$end_count" -ne 1 ]; then
      printf 'install-unix.sh: malformed Automexia markers in %s\n' "$profile_path" >&2
      exit 1
    fi
    start_line=$(grep -Fn "$marker_start" "$profile_path" | cut -d: -f1)
    end_line=$(grep -Fn "$marker_end" "$profile_path" | cut -d: -f1)
    if [ "$start_line" -ge "$end_line" ]; then
      printf 'install-unix.sh: reversed Automexia markers in %s\n' "$profile_path" >&2
      exit 1
    fi
  fi
  new_stage "$profile_path"
  if [ "$start_count" -eq 1 ]; then
    permissions=$(stat -c '%a' "$profile_path" 2>/dev/null || stat -f '%Lp' "$profile_path")
    (set -C; awk -v start="$marker_start" -v end="$marker_end" '
      $0 == start { skip=1; next }
      $0 == end { skip=0; next }
      !skip { print }
    ' "$profile_path" >"$stage_path")
  elif [ -f "$profile_path" ]; then
    permissions=$(stat -c '%a' "$profile_path" 2>/dev/null || stat -f '%Lp' "$profile_path")
    (set -C; cat "$profile_path" >"$stage_path")
  else
    (set -C; : >"$stage_path")
  fi
  printf '\n%s\n%s\n%s\n' "$marker_start" "$source_line" "$marker_end" >>"$stage_path"
  if [ "$start_count" -eq 1 ] || [ -f "$profile_path" ]; then
    chmod "$permissions" "$stage_path"
  fi
  publish_stage "$profile_path"
}

install_source "$script_dir/bash/automexia.bash" "$config_root/shell-integration.bash" 0644
install_source "$script_dir/zsh/automexia.zsh" "$config_root/shell-integration.zsh" 0644
install_source "$script_dir/completion/bash/automexia-completion.bash" "$config_root/automexia-completion.bash" 0644
install_source "$script_dir/completion/zsh/automexia-completion.zsh" "$config_root/automexia-completion.zsh" 0644
install_source "$script_dir/fish/automexia.fish" "$fish_conf_root/automexia.fish" 0644
install_source "$script_dir/completion/fish/automexia-completion.fish" "$fish_conf_root/automexia-completion.fish" 0644
install_source "$script_dir/posix/automexia-os.sh" "$config_root/automexia-os.sh" 0644
install_source "$script_dir/posix/automexia-os.sh" "$fish_conf_root/automexia-os.sh" 0644
install_source "$script_dir/posix/automexia-eza-filter.pl" "$config_root/automexia-eza-filter.pl" 0644
append_block "$HOME/.bashrc" "$bash_source_line"
append_block "$HOME/.zshrc" "$zsh_source_line"

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

new_stage "$state_file"
(set -C; printf '%s\n' "$source_fingerprint" >"$stage_path")
publish_stage "$state_file"
say 'Automexia Bash/Zsh/Fish integration, managed completion adapters, and user terminfo are ready.'
