#!/usr/bin/env bash
set -euo pipefail

# Exercise the production parser and publishers, including distro-neutral names.
python3 - "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)" <<'PY_OS_IDENTITY'
import base64
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import tempfile

root = Path(sys.argv[1])
helper = root / "shell-integration/posix/automexia-os.sh"
with tempfile.TemporaryDirectory(prefix="automexia-os-") as temporary:
    temp = Path(temporary)
    release = temp / "os-release"
    def detect(data, kernel="Linux"):
        release.write_bytes(data)
        result = subprocess.run(["sh", str(helper), str(release), kernel],
                                check=True, capture_output=True, timeout=5)
        return result.stdout.decode().splitlines()
    cases = [
        (b'NAME="Linux Mint"\nID=linuxmint\nVERSION_ID="22.2"\n', ["Linux Mint", "22.2"]),
        (b'NAME="LMDE"\nID=linuxmint\nVERSION_ID=7\n', ["LMDE", "7"]),
        (b'NAME="Debian GNU/Linux"\nID=debian\n', ["Debian GNU/Linux", ""]),
        (b'NAME="Fedora Linux"\n', ["Fedora Linux", ""]),
        (b'NAME=Arch\n', ["Arch", ""]),
        (b'NAME="An Unknown Distribution"\n', ["An Unknown Distribution", ""]),
        (b'PRETTY_NAME="Mint Custom Edition"\n', ["Mint Custom Edition", ""]),
        (b'ID=custom-distro\n', ["custom-distro", ""]),
        (b'NAME=""\nID=alpine\n', ["alpine", ""]),
        (b'NAME=Old\nNAME="New Edition"\n', ["New Edition", ""]),
        (b'NAME="broken\nID=valid-id\n', ["valid-id", ""]),
        (b'NAME="a""b"\n', ["Linux", ""]),
        (b'NAME="bad\x1bname"\n', ["Linux", ""]),
        (b'NAME="' + b'x' * 257 + b'"\n', ["Linux", ""]),
        (b'NAME="Mint"\n' + b'#' * 65536, ["Linux", ""]),
        (b'# empty data\n', ["Linux", ""]),
        ('NAME="发行版"\n'.encode(), ["发行版", ""]),
        (b'NAME="Mint \\"Edition\\""\n', ['Mint "Edition"', ""]),
    ]
    for distro_id, name in [("manjaro", "Manjaro Linux"), ("opensuse-tumbleweed", "openSUSE Tumbleweed"),
                           ("nixos", "NixOS"), ("alpine", "Alpine Linux"), ("pop", "Pop!_OS"),
                           ("rhel", "Red Hat Enterprise Linux"), ("rocky", "Rocky Linux"),
                           ("almalinux", "AlmaLinux"), ("kali", "Kali GNU/Linux")]:
        assert detect(('NAME="' + name + '"\nID=' + distro_id + '\nID_LIKE=ubuntu\n').encode())[0] == name
        assert detect(('ID=' + distro_id + '\n').encode())[0] == distro_id
    for data, expected in cases:
        assert detect(data) == expected, (data[:64], expected, detect(data))
    for kernel, expected in [("Darwin", "macOS"), ("FreeBSD", "FreeBSD"), ("OpenBSD", "OpenBSD"), ("MINGW64_NT", "Windows")]:
        assert detect(b'NAME=Linux\n', kernel) == [expected, ""]
    marker = temp / "must-not-execute"
    payload = '$(touch ' + shlex.quote(str(marker)) + ')'
    assert detect(('NAME="' + payload + '"\n').encode())[0] == payload
    assert not marker.exists(), "os-release data was executed"
    release.unlink()
    assert subprocess.check_output(["sh", str(helper), str(release), "Linux"]).decode().splitlines() == ["Linux", ""]
    os.mkfifo(release)
    assert subprocess.run(["sh", str(helper), str(release), "Linux"], timeout=2, capture_output=True, check=True).stdout == b'Linux\n\n'
    release.unlink()
    release.write_text('NAME="Linux Mint"\nVERSION_ID="22.2"\n')
    for shell, relative, source in [
        ("bash", "bash/automexia.bash", '. "$1"; printf "%s%s" "$__automexia_identity_frame" "$__automexia_identity_frame"'),
        ("zsh", "zsh/automexia.zsh", 'source "$1"; printf "%s%s" "$__automexia_identity_frame" "$__automexia_identity_frame"'),
        ("fish", "fish/automexia.fish", 'source $argv[1]; __automexia_fish_prompt; __automexia_fish_prompt'),
    ]:
        if not shutil.which(shell):
            print("EXTERNAL: OS publisher test needs", shell)
            continue
        for flattened in [False, True]:
            directory = temp / (shell + ("-flat" if flattened else "-tree"))
            script = directory / ("integration." + shell if flattened else relative)
            script.parent.mkdir(parents=True)
            shutil.copyfile(root / "shell-integration" / relative, script)
            adapter = directory / ("automexia-os.sh" if flattened else "posix/automexia-os.sh")
            adapter.parent.mkdir(parents=True, exist_ok=True)
            calls = directory / "calls"
            adapter.write_text('printf x >> ' + shlex.quote(str(calls)) + '\nexec sh ' + shlex.quote(str(helper)) + ' ' + shlex.quote(str(release)) + ' Linux\n')
            env = dict(os.environ, TERM_PROGRAM="Automexia", AUTOMEXIA_SHELL_INTEGRATION="1", AUTOMEXIA_PLAIN_LS="1", AUTOMEXIA_AMX="0")
            env.pop("WSL_DISTRO_NAME", None)
            argv = [shell, "--no-config", "-c", source, str(script)] if shell == "fish" else [shell, "-c", source, "os-test", str(script)]
            # Flattened installs may be sourced by a bare relative filename.
            if flattened:
                argv[-1] = script.name
            result = subprocess.run(argv, cwd=script.parent, env=env, capture_output=True, check=True, timeout=10)
            values = re.findall(rb'SetUserVar=automexia_os_name=([^\x07]*)\x07', result.stdout)
            # Bash line 188 and Zsh line 150 deliberately emit a startup frame,
            # then this probe replays two cached frames. Fish emits only on prompts.
            expected_publications = 2 if shell == 'fish' else 3
            assert len(values) == expected_publications and all(base64.b64decode(v) == b'Linux Mint' for v in values), (shell, flattened, result.stderr, values)
            assert calls.read_text() == 'x', "OS files reread on each prompt"
            assert b'SetUserVar=automexia_distro=\x07' in result.stdout, "native distro leaked into WSL launch identity"
    print("PASS: OS parser fixtures and cached Bash/Zsh/Fish publishers")
