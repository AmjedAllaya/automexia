amx_dir=/tmp/automexia-ssh.@@NONCE@@
(umask 077; command mkdir -- "$amx_dir") 2>/dev/null || exit 125
amx_cleanup() {
    command rm -f -- "$amx_dir/helper" "$amx_dir/describe" "$amx_dir/probe-status" 2>/dev/null
    command rmdir -- "$amx_dir" 2>/dev/null || :
}
trap amx_cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
(umask 077; set -C; command head -c @@LIMIT@@ > "$amx_dir/helper") 2>/dev/null || exit 125
[ "$(command wc -c < "$amx_dir/helper")" -eq @@SIZE@@ ] || exit 126
if command -v sha256sum >/dev/null 2>&1; then
    amx_hash=$(command sha256sum < "$amx_dir/helper") || exit 126
elif command -v shasum >/dev/null 2>&1; then
    amx_hash=$(command shasum -a 256 < "$amx_dir/helper") || exit 126
else
    exit 125
fi
[ "${amx_hash%% *}" = @@SHA256@@ ] || exit 126
command chmod 700 -- "$amx_dir/helper" 2>/dev/null || exit 125
# Bash monitor mode gives this one asynchronous job its own process group,
# even without a terminal. Both completion and the five-second watchdog kill
# their current group, whose identity stays pinned by the signalling process.
# No PID read from a file or already-reaped child is ever used as kill authority.
set -m || exit 125
case $- in *m*) ;; *) exit 125 ;; esac
{
    (
        set +m
        trap - EXIT
        trap '' HUP INT TERM
        (command sleep 5; builtin kill -KILL 0) &
        if (ulimit -f 1; exec "$amx_dir/helper" --describe-v1 < /dev/null > "$amx_dir/describe"); then
            printf '0\n' > "$amx_dir/probe-status"
        fi
        builtin kill -KILL 0
    ) >/dev/null 2>&1 &
    amx_probe=$!
    wait "$amx_probe" || :
} 2>/dev/null
set +m
[ "$(command cat -- "$amx_dir/probe-status" 2>/dev/null)" = 0 ] || exit 126
[ "$(command wc -c < "$amx_dir/describe")" -eq 14 ] || exit 126
[ "$(command cat -- "$amx_dir/describe")" = AMXSSHHELPER1 ] || exit 126
command rm -f -- "$amx_dir/describe" "$amx_dir/probe-status" 2>/dev/null || exit 125
amx_encoded=$(printf '%s' "$amx_dir" | command base64 | command tr -d '\r\n') || exit 125
[ -n "$amx_encoded" ] || exit 125
printf 'AMXSSHUPLOAD1|@@PANE@@|@@GENERATION@@|@@NONCE@@|@@SIZE@@|@@SHA256@@|%s\n' "$amx_encoded" || exit 125
trap - EXIT HUP INT TERM
exit 0
