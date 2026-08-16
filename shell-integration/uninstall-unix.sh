#!/bin/sh
set -eu
umask 077

if [ -z "${HOME:-}" ]; then
  printf 'uninstall-unix.sh: HOME is unavailable\n' >&2
  exit 1
fi

config_home=${XDG_CONFIG_HOME:-"$HOME/.config"}
system_name=$(uname -s)
if [ -n "${AUTOMEXIA_CONFIG_HOME:-}" ]; then
  config_root=$AUTOMEXIA_CONFIG_HOME
elif [ "$system_name" = Darwin ]; then
  config_root=$HOME/Library/Application\ Support/io.github.AmjedAllaya.AutomexiaTerminal
else
  config_root=$config_home/automexia
fi
fish_conf_root=$config_home/fish/conf.d
case "$config_root" in /*) ;; *) printf 'uninstall-unix.sh: config root must be absolute\n' >&2; exit 1 ;; esac
case "$fish_conf_root" in /*) ;; *) printf 'uninstall-unix.sh: Fish config root must be absolute\n' >&2; exit 1 ;; esac
marker_start='# >>> AUTOMEXIA SHELL INTEGRATION >>>'
marker_end='# <<< AUTOMEXIA SHELL INTEGRATION <<<'
temporary_suffix=.automexia-$$.tmp

cleanup() {
  rm -f "$HOME/.bashrc$temporary_suffix" "$HOME/.zshrc$temporary_suffix"
}
trap cleanup EXIT HUP INT TERM

validate_marked_block() {
  profile=$1
  [ -e "$profile" ] || return 0
  [ ! -L "$profile" ] || {
    printf 'uninstall-unix.sh: refusing linked profile: %s\n' "$profile" >&2
    return 1
  }
  [ -f "$profile" ] || {
    printf 'uninstall-unix.sh: profile is not a regular file: %s\n' "$profile" >&2
    return 1
  }
  [ "$(wc -c <"$profile")" -le 1048576 ] || {
    printf 'uninstall-unix.sh: profile exceeds 1 MiB: %s\n' "$profile" >&2
    return 1
  }
  starts=$(grep -Fxc "$marker_start" "$profile" || true)
  ends=$(grep -Fxc "$marker_end" "$profile" || true)
  [ "$starts" -eq 0 ] && [ "$ends" -eq 0 ] && return 0
  [ "$starts" -eq 1 ] && [ "$ends" -eq 1 ] || {
    printf 'uninstall-unix.sh: malformed managed block; profile left unchanged: %s\n' "$profile" >&2
    return 1
  }
  start_line=$(grep -Fn "$marker_start" "$profile" | cut -d: -f1)
  end_line=$(grep -Fn "$marker_end" "$profile" | cut -d: -f1)
  [ "$start_line" -lt "$end_line" ] || {
    printf 'uninstall-unix.sh: reversed managed block; profile left unchanged: %s\n' "$profile" >&2
    return 1
  }
}

remove_marked_block() {
  profile=$1
  validate_marked_block "$profile"
  [ -f "$profile" ] || return 0
  starts=$(grep -Fxc "$marker_start" "$profile" || true)
  [ "$starts" -eq 0 ] && return 0
  awk -v start="$marker_start" -v end="$marker_end" '
    $0 == start { skip=1; next }
    $0 == end { skip=0; next }
    !skip { print }
  ' "$profile" >"$profile$temporary_suffix"
  if permissions=$(stat -c '%a' "$profile" 2>/dev/null); then
    :
  else
    permissions=$(stat -f '%Lp' "$profile")
  fi
  chmod "$permissions" "$profile$temporary_suffix"
  mv -f "$profile$temporary_suffix" "$profile"
}

assert_real_directory() {
  directory=$1
  [ ! -L "$directory" ] || {
    printf 'uninstall-unix.sh: refusing linked managed directory: %s\n' "$directory" >&2
    exit 1
  }
  if [ -e "$directory" ] && [ ! -d "$directory" ]; then
    printf 'uninstall-unix.sh: managed path is not a directory: %s\n' "$directory" >&2
    exit 1
  fi
}

assert_owned_file() {
  path=$1
  [ ! -L "$path" ] || {
    printf 'uninstall-unix.sh: refusing linked managed file: %s\n' "$path" >&2
    exit 1
  }
  if [ -e "$path" ] && [ ! -f "$path" ]; then
    printf 'uninstall-unix.sh: managed path is not a regular file: %s\n' "$path" >&2
    exit 1
  fi
}

remove_owned_file() {
  path=$1
  assert_owned_file "$path"
  rm -f "$path"
}

assert_real_directory "$config_root"
assert_real_directory "$fish_conf_root"
assert_real_directory "$config_root/generated"
completion_root=$config_root/generated/completion
assert_real_directory "$completion_root"
validate_marked_block "$HOME/.bashrc"
validate_marked_block "$HOME/.zshrc"

for owned in \
  "$config_root/shell-integration.bash" \
  "$config_root/shell-integration.zsh" \
  "$config_root/automexia-completion.bash" \
  "$config_root/automexia-completion.zsh" \
  "$config_root/automexia-eza-filter.pl" \
  "$config_root/install-state-unix.sha256" \
  "$fish_conf_root/automexia.fish" \
  "$fish_conf_root/automexia-completion.fish"; do
  assert_owned_file "$owned"
done
for shell in powershell bash zsh fish cmd; do
  case "$shell" in
    powershell) extension=ps1 ;;
    bash) extension=bash ;;
    zsh) extension=zsh ;;
    fish) extension=fish ;;
    cmd) extension=cmd ;;
  esac
  directory=$completion_root/$shell
  assert_real_directory "$directory"
  for command in git docker kubectl oc helm terraform tofu aws_completer az gcloud ssh; do
    artifact=$directory/$command.$extension
    assert_owned_file "$artifact"
    assert_owned_file "$artifact.sha256"
    assert_owned_file "$artifact.json"
    assert_owned_file "$artifact.allow-override"
  done
done
assert_owned_file "$completion_root/.disabled"

# All destructive targets are validated before the first mutation.
remove_marked_block "$HOME/.bashrc"
remove_marked_block "$HOME/.zshrc"

# Remove only exact Automexia-owned files. User/provider files and canonical
# Quick Action data (introduced only in CP2) remain untouched.
for owned in \
  "$config_root/shell-integration.bash" \
  "$config_root/shell-integration.zsh" \
  "$config_root/automexia-completion.bash" \
  "$config_root/automexia-completion.zsh" \
  "$config_root/automexia-eza-filter.pl" \
  "$config_root/install-state-unix.sha256" \
  "$fish_conf_root/automexia.fish" \
  "$fish_conf_root/automexia-completion.fish"; do
  remove_owned_file "$owned"
done

for shell in powershell bash zsh fish cmd; do
  case "$shell" in
    powershell) extension=ps1 ;;
    bash) extension=bash ;;
    zsh) extension=zsh ;;
    fish) extension=fish ;;
    cmd) extension=cmd ;;
  esac
  directory=$completion_root/$shell
  assert_real_directory "$directory"
  for command in git docker kubectl oc helm terraform tofu aws_completer az gcloud ssh; do
    artifact=$directory/$command.$extension
    remove_owned_file "$artifact"
    remove_owned_file "$artifact.sha256"
    remove_owned_file "$artifact.json"
    remove_owned_file "$artifact.allow-override"
  done
  rmdir "$directory" 2>/dev/null || true
done
remove_owned_file "$completion_root/.disabled"
rmdir "$completion_root" "$config_root/generated" "$config_root" "$fish_conf_root" 2>/dev/null || true
printf '%s\n' 'Automexia shell integration and managed completion files removed. Restart your shells.'
