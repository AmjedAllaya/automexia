#!/usr/bin/env python3
"""Native Linux/macOS launch, PTY, features and UI regression scenarios.

Linux runs under isolated Xvfb (CI pins lavapipe). macOS reuses the owned AX/Quartz
driver and requires preauthorized Accessibility access. macOS pixels are checked
by the separate controlled-raster suite; real display scaling is recorded, not forced.
Only synthetic commands run in temporary homes. Reports exclude terminal text.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import signal
import shutil
import sys
import subprocess
import tempfile
import time


MACOS = sys.platform == "darwin"
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "tools/renderer-benchmarks/application"))
from macos_driver import MacDriver
from benchmark_model import BenchmarkError


class Failure(RuntimeError):
    pass


class Cancelled(BaseException):
    """Abort the matrix while still unwinding the owned child cleanup."""


def cancel(signum, frame):
    raise Cancelled()


def call(*args: str) -> str:
    return subprocess.run(args, check=True, capture_output=True, text=True,
                          timeout=10).stdout.strip()


def active(state: dict) -> dict:
    return next((panel for panel in state.get("panels", []) if panel.get("active")), {})


def check(condition: bool, message: str) -> None:
    if not condition:
        raise Failure(message)


def pid_exists(pid: int) -> bool:
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False


def run_case(binary: Path, captures: Path, backend: str, shell: str, scale: float | None, launch_mode: str) -> dict:
    label = f"{backend}-{shell}-{scale if scale is not None else 'native'}-{launch_mode}"
    output = captures / label
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="automexia-unix-native-") as temporary:
        home = Path(temporary)
        config = home / "config"
        config.mkdir()
        (home / ".bashrc").write_text("export AMX_NATIVE_RC=RC_LOADED\n", encoding="utf-8")
        # Isolate fixtures from the host's Debian completion trust prompt. This
        # test-only startup choice never alters the application bootstrap.
        (home / ".zshenv").write_text("skip_global_compinit=1\nexport AMX_NATIVE_ZSHENV=ENV_LOADED\n", encoding="utf-8")
        (home / ".zshrc").write_text("export AMX_NATIVE_RC=RC_LOADED\n", encoding="utf-8")
        (home / ".bash_profile").write_text("export AMX_NATIVE_RC=RC_LOADED\n", encoding="utf-8")
        fish = home / ".config/fish"
        fish.mkdir(parents=True)
        (fish / "config.fish").write_text("set -gx AMX_NATIVE_RC RC_LOADED\n", encoding="utf-8")
        renderer = "use-cpu = true" if backend == "cpu" else f"backend = {json.dumps(backend)}"
        program = ("/bin/zsh" if MACOS else "/bin/bash") if shell == "default" else (shutil.which("fish") if MACOS and shell == "fish" else "/bin/" + shell)
        check(program is not None, "required shell missing")
        shell_config = "" if shell == "default" else f"[shell]\nprogram = {json.dumps(program)}\nargs = {json.dumps(['--login'] if MACOS else [])}\n"
        # Exercise the platform default and, on macOS, optional fork mode.
        # Profile env and CWD must reach the same validated launch owner.
        fork_config = "use-fork = true\n" if launch_mode == "fork" else ""
        (config / "config.toml").write_text(
            'confirm-before-quit = false\n'
            'env-vars = ["AMX_NATIVE_PROFILE=PROFILE_LOADED"]\n'
            + fork_config + shell_config +
            f'[renderer]\n{renderer}\n[cursor]\nblinking = false\n'
            '[session-recovery]\nenabled = false\n', encoding="utf-8")
        control, snapshot = home / "control", home / "snapshot.json"
        control.write_text("", encoding="utf-8")
        env = dict(os.environ, HOME=str(home), XDG_CONFIG_HOME=str(home / ".config"),
                   XDG_DATA_HOME=str(home / ".local/share"), XDG_CACHE_HOME=str(home / ".cache"),
                   AUTOMEXIA_CONFIG_HOME=str(config), AUTOMEXIA_NATIVE_TEST_CONTROL=str(control),
                   AUTOMEXIA_RESIZE_SNAPSHOT=str(snapshot), WINIT_UNIX_BACKEND="x11",
                   WINIT_X11_SCALE_FACTOR=str(scale or 1), SHELL=program, HISTFILE="/dev/null", RUST_LOG="error",
                   AUTOMEXIA_SHELL_INTEGRATION="1", AUTOMEXIA_CONTEXT_PATH_HINTS="0",
                   PATH="/usr/local/bin:/usr/bin:/bin")
        for key in ("WAYLAND_DISPLAY", "ZDOTDIR", "AUTOMEXIA_ORIGINAL_ZDOTDIR", "AUTOMEXIA_ORIGINAL_ZDOTDIR_SET",
                    "AUTOMEXIA_VISUAL_TEST_FIXTURE"):
            env.pop(key, None)
        if MACOS:
            env.pop("WINIT_UNIX_BACKEND", None)
            env.pop("WINIT_X11_SCALE_FACTOR", None)
            # login(1) owns HOME/USER; isolate Zsh startup through ZDOTDIR.
            env["ZDOTDIR"] = str(home)
        else:
            env["USER"] = "fixture"
        # No unredacted native logs or snapshots are published.
        with (home / "stderr.log").open("wb") as log:
            started = time.monotonic()
            command_samples = []
            process = subprocess.Popen([str(binary), "--working-dir", str(home)],
                                       cwd=home, env=env, stdout=subprocess.DEVNULL,
                                       stderr=log, start_new_session=True)
            handle = None
            driver = None
            sequence = 0
            shell_pids: set[int] = set()
            try:
                def state() -> dict:
                    check(process.poll() is None, "application exited")
                    try:
                        value = json.loads(snapshot.read_text(encoding="utf-8"))
                    except (FileNotFoundError, json.JSONDecodeError):
                        return {}
                    for panel in value.get("panels", []):
                        if panel.get("shell_pid"):
                            shell_pids.add(panel["shell_pid"])
                    return value

                def wait(predicate, message: str, seconds: float = 15) -> dict:
                    end = time.monotonic() + seconds
                    while time.monotonic() < end:
                        value = state()
                        if value and predicate(value):
                            return value
                        time.sleep(.05)
                    raise Failure(message)

                wait(lambda s: s.get("sequence", 0) > 0, "no presented frame", 30)
                if MACOS:
                    driver = MacDriver(process)
                    driver.resize(1200, 800)
                else:
                    end = time.monotonic() + 10
                    while time.monotonic() < end and handle is None:
                        found = subprocess.run(["xdotool", "search", "--pid", str(process.pid)],
                                               capture_output=True, text=True, timeout=5).stdout.split()
                        if len(found) == 1:
                            handle = found[0]
                        else:
                            time.sleep(.1)
                    check(handle is not None, "no unique owned X11 window")
                    call("xdotool", "windowsize", handle, "1200", "800")
                    call("xdotool", "windowfocus", handle)

                def key(chord: str) -> None:
                    if driver:
                        driver.key(chord.replace("ctrl+", "meta+"))
                    else:
                        check(call("xdotool", "getwindowfocus") == handle, "owned window lost focus")
                        call("xdotool", "key", "--clearmodifiers", chord)

                def send(action: str) -> dict:
                    nonlocal sequence
                    sequence += 1
                    name, _, payload = action.partition(":")
                    command = f"{name}:{sequence}" + (f":{payload}" if payload else "")
                    pending = control.with_suffix(".next")
                    pending.write_text(command, encoding="utf-8")
                    pending.replace(control)
                    return wait(lambda s: s.get("last_control") == command, "control not presented")

                def capture(name: str) -> None:
                    # Native pixels, separate from the semantic assertions.
                    if not MACOS:
                        call("import", "-window", handle, str(output / f"{name}.png"))

                def command(text: str) -> dict:
                    previous = wait(lambda s: s.get("prompt_active"), "shell not ready")
                    prior_id = previous.get("latest_prompt_id")
                    command_start = time.monotonic()
                    send("write-line:" + text)
                    value = wait(lambda s: s.get("prompt_active") and s.get("latest_prompt_id") != prior_id,
                                "command did not complete at a fresh prompt")
                    command_samples.append(round((time.monotonic() - command_start) * 1000, 3))
                    return value

                initial = wait(lambda s: active(s).get("shell_integration") and
                               s.get("latest_prompt_start_count") == 1, "session integration absent")
                ready_ms = round((time.monotonic() - started) * 1000, 3)
                expected = "wgpu" if backend == "webgpu" else backend
                check(initial.get("renderer_backend") == expected, "unexpected renderer fallback")
                if scale is not None:
                    check(abs(initial.get("scale_factor", 0) - scale) < .01, "scale not applied")
                check(initial.get("scale_factor", 0) > 0, "invalid native scale")
                check(active(initial).get("current_directory") == str(home), "launch CWD discarded")
                result = command("printenv AMX_NATIVE_RC; printenv AMX_NATIVE_PROFILE")
                text = active(result).get("visible_text", "")
                check("RC_LOADED" in text and "PROFILE_LOADED" in text, "startup/profile environment lost")
                result = command("false")
                check(any(row.get("result_exit_code") == 1 for row in result.get("semantic_rows", [])), "failed command status lost")
                result = command("printf 'NAME  STATUS   AGE\\napi   Running  2d\\nweb   Pending  1d\\n'")
                check(result.get("inline_table_count", 0) >= 1, "inline table not rendered")
                check(bool(result.get("prompt_context_paints")), "information tags not painted")
                capture("terminal")

                key("ctrl+shift+p")
                wait(lambda s: s.get("palette_enabled"), "keyboard palette failed")
                capture("palette")
                key("Escape")
                wait(lambda s: not s.get("palette_enabled"), "palette did not close")
                send("open-customizations")
                wait(lambda s: s.get("settings", {}).get("ready"), "settings not ready")
                capture("customizations")
                key("ctrl+shift+p")
                check(not (state().get("palette_enabled") and state().get("settings", {}).get("open")),
                      "two overlapping root menus")
                key("Escape")
                wait(lambda s: not s.get("settings", {}).get("open"), "settings did not close")
                send("open-themes")
                wait(lambda s: bool(s.get("settings", {}).get("gallery")), "theme gallery missing")
                capture("themes")
                key("Escape")
                restored = wait(lambda s: not s.get("settings", {}).get("gallery"), "gallery did not close")
                if restored.get("settings", {}).get("open"):
                    key("Escape")
                    wait(lambda s: not s.get("settings", {}).get("open"), "gallery parent did not close")

                send("clone-right")
                split = wait(lambda s: s.get("panel_count") == 2 and active(s).get("shell_integration"),
                             "native pane clone failed")
                check(active(split).get("launch_program") == (None if MACOS and shell == "default" else program), "clone changed native shell")
                check(active(split).get("current_directory") == str(home), "clone lost CWD")
                command("printenv AMX_NATIVE_RC")
                send("local-tab")
                tabbed = wait(lambda s: active(s).get("local_tab_count") == 2 and
                              active(s).get("shell_integration"), "native local tab failed")
                check(active(tabbed).get("launch_program") == (None if MACOS and shell == "default" else program), "tab changed native shell")
                capture("panes")
                # Shrink and expand after integrated output and UI use. The
                # viewport and grid must remain nonzero and within the window.
                if driver:
                    previous_width = state().get("window_width")
                    driver.resize(720, 520)
                    resized = wait(lambda s: 0 < s.get("window_width", 0) < previous_width,
                                   "native resize not presented")
                else:
                    call("xdotool", "windowsize", handle, "720", "520")
                    resized = wait(lambda s: s.get("window_width") == 720 and s.get("window_height") == 520,
                                   "resize not presented")
                check(all(p.get("cell_width", 0) > 0 and p.get("cell_height", 0) > 0
                          for p in resized.get("panels", [])), "invalid grid after resize")
                capture("resized")
                return {"scenario": label, "renderer": expected, "scale": initial["scale_factor"],
                        "timing_kind": "diagnostic-polling-50ms-not-performance-baseline",
                        "startup_shell_ready_ms": ready_ms, "command_roundtrip_ms": command_samples,
                        "native_pixel_capture": not MACOS,
                        "shell_integration": True, "table": True, "tags": True,
                        "palette": True, "settings": True, "themes": True,
                        "split": True, "local_tab": True, "resize": True}
            finally:
                if driver:
                    driver.dispose()
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait(timeout=5)
                end = time.monotonic() + 5
                while time.monotonic() < end and any(pid_exists(pid) for pid in shell_pids):
                    time.sleep(.05)
                check(not any(pid_exists(pid) for pid in shell_pids),
                      "owned shell survived application shutdown")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--captures", type=Path, required=True)
    parser.add_argument("--backend", choices=("cpu", "vulkan", "webgpu"), action="append")
    parser.add_argument("--shell", choices=("default", "bash", "zsh", "fish"), action="append")
    parser.add_argument("--scale", type=float, action="append")
    parser.add_argument("--launch-mode", choices=("default", "fork"), action="append")
    parser.add_argument("--timeout", type=int, default=480)
    args = parser.parse_args()
    if not 1 <= args.timeout <= 1800:
        parser.error("timeout must be between 1 and 1800 seconds")
    signal.signal(signal.SIGTERM, cancel)
    signal.signal(signal.SIGINT, cancel)
    signal.signal(signal.SIGALRM, cancel)
    signal.alarm(args.timeout)
    captures = args.captures.resolve()
    captures.mkdir(parents=True, exist_ok=True)
    results = []
    if MACOS and args.scale:
        parser.error("macOS records its real display scale; controlled raster tests cover synthetic scales")
    for backend in args.backend or (["cpu", "webgpu"] if MACOS else ["cpu", "vulkan"]):
        for shell in args.shell or ["default", "bash", "zsh", "fish"]:
            for scale in args.scale or ([None] if MACOS else [1.0, 1.5]):
                for launch_mode in args.launch_mode or (["default", "fork"] if MACOS else ["default"]):
                    try:
                        result = run_case(args.binary.resolve(), captures, backend, shell, scale, launch_mode)
                    except (Failure, subprocess.SubprocessError, OSError, ValueError) as error:
                        # Emit scenario and bounded class, never private command/log data.
                        # These owners emit fixed diagnostics, never paths or terminal content.
                        message = str(error)[:240] if isinstance(error, (Failure, BenchmarkError)) else type(error).__name__
                        result = {"scenario": f"{backend}-{shell}-{scale if scale is not None else 'native'}-{launch_mode}", "failure": message}
                    results.append(result)
                    print(json.dumps(result), flush=True)
                    (captures / "summary.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    (captures / "summary.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    return int(any("failure" in result for result in results))


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Cancelled:
        print(json.dumps({"failure": "native driver cancelled or exceeded its deadline"}), flush=True)
        raise SystemExit(130)