PY_OS_IDENTITY

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)

# Exercise Bash's own interactive PROMPT_COMMAND dispatch. Running the hook
# with eval inside a conditional would mask the errexit failure under test.
python3 - "$root" "$BASH" "${BASH_VERSINFO[0]}" "${BASH_VERSINFO[1]}" <<'PY_STRICT_PROMPT'
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import tempfile

root, bash, major, minor = sys.argv[1:]
adapter = shlex.quote(str(Path(root) / 'shell-integration/bash/automexia.bash'))
user_hook = "printf 'user-status=%s\\n' \"$?\""
user_err_trap = "trap 'printf \"user-err=%s\\n\" \"$?\"' ERR"
commands = b'''set -euo pipefail
! :
printf 'strict-negation=%s\\n' "$?"
(exit 7) && :
printf 'strict-and-list=%s\\n' "$?"
set +e
(exit 11)
printf 'normal-failure=%s\\n' "$?"
set -e
command __automexia_test_missing_command
printf 'strict-command-continued\\n'
'''

with tempfile.TemporaryDirectory(prefix='automexia-bash-prompt-') as temporary:
    env = dict(os.environ, HOME=temporary, XDG_CONFIG_HOME=temporary,
               AUTOMEXIA_CONFIG_HOME=temporary, HISTFILE=os.devnull,
               TERM_PROGRAM='Automexia', AUTOMEXIA_SHELL_INTEGRATION='1',
               AUTOMEXIA_PLAIN_LS='1', AUTOMEXIA_AMX='0')
    for inherited in ('BASH_ENV', 'BASHOPTS', 'SHELLOPTS', 'ENV', 'PROMPT_COMMAND'):
        env.pop(inherited, None)
    rcfile = Path(temporary) / 'bashrc'
    prompt_forms = {'string': 'PROMPT_COMMAND=' + shlex.quote(user_hook)}
    if (int(major), int(minor)) >= (5, 1):
        prompt_forms['array'] = (
            'PROMPT_COMMAND=(' + shlex.quote(user_hook) + " 'printf \"second-hook\\n\"')"
        )
    for form, prompt in prompt_forms.items():
        for integrated in [False, True]:
            integration = f'. {adapter}\n' if integrated else ''
            rcfile.write_text(f'{prompt}\n{integration}{user_err_trap}\n')
            setup = f'. {shlex.quote(str(rcfile))}\n'.encode()
            result = subprocess.run(
                [bash, '--noprofile', '--norc', '--noediting', '-i'],
                input=setup + commands, env=env, capture_output=True, timeout=10,
            )
            case = (form, integrated, result.returncode)
            output = result.stdout
            assert result.returncode == 127, case
            for marker in (b'strict-negation=', b'strict-and-list=', b'normal-failure='):
                assert marker in output, (case, marker)
            assert b'strict-command-continued' not in output, case
            assert re.findall(rb'user-err=(\d+)\n', output) == [b'11', b'127'], case
            for status in (1, 7, 11):
                assert f'user-status={status}\n'.encode() in output, (case, status)
                if integrated:
                    assert f'\x1b]133;D;{status}\x07'.encode() in output, (case, status)
            if integrated:
                for marker in (b'strict-negation=1\n', b'strict-and-list=7\n', b'normal-failure=11\n'):
                    assert marker in output, (case, marker)
            if form == 'array':
                assert output.count(b'second-hook\n') == output.count(b'user-status='), case
    print('PASS: interactive Bash preserves conditional failures and native strict-mode exit')
