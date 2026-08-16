#!/bin/sh
set -eu
umask 077

if [ -z "${HOME:-}" ]; then
  printf 'uninstall-unix.sh: HOME is unavailable\n' >&2
  exit 1
fi

config_home=${XDG_CONFIG_HOME:-"$HOME/.config"}
config_root=${AUTOMEXIA_CONFIG_HOME:-$config_home/automexia}
fish_conf_root=$config_home/fish/conf.d
marker_start='# >>> AUTOMEXIA SHELL INTEGRATION >>>'
marker_end='# <<< AUTOMEXIA SHELL INTEGRATION <<<'
temporary_suffix=.automexia-$$.tmp

cleanup() {
  rm -f "$HOME/.bashrc$temporary_suffix" "$HOME/.zshrc$temporary_suffix"
}
trap cleanup EXIT HUP INT TERM

remove_marked_block() {
  profile=$1
  [ -f "$profile" ] || return 0
  [ ! -L "$profile" ] || {
    printf 'uninstall-unix.sh: refusing linked profile: %s\n' "$profile" >&2
    return 1
  }
  starts=$(grep -Fxc "$marker_start" "$profile" || true)
  ends=$(grep -Fxc "$marker_end" "$profile" || true)
  [ "$starts" -eq 0 ] && [ "$ends" -eq 0 ] && return 0
  [ "$starts" -eq 1 ] && [ "$ends" -eq 1 ] || {
    printf 'uninstall-unix.sh: malformed managed block; profile left unchanged: %s\n' "$profile" >&2
    return 1
  }
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

remove_marked_block "$HOME/.bashrc"
remove_marked_block "$HOME/.zshrc"

# Remove only exact Automexia-owned files. User/provider files and canonical
# Quick Action data (introduced only in CP2) remain untouched.
rm -f \
  "$config_root/shell-integration.bash" \
  "$config_root/shell-integration.zsh" \
  "$config_root/automexia-completion.bash" \
  "$config_root/automexia-completion.zsh" \
  "$config_root/automexia-eza-filter.pl" \
  "$config_root/install-state-unix.sha256" \
  "$fish_conf_root/automexia.fish" \
  "$fish_conf_root/automexia-completion.fish"

completion_root=$config_root/generated/completion
for shell in powershell bash zsh fish cmd; do
  case "$shell" in
    powershell) extension=ps1 ;;
    bash) extension=bash ;;
    zsh) extension=zsh ;;
    fish) extension=fish ;;
    cmd) extension=cmd ;;
  esac
  directory=$completion_root/$shell
  [ ! -L "$directory" ] || {
    printf 'uninstall-unix.sh: refusing linked completion directory: %s\n' "$directory" >&2
    exit 1
  }
  for command in git docker kubectl oc helm terraform tofu aws_completer az gcloud ssh; do
    artifact=$directory/$command.$extension
    rm -f "$artifact" "$artifact.sha256" "$artifact.json" "$artifact.allow-override"
  done
  rmdir "$directory" 2>/dev/null || true
done
rm -f "$completion_root/.disabled"
rmdir "$completion_root" "$config_root/generated" "$config_root" "$fish_conf_root" 2>/dev/null || true
printf '%s\n' 'Automexia shell integration and managed completion files removed. Restart your shells.'
