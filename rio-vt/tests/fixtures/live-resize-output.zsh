set -eu
stty -echo
trap 'stty echo' EXIT
printf '\033]133;A;aid=1\007\r\n\033]133;P;k=c;aid=1\007/example\r\n\033]133;P;k=c;aid=1\007lambda \033]133;B\007list\r\n\033]133;C\007'
if [[ ${1:-} == wide ]]; then
    for index in {1..32}; do
        printf 'ROW-%02d  -a---  2026-01-01 12:00:00  %04d  artifact-%02d-abcdefghijklmnopqrstuvwxyz0123456789.txt\r\n' "$index" "$index" "$index"
    done
else
    for index in {1..8}; do printf 'ROW-%02d  retained output\r\n' "$index"; done
fi
printf '\033]133;D;0\007\033]133;A;aid=2\007\r\n\033]133;P;k=c;aid=2\007/example\r\n\033]133;P;k=c;aid=2\007lambda \033]133;B\007'
printf '\033]2;RESIZE-READY\007'
# ZLE uses the Unix PTY resize contract; this probe emits no terminal rows.
for step in {0..11}; do
    read -rk1 probe
    printf '\033]2;RESIZE-ACK-%d\007' "$step"
done
read -rk1 probe