PY_STRICT_PROMPT

# Exercise the real startup notice with a PTY for diagnostics and a pipe for
# listing output. The fixture PATH makes eza availability deterministic.
python3 - "$root" "$BASH" <<'PY_WSL_LISTING_NOTICE'
import errno
import os
from pathlib import Path
import pty
import shutil
import subprocess
import sys
import tempfile

root, bash = sys.argv[1:]
notice = b'Automexia warning: eza is missing. Install it in this WSL distribution for file icons.'
with tempfile.TemporaryDirectory(prefix='automexia-wsl-listing-') as temporary:
    directory = Path(temporary)
    fixture_bin = directory / 'bin'
    fixture_bin.mkdir()
    for program in ('sh', 'uname', 'awk', 'dd', 'base64', 'tr', 'zsh'):
        if source := shutil.which(program):
            (fixture_bin / program).symlink_to(source)
    native_ls = fixture_bin / 'ls'
    native_ls.write_text("#!/bin/sh\nprintf '%s\\n' native-listing\n")
    native_ls.chmod(0o755)
    eza = fixture_bin / 'eza'
    drivers = [('bash', bash, ['--noprofile', '--norc', '--noediting'])]
    if zsh := shutil.which('zsh'):
        drivers.append(('zsh', zsh, ['-f']))
    else:
        print('EXTERNAL: optional-listing notice test needs zsh')
    for shell, executable, flags in drivers:
        adapter = str(Path(root) / 'shell-integration' / shell / f'automexia.{shell}')
        for case, wsl, interactive, tty, opt_out, installed, enabled in [
            ('missing', True, True, True, False, False, True),
            ('installed', True, True, True, False, True, True),
            ('disabled listings', True, True, True, True, False, True),
            ('disabled integration', True, True, True, False, False, False),
            ('non-WSL', False, True, True, False, False, True),
            ('non-interactive', True, False, True, False, False, True),
            ('redirected stderr', True, True, False, False, False, True),
        ]:
            if installed:
                eza.write_text("#!/bin/sh\nprintf 'enhanced %s\\n' \"$*\"\n")
                eza.chmod(0o755)
            else:
                eza.unlink(missing_ok=True)
            env = dict(PATH=str(fixture_bin), HOME=temporary, ZDOTDIR=temporary,
                       AUTOMEXIA_CONFIG_HOME=temporary, XDG_CONFIG_HOME=temporary,
                       TERM='xterm-256color', TERM_PROGRAM='Automexia',
                       AUTOMEXIA_SHELL_INTEGRATION=str(int(enabled)), AUTOMEXIA_AMX='0')
            if wsl:
                env['WSL_DISTRO_NAME'] = 'Fixture-Distro'
            if opt_out:
                env['AUTOMEXIA_PLAIN_LS'] = '1'
            argv = [executable, *flags, *(['-i'] if interactive else []), '-c',
                    '. "$1" >/dev/null; . "$1" >/dev/null; ls | while IFS= read -r row; do printf "%s\\n" "$row"; done',
                    'listing-notice-fixture', adapter]
            if tty:
                master, slave = pty.openpty()
                try:
                    try:
                        result = subprocess.run(argv, env=env, stdin=subprocess.DEVNULL,
                                                stdout=subprocess.PIPE, stderr=slave, check=True, timeout=10)
                    finally:
                        os.close(slave)
                    output = bytearray()
                    while True:
                        try:
                            chunk = os.read(master, 4096)
                        except OSError as error:
                            if error.errno != errno.EIO:
                                raise
                            break
                        if not chunk:
                            break
                        output.extend(chunk)
                finally:
                    os.close(master)
            else:
                result = subprocess.run(argv, env=env, stdin=subprocess.DEVNULL,
                                        capture_output=True, check=True, timeout=10)
                output = result.stderr
            assert output.count(notice) == int(case == 'missing'), (shell, case, output)
            expected = (b'enhanced --icons=auto --color=auto --group-directories-first\n'
                        if installed else b'native-listing\n')
            assert result.stdout == expected, (shell, case, result.stdout)
    print('PASS: WSL listing notice is bounded and preserves pipeline output')
