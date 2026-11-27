# The caller prepends install_bash_assets(), which supplies four bundled files.
# This script runs through /bin/sh -s -- <distribution> <uid> in the WSL account.
set -eu
export LC_ALL=C
umask 077

fail() {
  printf '%s\n' "$1" >&2
  exit 1
}

[ "$#" -eq 2 ] || fail 'The WSL Bash installation target is invalid.'
uid=$(command -p id -u 2>/dev/null) || fail 'The WSL account could not be verified.'
[ "$uid" != 0 ] && [ "$uid" = "$2" ] && [ "${WSL_DISTRO_NAME:-}" = "$1" ] ||
  fail 'The WSL distribution or account changed after setup was reviewed.'
[ "${AUTOMEXIA_SHELL_INTEGRATION:-}" != 0 ] || fail 'Shell integration is disabled in this WSL account.'
shell=${SHELL:-}
[ "${shell##*/}" = bash ] || fail 'The WSL account no longer starts Bash.'

home=${HOME:-}
case $home in
  /|/mnt/[A-Za-z]|/mnt/[A-Za-z]/*|/mnt/wsl|/mnt/wsl/*|/mnt/wslg|/mnt/wslg/*)
    fail 'Bash integration requires a home directory on the Linux filesystem.' ;;
  /*) ;;
  *) fail 'The WSL home directory is unavailable.' ;;
esac

owned_directory() {
  [ ! -L "$1" ] && [ -d "$1" ] &&
    [ "$(command -p stat -c '%u' -- "$1" 2>/dev/null)" = "$uid" ]
}

owned_directory "$home" && [ "$(cd -P "$home" 2>/dev/null && pwd -P)" = "$home" ] ||
  fail 'The WSL home directory is linked or belongs to another account.'

cfg=${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$home/.config}/automexia}
case $cfg in
  "$home"/?*) ;;
  *) fail 'The Automexia config directory must be inside this WSL home.' ;;
esac
case $cfg/ in
  *//*|*/./*|*/../*) fail 'The Automexia config directory is not normalized.' ;;
esac
[ "${#cfg}" -le 4096 ] || fail 'The Automexia config path exceeds its size limit.'

check_config_directories() {
  create=$1
  relative=${cfg#"$home"/}
  directory=$home
  depth=0
  while [ -n "$relative" ]; do
    depth=$((depth + 1))
    [ "$depth" -le 32 ] || fail 'The Automexia config directory is nested too deeply.'
    directory=$directory/${relative%%/*}
    case $relative in */*) relative=${relative#*/} ;; *) relative= ;; esac
    if [ -L "$directory" ] || [ -e "$directory" ]; then
      owned_directory "$directory" || fail 'An Automexia config directory is linked or belongs to another account.'
    elif [ "$create" = yes ]; then
      command -p mkdir -- "$directory" 2>/dev/null ||
        owned_directory "$directory" || fail 'The Automexia config directory could not be created.'
    fi
  done
}

file_limit=1048576
check_file() {
  [ ! -L "$1" ] || fail 'A Bash installation file is linked.'
  if [ -e "$1" ]; then
    [ -f "$1" ] && [ -r "$1" ] &&
      [ "$(command -p stat -c '%u:%h' -- "$1" 2>/dev/null)" = "$uid:1" ] ||
      fail 'A Bash installation file is unreadable, linked, or belongs to another account.'
    size=$(command -p stat -c '%s' -- "$1" 2>/dev/null) || fail 'A Bash installation file could not be inspected.'
    [ "$size" -le "$file_limit" ] || fail 'A Bash installation file exceeds its size limit.'
  fi
}

directory_identity() {
  command -p stat -c '%d:%i' -- "$1" 2>/dev/null
}

enter_original_directory() {
  cd -P "$1" 2>/dev/null && [ "$(pwd -P)" = "$1" ] && [ "$(directory_identity .)" = "$2" ]
}

profile_stage=
profile_stage_id=
asset_stage=
asset_stage_id=

remove_stage() {
  [ -n "$1" ] && [ -n "$2" ] && owned_directory "$1" || return 0
  (
    cd -P "$1" && [ "$(directory_identity .)" = "$2" ] || exit 0
    command -p rm -f -- ./original ./profile ./shell-integration.bash \
      ./automexia-completion.bash ./automexia-os.sh ./automexia-eza-filter.pl
  ) 2>/dev/null || return 0
  [ "$(directory_identity "$1")" = "$2" ] || return 0
  command -p rmdir -- "$1" 2>/dev/null || :
}

cleanup() {
  remove_stage "$asset_stage" "$asset_stage_id"
  remove_stage "$profile_stage" "$profile_stage_id"
}
trap cleanup 0
trap 'exit 1' HUP INT TERM

profile=$home/.bashrc
marker_start='# >>> AUTOMEXIA SHELL INTEGRATION >>>'
marker_end='# <<< AUTOMEXIA SHELL INTEGRATION <<<'
source_line='[ -r "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.bash" ] && . "${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.bash"'
assets='shell-integration.bash automexia-completion.bash automexia-os.sh automexia-eza-filter.pl'

check_file "$profile"
check_config_directories no
for asset in $assets; do check_file "$cfg/$asset"; done

home_id=$(directory_identity "$home") || fail 'The WSL home directory could not be inspected.'
profile_stage=$(command -p mktemp -d -- "$home/.automexia-bash.XXXXXXXX" 2>/dev/null) ||
  fail 'The Bash profile staging directory could not be created.'
