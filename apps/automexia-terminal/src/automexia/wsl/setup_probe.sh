#!/bin/sh
# This fixed probe is sent to WSL on stdin. Its stdout is a bounded protocol.
export LC_ALL=C

distro=${WSL_DISTRO_NAME:-}
case $distro in
  [!A-Za-z0-9]*|*[!A-Za-z0-9._-]*) distro='' ;;
esac
if [ "${#distro}" -gt 96 ]; then distro=''; fi

shell_path=${SHELL:-}
shell=${shell_path##*/}
case $shell in
  bash|zsh|fish) ;;
  *) shell=other ;;
esac

home=${HOME:-}
case $home in /*) ;; *) home='' ;; esac
xdg=${XDG_CONFIG_HOME:-${home:+$home/.config}}
case $xdg in /*) ;; *) xdg='' ;; esac
config=${AUTOMEXIA_CONFIG_HOME:-${xdg:+$xdg/automexia}}
case $config in /*) ;; *) config='' ;; esac

regular_file() {
  [ -f "$1" ] && [ ! -L "$1" ] && [ -r "$1" ]
}

has_managed_hook() {
  regular_file "$1" || return 1
  # Read at most the installer profile limit plus one byte. A complete block
  # has exactly the installer-owned start, source statement, and end lines.
  command -p dd if="$1" bs=1048577 count=1 2>/dev/null |
    command -p awk -v source="$2" '
      {
        bytes += length($0) + 1
        if (bytes > 1048576) exit 1
        if ($0 == "# >>> AUTOMEXIA SHELL INTEGRATION >>>") {
          if (state != 0) exit 1
          state = 1
        } else if ($0 == "# <<< AUTOMEXIA SHELL INTEGRATION <<<") {
          if (state != 2) exit 1
          state = 3
        } else if (state == 1 && $0 == source) {
          state = 2
        } else if (state == 1 || state == 2) {
          exit 1
        }
      }
      END { if (state != 3) exit 1 }
    ' 2>/dev/null
}

hook=missing
icons=unsupported
case $shell in
  bash|zsh)
    case $shell in
      bash)
        rc=.bashrc
        source_line='[ -r "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.bash" ] && . "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.bash"'
        ;;
      zsh)
        rc=.zshrc
        source_line='[ -r "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.zsh" ] && . "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.zsh"'
        ;;
    esac
    if [ -n "$config" ] && regular_file "$config/shell-integration.$shell"; then
      hook=adapter-only
      if [ -n "$home" ] && has_managed_hook "$home/$rc" "$source_line"; then
        hook=managed
      fi
    fi
    if [ "${AUTOMEXIA_SHELL_INTEGRATION:-}" = 0 ] || [ "${AUTOMEXIA_PLAIN_LS:-}" = 1 ]; then
      icons=disabled
    elif command -v eza >/dev/null 2>&1; then
      icons=present
    else
      icons=missing
    fi
    ;;
  fish)
    if [ -n "$xdg" ] && regular_file "$xdg/fish/conf.d/automexia.fish"; then
      hook=managed
    fi
    ;;
  *) hook=unsupported ;;
esac

has_approved_os_id() {
  # Stock Debian and Ubuntu may symlink this system-owned file.
  [ -f /etc/os-release ] && [ -r /etc/os-release ] || return 1
  command -p dd if=/etc/os-release bs=65537 count=1 2>/dev/null |
    command -p awk '
      {
        bytes += length($0) + 1
        if (bytes > 65536) exit 1
      }
      /^ID=/ {
        ids++
        if ($0 ~ /^ID=(debian|ubuntu)$/ ||
            $0 ~ /^ID="(debian|ubuntu)"$/ ||
            $0 ~ /^ID=\047(debian|ubuntu)\047$/) approved = 1
      }
      END { if (ids != 1 || !approved) exit 1 }
    ' 2>/dev/null
}

installer=unavailable
# OS-owned command paths can be installed through the alternatives system.
# Follow those links while still requiring executable regular-file targets.
if [ -n "$distro" ] && [ -f /usr/bin/sudo ] && [ -x /usr/bin/sudo ] &&
   [ -f /usr/bin/apt-get ] && [ -x /usr/bin/apt-get ] && has_approved_os_id; then
  installer=apt
fi

bash_uid=-
if [ "$shell" = bash ] && [ -n "$home" ] && [ -n "$config" ] &&
   [ "${AUTOMEXIA_SHELL_INTEGRATION:-}" != 0 ]; then
  uid=$(command -p id -u 2>/dev/null) || uid=''
  case $uid in
    [1-9]|[1-9][0-9]*)
      case $uid in *[!0-9]*) ;; *) bash_uid=$uid ;; esac
      ;;
  esac
fi

printf '%s\n' amx-wsl-setup-v2 "$distro" "$shell" "$hook" "$icons" "$installer" "$bash_uid"