PY_WSL_LISTING_NOTICE

# The diagnostic reads the managed block without executing shell startup code.
python3 - "$root" <<'PY_WSL_SETUP_REPORT'
from pathlib import Path
import os
import shlex
import subprocess
import sys
import tempfile

probe = (Path(sys.argv[1]) / 'apps/automexia-terminal/src/automexia/wsl/setup_probe.sh').read_bytes()
with tempfile.TemporaryDirectory(prefix='automexia-wsl-setup-') as temporary:
    home = Path(temporary)
    tools = home / 'bin'
    tools.mkdir()
    hook_dir = home / '.config' / 'automexia'
    hook_dir.mkdir(parents=True)
    env = dict(HOME=temporary, SHELL='/bin/bash', PATH=str(tools),
               WSL_DISTRO_NAME='Fixture-Distro')

    def check(hook, listing, *, script=probe, installer=None, **overrides):
        result = subprocess.run(['/bin/sh', '-s'], input=script,
                                env=env | overrides, capture_output=True, check=True, timeout=5)
        *payload, capability, uid = result.stdout.decode().splitlines()
        assert payload == ['amx-wsl-setup-v2', 'Fixture-Distro', 'bash', hook, listing], payload
        expected_uid = str(os.getuid()) if os.getuid() and overrides.get('AUTOMEXIA_SHELL_INTEGRATION') != '0' else '-'
        assert uid == expected_uid, (uid, expected_uid)
        assert capability in {'apt', 'unavailable'} and result.stderr == b'', result
        if installer is not None:
            assert capability == installer, result.stdout

    check('missing', 'missing')
    (hook_dir / 'shell-integration.bash').write_text('exit 99\n')
    check('adapter-only', 'missing')
    source = '${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}/shell-integration.bash'
    (home / '.bashrc').write_text(
        'exit 99\n# >>> AUTOMEXIA SHELL INTEGRATION >>>\n'
        f'[ -r "{source}" ] && . "{source}"\n'
        '# <<< AUTOMEXIA SHELL INTEGRATION <<<\n'
    )
    check('managed', 'missing')
    check('managed', 'disabled', AUTOMEXIA_PLAIN_LS='1')
    check('managed', 'disabled', AUTOMEXIA_SHELL_INTEGRATION='0')
    (tools / 'eza').symlink_to('/bin/sh')
    check('managed', 'present')

    # Redirect only the system paths in the fixture. None of the stand-in
    # programs should run. Ubuntu's sudo path traverses an alternatives link.
    sudo = home / 'sudo'
    alternative = home / 'sudo-alternative'
    sudo_target = home / 'sudo-rs'
    apt_get = home / 'apt-get'
    release = home / 'os-release'
    for executable in [sudo_target, apt_get]:
        executable.write_text('#!/bin/sh\nexit 99\n')
        executable.chmod(0o755)
    alternative.symlink_to(sudo_target.name)
    sudo.symlink_to(alternative.name)
    fixture = probe
    for system_path, path in [(b'/usr/bin/sudo', sudo), (b'/usr/bin/apt-get', apt_get),
                              (b'/etc/os-release', release)]:
        fixture = fixture.replace(system_path, shlex.quote(str(path)).encode())
    release.write_text('ID=ubuntu\n')
    check('managed', 'present', script=fixture, installer='apt')
    release.write_text('ID=fedora\n')
    check('managed', 'present', script=fixture, installer='unavailable')
    release.write_text('ID=ubuntu\n')
    sudo_target.unlink()
    check('managed', 'present', script=fixture, installer='unavailable')
print('PASS: WSL setup probe reports prerequisites and accepts valid APT alternatives links')
PY_WSL_SETUP_REPORT

# Exercise the guest writer against disposable profiles using the bundled bytes.
if [[ $(uname -s) == Linux ]]; then
python3 - "$root" <<'PY_WSL_BASH_INSTALL'
import os
from pathlib import Path
import subprocess
import sys
import tempfile