profile_stage_id=$(directory_identity "$profile_stage") || fail 'The Bash profile staging directory could not be inspected.'

profile_exists=no
if [ -f "$profile" ]; then
  profile_exists=yes
  profile_mode=$(command -p stat -c '%a' -- "$profile" 2>/dev/null) || fail 'The Bash profile mode could not be read.'
  (set -C; command -p dd if="$profile" bs=1048577 count=1 >"$profile_stage/original") 2>/dev/null ||
    fail 'The Bash profile could not be read.'
  check_file "$profile_stage/original"
else
  profile_mode=600
  (set -C; : >"$profile_stage/original") 2>/dev/null || fail 'The empty Bash profile could not be staged.'
fi

# Only the established three-line managed block grants permission to refresh
# different existing asset bytes. Refuse unknown or malformed marked content.
hook=$(command -p awk -v start="$marker_start" -v end="$marker_end" -v source="$source_line" '
  $0 == start { if (state != 0) exit 1; state = 1; next }
  $0 == end { if (state != 2) exit 1; state = 3; next }
  state == 1 && $0 == source { state = 2; next }
  state == 1 || state == 2 { exit 1 }
  { line = $0; gsub(/^[ \t]+|[ \t\r]+$/, "", line) }
  line == start || line == end { exit 1 }
  END {
    if (state == 3) print "managed"
    else if (state == 0) print "absent"
    else exit 1
  }
' "$profile_stage/original" 2>/dev/null) || fail 'The existing Automexia Bash profile block could not be verified.'

if [ "$hook" = absent ]; then
  (
    set -C
    {
      command -p cat -- "$profile_stage/original" || exit 1
      if [ -s "$profile_stage/original" ]; then printf '\n' || exit 1; fi
      printf '%s\n' "$marker_start" "$source_line" "$marker_end"
    } >"$profile_stage/profile"
  ) 2>/dev/null || fail 'The Bash profile update could not be staged.'
  check_file "$profile_stage/profile"
  command -p chmod -- "$profile_mode" "$profile_stage/profile" 2>/dev/null || fail 'The Bash profile mode could not be preserved.'
fi

assert_profile_unchanged() {
  check_file "$profile"
  if [ "$profile_exists" = yes ]; then
    [ "$(command -p stat -c '%a' -- "$profile" 2>/dev/null)" = "$profile_mode" ] &&
      command -p cmp -s -- "$profile_stage/original" "$profile" || fail 'The Bash profile changed during installation.'
  elif [ -e "$profile" ]; then
    fail 'The Bash profile appeared during installation.'
  fi
}

install_asset() {
  asset_name=${1##*/}
  staged_asset=$asset_stage/$asset_name
  (set -C; command -p cat >"$staged_asset") 2>/dev/null || fail 'A bundled Bash file could not be staged.'
  check_file "$staged_asset"
  if [ -e "$1" ] && ! command -p cmp -s -- "$staged_asset" "$1" && [ "$hook" != managed ]; then
    fail 'An existing Bash file differs from the bundle and has no matching managed profile block.'
  fi
}

check_config_directories yes
cfg_id=$(directory_identity "$cfg") || fail 'The Automexia config directory could not be inspected.'
asset_stage=$(command -p mktemp -d -- "$cfg/.automexia-bash.XXXXXXXX" 2>/dev/null) ||
  fail 'The Bash asset staging directory could not be created.'
asset_stage_id=$(directory_identity "$asset_stage") || fail 'The Bash asset staging directory could not be inspected.'
install_bash_assets
assert_profile_unchanged

publish_new_file() {
  command -p mv -nT -- "$1" "$2" 2>/dev/null && [ ! -e "$1" ]
}

# Publish the startup block last so a failed asset installation does not enable
# a partial hook. Skip all byte-identical files so retry is idempotent.
(
  enter_original_directory "$cfg" "$cfg_id" || fail 'The Automexia config directory changed during installation.'
  asset_stage=./${asset_stage##*/}
  owned_directory "$asset_stage" && [ "$(directory_identity "$asset_stage")" = "$asset_stage_id" ] ||
    fail 'The Bash asset staging directory changed during installation.'
  for asset in $assets; do
    target=./$asset
    check_file "$target"
    check_file "$asset_stage/$asset"
    if command -p cmp -s -- "$asset_stage/$asset" "$target"; then continue; fi
    if [ -e "$target" ]; then
      [ "$hook" = managed ] || fail 'A Bash file changed during installation.'
      command -p mv -fT -- "$asset_stage/$asset" "$target" 2>/dev/null || fail 'A bundled Bash file could not be installed.'
    else
      publish_new_file "$asset_stage/$asset" "$target" || fail 'A bundled Bash file could not be created.'
    fi
  done
)

if [ "$hook" = absent ]; then
  (
    enter_original_directory "$home" "$home_id" || fail 'The WSL home directory changed during installation.'
    profile_stage=./${profile_stage##*/}
    owned_directory "$profile_stage" && [ "$(directory_identity "$profile_stage")" = "$profile_stage_id" ] ||
      fail 'The Bash profile staging directory changed during installation.'
    profile=./.bashrc
    assert_profile_unchanged
    check_file "$profile_stage/profile"
    if [ "$profile_exists" = yes ]; then
      command -p mv -fT -- "$profile_stage/profile" "$profile" 2>/dev/null || fail 'The Bash profile could not be updated.'
    else
      publish_new_file "$profile_stage/profile" "$profile" || fail 'The Bash profile could not be created.'
    fi
  )
fi

printf '%s\n' amx-wsl-bash-install-v1
