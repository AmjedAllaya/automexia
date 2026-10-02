#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
fixture=$(mktemp -d)
trap 'rm -rf -- "$fixture"' EXIT
mkdir -p "$fixture/home" "$fixture/xdg/automexia"
printf '%s\n' outside-sentinel >"$fixture/home/outside-sentinel"
printf '%s\n' existing-profile >"$fixture/home/.bashrc"
chmod 0640 "$fixture/home/.bashrc"

# Source the installer in an isolated shell so the old PID-based name is known
# before its first write. A symlink at that name must neither be followed nor
# removed by cleanup; the installer must still publish its own source file.
HOME="$fixture/home" XDG_CONFIG_HOME="$fixture/xdg" sh -c '
  legacy="$XDG_CONFIG_HOME/automexia/shell-integration.bash.automexia-$$.tmp"
  ln -s "$HOME/outside-sentinel" "$legacy"
  . "$0"
  [ "$(cat "$HOME/outside-sentinel")" = outside-sentinel ] || {
    printf "%s\n" "installer changed the precreated temporary link target" >&2
    exit 1
  }
  [ -L "$legacy" ] || {
    printf "%s\n" "installer removed the precreated temporary link" >&2
    exit 1
  }
' "$root/shell-integration/install-unix.sh" >/dev/null
cmp -s "$root/shell-integration/bash/automexia.bash" \
  "$fixture/xdg/automexia/shell-integration.bash"
[[ $(stat -c '%a' "$fixture/home/.bashrc" 2>/dev/null ||
      stat -f '%Lp' "$fixture/home/.bashrc") == 640 ]]

# The same collision in profile removal must not change an unrelated file.
HOME="$fixture/home" XDG_CONFIG_HOME="$fixture/xdg" sh -c '
  legacy="$HOME/.bashrc.automexia-$$.tmp"
  ln -s "$HOME/outside-sentinel" "$legacy"
  . "$0"
  [ "$(cat "$HOME/outside-sentinel")" = outside-sentinel ] || {
    printf "%s\n" "uninstaller changed the precreated temporary link target" >&2
    exit 1
  }
  [ -L "$legacy" ] || {
    printf "%s\n" "uninstaller removed the precreated temporary link" >&2
    exit 1
  }
' "$root/shell-integration/uninstall-unix.sh" >/dev/null
! grep -Fq '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$fixture/home/.bashrc"
grep -Fqx existing-profile "$fixture/home/.bashrc"
[[ $(stat -c '%a' "$fixture/home/.bashrc" 2>/dev/null ||
      stat -f '%Lp' "$fixture/home/.bashrc") == 640 ]]

# An error after exclusive staging must remove only that invocation's stage.
mkdir -p "$fixture/fail-home" "$fixture/fail-xdg/automexia" "$fixture/fail-bin"
cat >"$fixture/fail-bin/chmod" <<'CHMOD'
#!/bin/sh
case "$*" in */payload) exit 37 ;; esac
exec /bin/chmod "$@"
CHMOD
chmod 0755 "$fixture/fail-bin/chmod"
if HOME="$fixture/fail-home" XDG_CONFIG_HOME="$fixture/fail-xdg" \
    PATH="$fixture/fail-bin:$PATH" \
    sh "$root/shell-integration/install-unix.sh" --quiet >/dev/null 2>&1; then
  printf '%s\n' 'installer ignored a staged-file write failure' >&2
  exit 1
fi
if find "$fixture/fail-xdg/automexia" -maxdepth 1 -type d \
    -name '*.automexia.*' | grep -q .; then
  printf '%s\n' 'installer left an owned staging directory after failure' >&2
  exit 1
fi

# A replacement at the stage directory's name must not have its payload
# unlinked by the failure trap. The original private directory was moved away.
mkdir -p "$fixture/swap-home" "$fixture/swap-xdg/automexia" "$fixture/swap-bin"
cat >"$fixture/swap-bin/chmod" <<'CHMOD_SWAP'
#!/bin/sh
case "$*" in
  */payload)
    for staged_file do :; done
    stage_dir=${staged_file%/payload}
    mv "$stage_dir" "$stage_dir.saved"
    mkdir "$stage_dir"
    printf '%s\n' replacement-sentinel >"$stage_dir/payload"
    exit 37
    ;;
esac
exec /bin/chmod "$@"
CHMOD_SWAP
chmod 0755 "$fixture/swap-bin/chmod"
if HOME="$fixture/swap-home" XDG_CONFIG_HOME="$fixture/swap-xdg" \
    PATH="$fixture/swap-bin:$PATH" \
    sh "$root/shell-integration/install-unix.sh" --quiet >/dev/null 2>&1; then
  printf '%s\n' 'installer ignored a substituted staging directory' >&2
  exit 1
fi
if ! find "$fixture/swap-xdg/automexia" -type f -name payload \
    -exec grep -lFx replacement-sentinel {} + | grep -q .; then
  printf '%s\n' 'installer cleanup removed a replacement directory payload' >&2
  exit 1
fi

mkdir -p "$fixture/uninstall-swap-home" "$fixture/uninstall-swap-xdg"
cat >"$fixture/uninstall-swap-home/.bashrc" <<'PROFILE'
existing-profile
# >>> AUTOMEXIA SHELL INTEGRATION >>>
managed-hook
# <<< AUTOMEXIA SHELL INTEGRATION <<<
PROFILE
if HOME="$fixture/uninstall-swap-home" XDG_CONFIG_HOME="$fixture/uninstall-swap-xdg" \
    PATH="$fixture/swap-bin:$PATH" \
    sh "$root/shell-integration/uninstall-unix.sh" >/dev/null 2>&1; then
  printf '%s\n' 'uninstaller ignored a substituted staging directory' >&2
  exit 1
fi
if ! find "$fixture/uninstall-swap-home" -type f -name payload \
    -exec grep -lFx replacement-sentinel {} + | grep -q .; then
  printf '%s\n' 'uninstaller cleanup removed a replacement directory payload' >&2
  exit 1
fi

printf '%s\n' 'PASS: Unix shell staging rejects PID collisions and substituted directory cleanup'