root = Path(sys.argv[1])
installer = (root / 'apps/automexia-terminal/src/automexia/wsl/install_bash.sh').read_bytes()
bundle = root / 'shell-integration'
assets = {
    'shell-integration.bash': (bundle / 'bash/automexia.bash').read_bytes(),
    'automexia-completion.bash': (bundle / 'completion/bash/automexia-completion.bash').read_bytes(),
    'automexia-os.sh': (bundle / 'posix/automexia-os.sh').read_bytes(),
    'automexia-eza-filter.pl': (bundle / 'posix/automexia-eza-filter.pl').read_bytes(),
}
payload = b'install_bash_assets() {\n' + b''.join(
    f'install_asset "$cfg/{name}" <<\'AMX_FIXTURE_ASSET\'\n'.encode()
    + content + b'AMX_FIXTURE_ASSET\n' for name, content in assets.items()
) + b'}\n' + installer
uid = os.geteuid() or 65534
child_account = {'user': uid, 'group': uid, 'extra_groups': []} if os.geteuid() == 0 else {}

with tempfile.TemporaryDirectory(prefix='automexia-wsl-bash-') as temporary:
    home = Path(temporary).resolve()
    if child_account:
        os.chown(home, uid, uid)
    rc = home / '.bashrc'
    original = b'# keep user startup bytes\nprintf custom-command'
    rc.write_bytes(original)
    rc.chmod(0o640)
    if child_account:
        os.chown(rc, uid, uid)
    env = dict(HOME=str(home), SHELL='/bin/bash', WSL_DISTRO_NAME='Fixture')

    def install(distro='Fixture', account=uid, **overrides):
        return subprocess.run(['/bin/sh', '-s', '--', distro, str(account)],
                              input=payload, env=env | overrides, capture_output=True,
                              timeout=10, **child_account)

    assert install('Elsewhere').returncode != 0
    assert install(account=uid + 1).returncode != 0
    assert install(AUTOMEXIA_SHELL_INTEGRATION='0').returncode != 0
    assert rc.read_bytes() == original and not (home / '.config').exists()

    result = install()
    assert result.returncode == 0 and result.stdout == b'amx-wsl-bash-install-v1\n', result
    cfg = home / '.config/automexia'
    assert {file.name: file.read_bytes() for file in cfg.iterdir()} == assets
    installed = rc.read_bytes()
    assert installed.startswith(original + b'\n')
    assert installed.count(b'# >>> AUTOMEXIA SHELL INTEGRATION >>>\n') == 1
    assert rc.stat().st_mode & 0o777 == 0o640
    before = {path: path.stat().st_ino for path in [rc, *cfg.iterdir()]}
    assert install().returncode == 0 and rc.read_bytes() == installed
    assert before == {path: path.stat().st_ino for path in before}

    rc.write_bytes(original + b'\n# >>> AUTOMEXIA SHELL INTEGRATION >>>\n')
    assert install().returncode != 0
    assert rc.read_bytes() == original + b'\n# >>> AUTOMEXIA SHELL INTEGRATION >>>\n'
    rc.unlink()
    sibling = home / '.zshrc'
    sibling.write_bytes(b'custom-zsh-content\n')
    rc.symlink_to(sibling)
    assert install().returncode != 0 and sibling.read_bytes() == b'custom-zsh-content\n'
    assert not list(home.glob('.automexia-bash.*'))
print('PASS: WSL Bash installation requires confirmed consent scope and preserves profile contents')
PY_WSL_BASH_INSTALL
fi

export TERM_PROGRAM=Automexia
PROMPT_COMMAND='printf user-hook >/dev/null'

# Use a controlled eza stand-in so this test proves alias behavior without
# making the contributor machine install an optional presentation tool.
fixture_bin=$(mktemp -d)
trap 'rm -rf "$fixture_bin"' EXIT
export AUTOMEXIA_CONFIG_HOME="$fixture_bin/config"
completion_root="$AUTOMEXIA_CONFIG_HOME/generated/completion/bash"
mkdir -p "$completion_root"
printf '%s\n' 'complete -W "managed-candidate" kubectl' >"$completion_root/kubectl.bash"
kubectl_digest=$(sha256sum "$completion_root/kubectl.bash" | awk '{print $1}')
# Simulate an interrupted refresh: the candidate digest may be the second entry.
printf '%064d\n%s\n' 0 "$kubectl_digest" >"$completion_root/kubectl.bash.sha256"
printf '%s\n' 'complete -W "must-not-load" docker' >"$completion_root/docker.bash"
sha256sum "$completion_root/docker.bash" | awk '{print $1}' >"$completion_root/docker.bash.sha256"
complete -W 'native-candidate' docker
cat >"$fixture_bin/eza" <<'EOF'
#!/usr/bin/env sh
printf '%s\n' "$*"
EOF
chmod +x "$fixture_bin/eza"
PATH="$fixture_bin:$PATH"
alias_generation=$(
  python3 "$root/tools/ci/create_cp31_alias_fixture.py" \
    --config-root "$AUTOMEXIA_CONFIG_HOME" --alias axt --value first
)
[[ $alias_generation =~ ^[0-9a-f]{64}$ ]]
shopt -s expand_aliases
# Reproduce Ubuntu's default interactive profile. Defining a same-named shell
# function while this alias is active used to make Bash reject the integration.
alias ls='ls --color=auto'

# shellcheck source=/dev/null
source "$root/shell-integration/bash/automexia.bash" >/dev/null
# shellcheck source=/dev/null
source "$root/shell-integration/bash/automexia.bash" >/dev/null

count=$(grep -o '__automexia_pre_prompt' <<<"$PROMPT_COMMAND" | wc -l)
[[ $count -eq 1 ]]
[[ ${PROMPT_COMMAND#'__automexia_legacy_install && :;'} == '__automexia_capture_status && :;printf user-hook'* ]]
[[ $(grep -o __automexia_capture_status <<<"$PROMPT_COMMAND" | wc -l) -eq 1 ]]

failure_marker="$fixture_bin/bash-failure-marker"
set +e
false
__automexia_pre_prompt >"$failure_marker"
status=$?
set -e
[[ $status -eq 1 ]]
grep -qF $'\e]133;D;1\a' "$failure_marker"
grep -qF $'\e]133;A;aid=' "$failure_marker"
# Run the complete prompt chain: a successful user hook must not erase the
# failed command result. The capture also returns that status to the hook.
set +e
false
eval "$PROMPT_COMMAND" >"$failure_marker"
status=$?
set -e
[[ $status -eq 1 ]]
grep -qF $'\e]133;D;1\a' "$failure_marker"

grep -qF "AUTOMEXIA_SHELL_INTEGRATION" "$root/shell-integration/bash/automexia.bash"
grep -qF '\xCE\xBB' "$root/shell-integration/bash/automexia.bash"
[[ $PS1 == *$'\u03BB'* ]]
[[ $PS1 != *'\w'* ]]
# Leave native explicit syntax colors authoritative; plain input starts in the
# default foreground so the shared command/option accent can be displayed.
[[ $PS1 == *'\[\e]133;B\a\]' ]]
if grep -qF 'PROMPT_DIRTRIM' "$root/shell-integration/bash/automexia.bash"; then exit 1; fi
if grep -qF '__automexia_git_segment' "$root/shell-integration/bash/automexia.bash"; then exit 1; fi
grep -qF '133;A;aid=%s\a \n' "$root/shell-integration/bash/automexia.bash"
grep -qF '133;P;k=c;aid=%s\a' "$root/shell-integration/bash/automexia.bash"
grep -qF '133;A;aid=%s\a' "$root/shell-integration/fish/automexia.fish"
grep -qF 'SetUserVar=automexia_prompt_active=MQ==\a' "$root/shell-integration/fish/automexia.fish"
grep -qF '133;A;aid=%s\a \n' "$root/shell-integration/fish/automexia.fish"
grep -qF '133;P;k=c;aid=%s\a' "$root/shell-integration/fish/automexia.fish"
grep -qF 'SetUserVar=automexia_prompt_active=MA==\a' "$root/shell-integration/fish/automexia.fish"
grep -qF 'fish_preexec' "$root/shell-integration/fish/automexia.fish"
grep -qF '133;C\a' "$root/shell-integration/fish/automexia.fish"
grep -qF 'fish_postexec' "$root/shell-integration/fish/automexia.fish"
grep -qF 'fish_posterror' "$root/shell-integration/fish/automexia.fish"
grep -qF '133;D;%s\a' "$root/shell-integration/fish/automexia.fish"
# shellcheck disable=SC2016 # Search for the literal integration contract.
grep -qF '__automexia_print_colored_path "$PWD"' "$root/shell-integration/bash/automexia.bash"
[[ $PS1 != *'PWD'* ]]
grep -qF '__automexia_prompt_is_active=0' "$root/shell-integration/bash/automexia.bash"
if grep -Eiq 'alias (docker|kubectl)=|function (ax|kgp)' "$root/shell-integration/bash/automexia.bash"; then exit 1; fi
sample_path='/srv/cloud project/region/production'
expected_path=$'\e[38;2;88;113;141m/\e[38;2;98;176;255msrv\e[38;2;88;113;141m/\e[38;2;80;213;255mcloud project\e[38;2;88;113;141m/\e[38;2;167;139;250mregion\e[38;2;88;113;141m/\e[38;2;184;243;107mproduction\e[0m'
[[ $(__automexia_print_colored_path "$sample_path") == "$expected_path" ]]
[[ $(type -t ls) == function ]]
[[ $(type -t ll) == function ]]
[[ $(ls -ll) == '--icons=auto --color=auto --group-directories-first --header --group --time-style=long-iso -ll' ]]
[[ $(ll) == '--icons=auto --color=auto --group-directories-first --header --group --time-style=long-iso -lah --git' ]]
[[ $EZA_COLORS == reset:* ]]
[[ $EZA_COLORS == *'ur=38;5;81'* ]]
[[ $EZA_COLORS == *'uw=38;5;220'* ]]
[[ $EZA_COLORS == *'ux=38;5;114'* ]]
[[ $EZA_COLORS == *'hd=1;38;5;117'* ]]
[[ $EZA_COLORS == *'di=1;38;5;39'* ]]
[[ $EZA_COLORS == *'ex=38;5;252'* ]]
complete -p kubectl | grep -qF 'managed-candidate'
complete -p docker | grep -qF 'native-candidate'
! complete -p docker | grep -qF 'must-not-load'
automexia_completion_health | grep -qF 'loaded=kubectl'
automexia_completion_health | grep -qF 'collisions=docker'
automexia_aliases_health | grep -qF 'state=ready'
[[ $(axt) == first ]]

performance_failed=0
alias_samples=''
for iteration in {1..25}; do
  sample=$(
    TIMEFORMAT='%R'
    { time automexia_aliases_reload >/dev/null; } 2>&1
  )
  (( iteration > 5 )) && alias_samples+="$sample"$'\n'
done
bash_alias_reload_p95=$(printf '%s' "$alias_samples" | sort -n | sed -n '19p')
printf 'SHELL_BENCHMARK shell=bash operation=alias-reload p95=%s unit=s\n' "$bash_alias_reload_p95"
awk -v p95="$bash_alias_reload_p95" 'BEGIN { if (p95 !~ /^[0-9]+([.][0-9]+)?$/ || p95 > 0.050) { print "FAIL: Bash alias reload p95 invalid or exceeds 0.050s: " p95 > "/dev/stderr"; exit 1 } }' || performance_failed=1

second_alias_generation=$(
  python3 "$root/tools/ci/create_cp31_alias_fixture.py" \
    --config-root "$AUTOMEXIA_CONFIG_HOME" --alias ayt --value second
)
[[ $second_alias_generation != "$alias_generation" ]]
ayt() { printf '%s' native; }
if automexia_aliases_reload; then exit 1; fi
[[ $(axt) == first ]]
[[ $(ayt) == native ]]
automexia_aliases_health | grep -qF 'state=collision'
unset -f ayt
automexia_aliases_reload
[[ -z $(type -t axt 2>/dev/null || true) ]]
[[ $(ayt) == second ]]
wrong_compiler_generation=$(
  python3 "$root/tools/ci/create_cp31_alias_fixture.py" \
    --config-root "$AUTOMEXIA_CONFIG_HOME" --alias azt --value wrong \
    --generator automexia-devops/999.0.0
)
[[ $wrong_compiler_generation != "$second_alias_generation" ]]
if automexia_aliases_reload; then exit 1; fi
[[ $(ayt) == second ]]
[[ -z $(type -t azt 2>/dev/null || true) ]]
automexia_aliases_health | grep -qF 'state=tampered'
printf '%s\n' "$second_alias_generation" >"$AUTOMEXIA_CONFIG_HOME/generated/aliases/current"
chmod 600 "$AUTOMEXIA_CONFIG_HOME/generated/aliases/current"

current_alias_generation=$(<"$AUTOMEXIA_CONFIG_HOME/generated/aliases/current")
printf '%s\n' '# tampered' >>"$AUTOMEXIA_CONFIG_HOME/generated/aliases/generations/$current_alias_generation/bash/automexia-aliases.bash"
if automexia_aliases_reload; then exit 1; fi
[[ $(ayt) == second ]]
automexia_aliases_health | grep -qF 'state=tampered'
printf '%s\n' disabled >"$AUTOMEXIA_CONFIG_HOME/generated/aliases/current"
chmod 600 "$AUTOMEXIA_CONFIG_HOME/generated/aliases/current"
AUTOMEXIA_TEST_ROOT="$root" bash --noprofile --norc -c '
  shopt -s expand_aliases
  source "$AUTOMEXIA_TEST_ROOT/shell-integration/bash/automexia.bash" >/dev/null
  [[ -z $(type -t ayt 2>/dev/null || true) ]]
  automexia_aliases_health | grep -qF "state=disabled"
'

for category_color in \
  '*secret=1;38;5;203' '*config=38;5;214' '*logs=38;5;220' \
  '*src=38;5;81' '*docs=38;5;114' '*tests=38;5;177' \
  '*target=38;5;209' '*assets=38;5;211' '*data=38;5;105' \
  '*cache=38;5;245' '*infra=38;5;39' '*packaging=38;5;214'; do
  [[ $EZA_COLORS == *"$category_color"* ]]
done

badge_filter="$root/shell-integration/posix/automexia-eza-filter.pl"
[[ -r $badge_filter ]]
badge_input=$'\e[38;5;39m\uE5FF \e[1mconfig\e[0m  \e[38;5;39m\uE5FF \e[1mrio-vt\e[0m  \e[38;5;39m\uE5FF \e[1mordinary\e[0m'
badge_output=$(printf '%s\n' "$badge_input" | perl -CS "$badge_filter")
[[ $badge_output == *$'\U000F107F \e[1mconfig'* ]]
[[ $badge_output == *$'\U000F19F6 \e[1mrio-vt'* ]]
[[ $badge_output == *$'\uE5FF \e[1mordinary'* ]]
[[ $badge_output == *$'\e[38;2;255;176;32m'* ]]
[[ $badge_output == *$'\e[38;2;80;213;255m'* ]]

unset -f ls l ll la lA tree __automexia_eza __automexia_run_eza
export AUTOMEXIA_PLAIN_LS=1
# shellcheck source=/dev/null
source "$root/shell-integration/bash/automexia.bash" >/dev/null
[[ $(type -t ls) != function ]]

unset AUTOMEXIA_COMPLETION_ADAPTER_BASH_LOADED
export AUTOMEXIA_COMPLETION_DISABLED=1
complete -r kubectl
source "$root/shell-integration/completion/bash/automexia-completion.bash"
! complete -p kubectl >/dev/null 2>&1
automexia_completion_health | grep -qF 'state=disabled'

adapter_samples=''
for iteration in {1..25}; do
  sample=$(
    unset AUTOMEXIA_COMPLETION_ADAPTER_BASH_LOADED AUTOMEXIA_COMPLETION_DISABLED
    complete -r kubectl 2>/dev/null || true
    TIMEFORMAT='%R'
    { time source "$root/shell-integration/completion/bash/automexia-completion.bash" >/dev/null; } 2>&1
  )
  (( iteration > 5 )) && adapter_samples+="$sample"$'\n'
done
bash_adapter_p95=$(printf '%s' "$adapter_samples" | sort -n | sed -n '19p')
printf 'SHELL_BENCHMARK shell=bash operation=adapter p95=%s unit=s\n' "$bash_adapter_p95"
awk -v p95="$bash_adapter_p95" 'BEGIN { if (p95 !~ /^[0-9]+([.][0-9]+)?$/ || p95 > 0.050) { print "FAIL: Bash completion adapter p95 invalid or exceeds 0.050s: " p95 > "/dev/stderr"; exit 1 } }' || performance_failed=1

unset AUTOMEXIA_COMPLETION_ADAPTER_BASH_LOADED AUTOMEXIA_COMPLETION_DISABLED
complete -r kubectl 2>/dev/null || true
unsafe_completion="$fixture_bin/unsafe-bash"
mkdir -p "$unsafe_completion"
printf '%s\n' 'complete -W "linked-candidate" kubectl' >"$unsafe_completion/kubectl.bash"
sha256sum "$unsafe_completion/kubectl.bash" | awk '{print $1}' >"$unsafe_completion/kubectl.bash.sha256"
rm -rf -- "$completion_root"
ln -s "$unsafe_completion" "$completion_root"
source "$root/shell-integration/completion/bash/automexia-completion.bash"
! complete -p kubectl >/dev/null 2>&1
automexia_completion_health | grep -qF 'state=unsafe-path/native-fallback'

unset AUTOMEXIA_COMPLETION_ADAPTER_BASH_LOADED
export AUTOMEXIA_CONFIG_HOME=relative-config-root
source "$root/shell-integration/completion/bash/automexia-completion.bash"
automexia_completion_health | grep -qF 'state=unsafe-path/native-fallback'

# The declared path ceiling must fail before any filesystem probe.
unset AUTOMEXIA_COMPLETION_ADAPTER_BASH_LOADED
printf -v overlong_root '%*s' 4097 ''
overlong_root=${overlong_root// /x}
export AUTOMEXIA_CONFIG_HOME="/$overlong_root"
source "$root/shell-integration/completion/bash/automexia-completion.bash"
automexia_completion_health | grep -qF 'state=unsafe-path/native-fallback'

python3 "$root/tools/ci/test_shell_artifact_validation.py" --shell bash
echo 'PASS: Bash behavioral assertions completed'
if (( performance_failed )); then exit 1; fi
echo "PASS: Bash integration is prompt-safe, native-first, digest-verified, linked-parent-safe, disable-safe, idempotent, adapter-p95=${bash_adapter_p95}s, alias-reload-p95=${bash_alias_reload_p95}s, and readable icon-listing aware"
