#![cfg(any(unix, windows))]
//! Native transport tests, not a compositor or physical-input latency claim.
//! The default fixture uses only a non-profile system shell and fictional text.
use automexia_terminal::automexia::inline_tables::{InlineTables, Snapshot};
use rio_backend::{
    ansi::CursorShape,
    crosswords::{Crosswords, CrosswordsSize},
    event::{sync::FairMutex, EventListener, Msg, RioEvent, WindowId},
    performer::{Machine, PtySender, PtyWorkerHandle},
};
use std::borrow::Cow;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
mod fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../automexia-ui-model/tests/support/inline_pipeline_fixture.rs"
    ));
}
#[derive(Clone, Copy, Debug)]
enum NativeFixture {
    PaddedEndpoints,
    #[cfg(windows)]
    LongPorts,
}
impl NativeFixture {
    fn rows(self) -> Vec<String> {
        match self {
            Self::PaddedEndpoints => fixture::pipeline_rows(),
            #[cfg(windows)]
            Self::LongPorts => [
                ["CONTAINER ID", "IMAGE", "COMMAND", "CREATED", "STATUS", "PORTS", "NAMES"],
                ["0123456789ab", "registry/node:v1.2.34", "\"/usr/local/bin/entr…\"", "3 weeks ago", "Up 13 hours", "0.0.0.0:8080->80/tcp, 0.0.0.0:8443->443/tcp, 127.0.0.1:43665->6443/tcp", "example-control-plane"],
                ["123456789abc", "registry/node:v1.2.34", "\"/usr/local/bin/entr…\"", "3 weeks ago", "Up 13 hours", "", "example-worker"],
            ]
            .map(|r| format!("{:<15}{:<23}{:<25}{:<14}{:<14}{:<73}{}", r[0], r[1], r[2], r[3], r[4], r[5], r[6]))
            .into_iter()
            .collect(),
        }
    }
    fn starts(self) -> &'static [usize] {
        match self {
            Self::PaddedEndpoints => &fixture::PIPELINE_STARTS,
            #[cfg(windows)]
            Self::LongPorts => &[0, 15, 38, 63, 77, 91, 164],
        }
    }
}
#[cfg(windows)]
const INTERACTIVE_DIMENSIONS: &[(usize, usize)] = &[
    (80, 24),
    (132, 24),
    (133, 24),
    (136, 24),
    (160, 24),
    (320, 24),
    (80, 64),
    (320, 64),
];
#[derive(Clone)]
struct Events(mpsc::SyncSender<RioEvent>, bool);
impl EventListener for Events {
    fn send_event(&self, event: RioEvent, _: WindowId) {
        if matches!(&event, RioEvent::Title(title) if title == "INLINE-PIPELINE-READY")
            || matches!(&event, RioEvent::PtyWrite(..) | RioEvent::ChildExited(..))
            || self.1 && matches!(&event, RioEvent::TerminalDamaged(..))
        {
            self.0
                .try_send(event)
                .expect("bounded native fixture event queue");
        }
    }
}
struct Session {
    sender: PtySender,
    handle: Option<PtyWorkerHandle<()>>,
}
impl Session {
    fn close(mut self) {
        // A finite fixture can exit after publishing all verified rows and
        // readiness. Disconnection is already-stopped, not failed cleanup;
        // still require the worker's bounded join and reject real I/O errors.
        match self.sender.send(Msg::Shutdown) {
            Ok(()) | Err(corcovado::channel::SendError::Disconnected(_)) => {}
            Err(error) => panic!("request fixture shutdown: {error:?}"),
        }
        let mut handle = self.handle.take().unwrap();
        assert!(
            handle.join_timeout(Duration::from_secs(10)),
            "native fixture cleanup timed out"
        );
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.sender.send(Msg::Shutdown);
        if let Some(mut handle) = self.handle.take() {
            if !handle.join_timeout(Duration::from_secs(10)) {
                // Never double-panic during a failed assertion. The outer test
                // runner enforces a process-tree deadline as a second boundary.
                eprintln!("native inline fixture cleanup did not acknowledge completion");
            }
        }
    }
}
fn posix_script(rows: &[String]) -> String {
    let mut script = String::from("printf '%s\\n' ");
    for row in rows {
        assert!(!row.contains('\''));
        script.push('\'');
        script.push_str(row);
        script.push_str("' ");
    }
    script
        .push_str("''; printf '\\033]2;INLINE-PIPELINE-READY\\007'; IFS= read -r answer");
    script
}
fn command(
    wsl: bool,
    interactive_rcfile: Option<&str>,
    nested_host_script: Option<&std::path::Path>,
    rows: &[String],
) -> (String, Vec<String>) {
    #[cfg(windows)]
    {
        let windows = std::env::var_os("SystemRoot").expect("Windows system root");
        if wsl {
            let distro = std::env::var("AUTOMEXIA_TABLE_TEST_WSL_DISTRO")
                .expect("explicit WSL distro is required for this opt-in test");
            assert!(
                !distro.is_empty()
                    && !distro.starts_with('-')
                    && !distro.chars().any(char::is_control)
            );
            let exe = std::path::PathBuf::from(windows).join("System32/wsl.exe");
            if let Some(rcfile) = interactive_rcfile {
                if let Some(host_script) = nested_host_script {
                    let host = std::path::PathBuf::from(
                        std::env::var_os("SystemRoot").expect("Windows system root"),
                    )
                    .join("System32/WindowsPowerShell/v1.0/powershell.exe");
                    return (
                        host.to_string_lossy().into_owned(),
                        vec![
                            "-NoLogo".into(),
                            "-NoProfile".into(),
                            "-NonInteractive".into(),
                            "-File".into(),
                            host_script.to_string_lossy().into_owned(),
                            "-WslExe".into(),
                            exe.to_string_lossy().into_owned(),
                            "-Distro".into(),
                            distro,
                            "-Rcfile".into(),
                            rcfile.into(),
                        ],
                    );
                }
                return (
                    exe.to_string_lossy().into_owned(),
                    vec![
                        "--distribution".into(),
                        distro,
                        "--exec".into(),
                        "/bin/bash".into(),
                        "--noprofile".into(),
                        "--rcfile".into(),
                        rcfile.into(),
                        "-i".into(),
                    ],
                );
            }
            return (
                exe.to_string_lossy().into_owned(),
                vec![
                    "--distribution".into(),
                    distro,
                    "--exec".into(),
                    "/bin/sh".into(),
                    "-c".into(),
                    posix_script(rows),
                ],
            );
        }
        let exe = std::path::PathBuf::from(windows)
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut script = String::new();
        for row in rows {
            assert!(!row.contains('\''));
            script.push_str(&format!("[Console]::WriteLine('{row}');"));
        }
        script.push_str("[Console]::WriteLine();[Console]::Write([char]27 + ']2;INLINE-PIPELINE-READY' + [char]7);[Console]::ReadLine() | Out-Null");
        (
            exe.to_string_lossy().into_owned(),
            vec![
                "-NoLogo".into(),
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                script,
            ],
        )
    }
    #[cfg(unix)]
    {
        assert!(interactive_rcfile.is_none());
        assert!(nested_host_script.is_none());
        assert!(
            !wsl,
            "WSL/ConPTY test must run from a Windows-host test binary"
        );
        ("/bin/sh".into(), vec!["-c".into(), posix_script(rows)])
    }
}
#[cfg(windows)]
fn interactive_resource_fixture(
    rows: &[String],
) -> (tempfile::TempDir, String, std::path::PathBuf) {
    let target =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    let directory = tempfile::Builder::new()
        .prefix("inline-interactive-fixture-")
        .tempdir_in(target)
        .expect("isolated native fixture directory");
    let path = directory.path().join("fixture.bash");
    // Exercise the package-owned prompt resource without a user profile,
    // persistent history, completion adapter, or saved alias configuration.
    let mut resource = String::from(concat!(
        "export AUTOMEXIA_SHELL_INTEGRATION=1 TERM_PROGRAM=Automexia ",
        "AUTOMEXIA_PLAIN_LS=1 AUTOMEXIA_AMX=0 AUTOMEXIA_CONTEXT_PATH_HINTS=0\n",
        "export AUTOMEXIA_CONFIG_HOME=/nonexistent-inline-fixture-config\n",
        "HISTFILE=/dev/null\nunset PROMPT_COMMAND\ncd /\n",
    ));
    resource.push_str(
        &include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../shell-integration/bash/automexia.bash"
        ))
        .replace("\r\n", "\n"),
    );
    resource.push_str("\n__inline_fixture_table() {\n");
    for row in rows {
        assert!(!row.contains('\''));
        resource.push_str(&format!("printf '%s\\n' '{row}'\n"));
    }
    // Readiness follows the real prompt's OSC 133 B. The first prompt permits
    // input; the second proves output and the next prompt completed normally.
    resource.push_str("}\nPS1+='\\[\\e]2;INLINE-PIPELINE-READY\\a\\]'\n");
    std::fs::write(&path, resource).expect("native fixture resource");
    let distro = std::env::var("AUTOMEXIA_TABLE_TEST_WSL_DISTRO")
        .expect("explicit WSL distro is required for this opt-in test");
    assert!(
        !distro.is_empty()
            && !distro.starts_with('-')
            && !distro.chars().any(char::is_control)
    );
    let exe = std::path::PathBuf::from(
        std::env::var_os("SystemRoot").expect("Windows system root"),
    )
    .join("System32/wsl.exe");
    let converted = std::process::Command::new(exe)
        .args(["--distribution", &distro, "--exec", "wslpath", "-u"])
        .arg(&path)
        .output()
        .expect("convert application-owned fixture path");
    assert!(converted.status.success(), "fixture path conversion failed");
    let path = String::from_utf8(converted.stdout)
        .expect("UTF-8 fixture path")
        .trim()
        .to_owned();
    assert!(path.starts_with('/') && !path.chars().any(char::is_control));
    let host_script = directory.path().join("nested.ps1");
    // Each caller-owned value is a distinct native argument and a typed script
    // parameter; no command-string interpolation or shell evaluation is used.
    std::fs::write(
        &host_script,
        concat!(
            "param([string]$WslExe, [string]$Distro, [string]$Rcfile)\n",
            "& $WslExe --distribution $Distro --exec /bin/bash ",
            "--noprofile --rcfile $Rcfile -i\nexit $LASTEXITCODE\n",
        ),
    )
    .expect("fixed nested launcher script");
    (directory, path, host_script)
}
fn run(
    wsl: bool,
    interactive: bool,
    nested: bool,
    fixture: NativeFixture,
    dimensions: &[(usize, usize)],
) {
    assert!(!nested || (wsl && interactive));
    let expected = fixture.rows();
    let expected_starts = fixture.starts();
    #[cfg(windows)]
    let resource = interactive.then(|| interactive_resource_fixture(&expected));
    #[cfg(windows)]
    let rcfile = resource.as_ref().map(|(_, path, _)| path.as_str());
    #[cfg(windows)]
    let nested_host_script = resource
        .as_ref()
        .filter(|_| nested)
        .map(|(_, _, path)| path.as_path());
    #[cfg(unix)]
    let rcfile = None;
    #[cfg(unix)]
    let nested_host_script = None;
    for &(columns, screen_rows) in dimensions {
        let (program, args) = command(wsl, rcfile, nested_host_script, &expected);
        #[cfg(windows)]
        let pty = teletypewriter::create_pty(
            Some(&program),
            args,
            &None,
            None,
            columns as u16,
            screen_rows as u16,
        )
        .unwrap_or_else(|error| panic!("native fixture launch: {error}"));
        #[cfg(unix)]
        let pty = teletypewriter::create_pty_with_spawn(
            Some(&program),
            args,
            &None,
            None,
            columns as u16,
            screen_rows as u16,
            0,
            0,
        )
        .unwrap_or_else(|error| panic!("native fixture launch: {error}"));
        let (tx, rx) = mpsc::sync_channel(32);
        let listener = Events(tx, interactive);
        let terminal = Arc::new(FairMutex::new(Crosswords::new(
            CrosswordsSize::new(columns, screen_rows),
            CursorShape::Block,
            listener.clone(),
            WindowId::from(0),
            0,
            2000,
        )));
        let machine = Machine::new(terminal.clone(), pty, listener, WindowId::from(0), 0)
            .unwrap_or_else(|error| panic!("native machine launch: {error}"));
        let session = Session {
            sender: machine.channel(),
            handle: Some(machine.spawn()),
        };
        let mut tables = InlineTables::default();
        let mut entered_fixture = !interactive;
        let mut prompt_ready = false;
        let mut table_seen_on_damage = false;
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let event = rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap_or_else(|_| {
                    use rio_backend::crosswords::grid::Dimensions;
                    use rio_backend::crosswords::pos::{Column, Line, Pos};
                    let terminal = terminal.lock();
                    for row in terminal.grid.topmost_line().0.max(-256)..=terminal.grid.bottommost_line().0 {
                        let text = terminal.bounds_to_display_string_bounded(
                            Pos::new(Line(row), Column(0)),
                            Pos::new(Line(row), terminal.grid.last_column()),
                            4096,
                        ).unwrap();
                        let safe = if expected.iter().any(|line| line.contains(text.trim())) {
                            text.as_str()
                        } else {
                            "<non-fixture>"
                        };
                        eprintln!("fixture row {row}, wrap={}, semantic={:?}, text={safe:?}",
                            terminal.grid[Line(row)][terminal.grid.last_column()].wrapline(),
                            terminal.grid[Line(row)].semantic_prompt);
                    }
                    panic!("native table readiness timed out: fixture={fixture:?}, width={columns}, height={screen_rows}, nested={nested}, prompt_ready={prompt_ready}, diagnostics={:?}", tables.diagnostics())
                });
            match event {
                RioEvent::Title(title) if title == "INLINE-PIPELINE-READY" => {
                    if entered_fixture {
                        prompt_ready = true;
                        if !interactive || table_seen_on_damage {
                            break;
                        }
                    } else {
                        entered_fixture = true;
                        session
                            .sender
                            .send(Msg::Input(Cow::Borrowed(b"__inline_fixture_table\r")))
                            .expect("submit fixed fixture command");
                    }
                }
                RioEvent::TerminalDamaged(_) if interactive => {
                    let snapshot = {
                        let mut terminal = terminal.lock();
                        // Match the renderer's existing damage-consumption
                        // boundary; do not trigger a resize or synthetic redraw.
                        terminal.damage_event_in_flight = false;
                        terminal.reset_damage();
                        Snapshot::capture(&*terminal)
                    };
                    tables.refresh(snapshot);
                    // The complete long endpoint cell and all remaining source
                    // cells must survive a real output-damage frame. Surface
                    // count alone would miss dropped or shortened wide values.
                    table_seen_on_damage |= tables.surfaces.len() == 1
                        && tables.surfaces[0].table.source() == expected
                        && tables.surfaces[0].table.column_starts() == expected_starts;
                    if prompt_ready && table_seen_on_damage {
                        break;
                    }
                }
                RioEvent::PtyWrite(_, bytes) => session
                    .sender
                    .send(Msg::Input(Cow::Owned(bytes.into_bytes())))
                    .unwrap(),
                RioEvent::ChildExited(_, status) => {
                    panic!("fixture exited before readiness: {status:?}")
                }
                _ => {}
            }
        }
        let snapshot = Snapshot::capture(&*terminal.lock());
        tables.refresh(snapshot);
        assert!(!interactive || table_seen_on_damage);
        assert_eq!(tables.surfaces.len(), 1,
            "native transport did not preserve a recognized table at width {columns}, height {screen_rows}, nested={nested}: {:?}. Inspect native wrap provenance; do not weaken this assertion.", tables.diagnostics());
        assert_eq!(tables.surfaces[0].table.column_starts(), expected_starts);
        assert_eq!(tables.surfaces[0].table.source(), expected);
        session.close();
    }
}
#[test]
fn inline_pipeline_native_transport_preserves_wide_records() {
    run(
        false,
        false,
        false,
        NativeFixture::PaddedEndpoints,
        &[(80, 64), (320, 64)],
    );
}
#[cfg(windows)]
#[test]
#[ignore = "requires an explicitly selected installed WSL distro; run through --wsl-distro"]
fn inline_pipeline_native_wsl_conpty_preserves_wide_records() {
    run(
        true,
        false,
        false,
        NativeFixture::PaddedEndpoints,
        &[(80, 64), (320, 64)],
    );
}
#[cfg(windows)]
#[test]
#[ignore = "requires an explicitly selected installed WSL Bash distro; run through --wsl-distro"]
fn inline_pipeline_native_wsl_conpty_interactive_prompt_preserves_wide_records() {
    run(
        true,
        true,
        false,
        NativeFixture::PaddedEndpoints,
        INTERACTIVE_DIMENSIONS,
    );
}
#[cfg(windows)]
#[test]
#[ignore = "requires an explicitly selected installed WSL Bash distro; run through --wsl-distro"]
fn inline_pipeline_native_wsl_conpty_nested_powershell_preserves_wide_records() {
    run(
        true,
        true,
        true,
        NativeFixture::PaddedEndpoints,
        INTERACTIVE_DIMENSIONS,
    );
}
#[cfg(windows)]
#[test]
#[ignore = "requires an explicitly selected installed WSL Bash distro; run through --wsl-distro"]
fn inline_pipeline_native_wsl_conpty_interactive_long_ports() {
    run(
        true,
        true,
        false,
        NativeFixture::LongPorts,
        INTERACTIVE_DIMENSIONS,
    );
}
#[cfg(windows)]
#[test]
#[ignore = "requires an explicitly selected installed WSL Bash distro; run through --wsl-distro"]
fn inline_pipeline_native_wsl_conpty_nested_powershell_long_ports() {
    run(
        true,
        true,
        true,
        NativeFixture::LongPorts,
        INTERACTIVE_DIMENSIONS,
    );
}
// Keep each narrow boundary independently runnable so one native failure does
// not prevent the other initial dimensions and schemas from being exercised.
#[cfg(windows)]
#[test]
#[ignore = "requires an explicitly selected installed WSL Bash distro; run through --wsl-distro"]
fn inline_pipeline_native_wsl_conpty_narrow_short_pane() {
    run(
        true,
        true,
        false,
        NativeFixture::PaddedEndpoints,
        &[(31, 8)],
    );
}
#[cfg(windows)]
#[test]
#[ignore = "requires an explicitly selected installed WSL Bash distro; run through --wsl-distro"]
fn inline_pipeline_native_wsl_conpty_nested_powershell_narrow_short_pane() {
    run(true, true, true, NativeFixture::PaddedEndpoints, &[(31, 8)]);
}
#[cfg(windows)]
#[test]
#[ignore = "requires an explicitly selected installed WSL Bash distro; run through --wsl-distro"]
fn inline_pipeline_native_wsl_conpty_narrow_tall_pane() {
    run(
        true,
        true,
        false,
        NativeFixture::PaddedEndpoints,
        &[(31, 64)],
    );
}
#[cfg(windows)]
#[test]
#[ignore = "requires an explicitly selected installed WSL Bash distro; run through --wsl-distro"]
fn inline_pipeline_native_wsl_conpty_short_pane() {
    run(
        true,
        true,
        false,
        NativeFixture::PaddedEndpoints,
        &[(80, 8)],
    );
}

#[cfg(windows)]
mod ssh_native {
    use super::*;
    use automexia_ssh_integration::{
        session::{RemoteContext, RemoteContextField, RemoteDirectoryUpdate},
        Capabilities, GenerationKey, Negotiation, Phase,
    };
    use rio_backend::{
        crosswords::{grid::Dimensions, pos::Line},
        event::WindowSize,
        performer::handler::Processor,
    };

    // This uses the same PTY worker, bounded event queue and cleanup guard as
    // the native table tests. No second reader or SSH process owner is added.
    fn wait(
        terminal: &Arc<FairMutex<Crosswords<Events>>>,
        session: &Session,
        events: &mpsc::Receiver<RioEvent>,
        stage: &str,
        predicate: impl Fn(&Crosswords<Events>) -> bool,
    ) {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            {
                let mut terminal = terminal.lock();
                if predicate(&terminal) {
                    return;
                }
                terminal.damage_event_in_flight = false;
                terminal.reset_damage();
            }
            if Instant::now() >= deadline {
                use rio_backend::crosswords::pos::{Column, Pos};
                let terminal = terminal.lock();
                let visible = terminal
                    .bounds_to_display_string_bounded(
                        Pos::new(Line(0), Column(0)),
                        Pos::new(
                            terminal.grid.bottommost_line(),
                            terminal.grid.last_column(),
                        ),
                        16 * 1024,
                    )
                    .unwrap_or_default();
                let keys = [
                    "automexia_ssh_ready",
                    "automexia_ssh_user",
                    "automexia_ssh_cwd",
                    "automexia_ssh_context",
                ]
                .map(|name| (name, terminal.user_vars.contains_key(name)));
                let markers = [
                    "AMX_AUDIT_PROMPT",
                    "Permission denied",
                    "required SSH integration",
                    "not recognized",
                    "host key",
                ]
                .map(|name| (name, visible.contains(name)));
                panic!("native SSH stage timed out: {stage}; scope={}, prompt={}, input_raw={}, metadata={keys:?}, markers={markers:?}",
                    terminal.integration_scope_active(), terminal.integration_scope_prompt_active(), terminal.title == "HELPER-SLEEP:True");
            }
            match events.recv_timeout(Duration::from_millis(20)) {
                Ok(RioEvent::PtyWrite(_, bytes)) => session
                    .sender
                    .send(Msg::Input(Cow::Owned(bytes.into_bytes())))
                    .expect("native SSH protocol response"),
                Ok(RioEvent::ChildExited(_, status)) => {
                    panic!("native SSH exited during {stage}: {status:?}");
                }
                Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    panic!("native SSH worker disconnected during {stage}");
                }
            }
        }
    }

    fn input(session: &Session, bytes: &'static [u8]) {
        session
            .sender
            .send(Msg::Input(Cow::Borrowed(bytes)))
            .expect("fixed native SSH fixture input");
    }

    fn has_result(terminal: &Crosswords<Events>, code: i32) -> bool {
        (terminal.grid.topmost_line().0.max(-256)..=terminal.grid.bottommost_line().0)
            .any(|line| {
                terminal.grid[Line(line)]
                    .semantic_command_result
                    .is_some_and(|result| result.exit_code == Some(code))
            })
    }

    fn key(terminal: &Crosswords<Events>) -> GenerationKey {
        let scope = terminal
            .integration_scope()
            .expect("real CLI scope is active");
        GenerationKey::new(scope.pane, scope.generation)
            .expect("bounded native scope key")
    }

    fn helper_context(
        terminal: &Crosswords<Events>,
    ) -> Option<automexia_ssh_integration::helper::ContextUpdate> {
        use automexia_ssh_integration::helper::{ContextUpdate, Revision};
        let key = key(terminal);
        let revision =
            Revision::decode(key, terminal.user_vars.get("automexia_ssh_revision")?)
                .ok()??;
        ContextUpdate::decode(
            key,
            revision.revision(),
            terminal.user_vars.get("automexia_ssh_context_v2")?,
        )
        .ok()?
    }

    #[test]
    #[ignore = "requires explicitly selected native helper and PS5/PS7 runtimes; run under the bounded native fixture owner"]
    fn inline_pipeline_native_helper_powershell_discovers_and_retires() {
        use automexia_ssh_integration::{
            bootstrap,
            helper::{ContextField, DISCOVERY_ENV_KEYS},
        };
        use sha2::{Digest, Sha256};
        let helper = std::env::var_os("AUTOMEXIA_SSH_HELPER_NATIVE_BINARY")
            .map(std::path::PathBuf::from)
            .expect("explicit native helper executable is required");
        assert!(helper.is_absolute() && helper.is_file());
        for (shell, ignore_ctrl_c) in [
            ("powershell", 0),
            ("powershell", 1),
            ("pwsh", 0),
            ("pwsh", 1),
        ] {
            // The outer QA owner isolates this serial test in its own Windows
            // process group. Exercise both possible inherited ignore states;
            // the real helper must restore native shell Ctrl+C delivery.
            // SAFETY: only this disposable test process's handler state changes.
            assert_ne!(
                unsafe {
                    windows_sys::Win32::System::Console::SetConsoleCtrlHandler(
                        None,
                        ignore_ctrl_c,
                    )
                },
                0
            );
            let fixture = tempfile::tempdir().unwrap();
            let home = fixture.path().join("home");
            let temporary = fixture.path().join("temporary");
            let first = fixture.path().join("first");
            let second = fixture.path().join("second");
            for directory in [&home, &temporary, &first, &second] {
                std::fs::create_dir(directory).unwrap();
            }
            for (directory, branch, workspace) in [
                (&first, "fixture-first", "workspace-first"),
                (&second, "fixture-second", "workspace-second"),
            ] {
                std::fs::create_dir(directory.join(".git")).unwrap();
                std::fs::write(
                    directory.join(".git/HEAD"),
                    format!("ref: refs/heads/{branch}\n"),
                )
                .unwrap();
                std::fs::create_dir(directory.join(".terraform")).unwrap();
                std::fs::write(directory.join(".terraform/environment"), workspace)
                    .unwrap();
            }
            let kube = fixture.path().join("kube.yaml");
            std::fs::write(&kube, "current-context: fixture-one\ncontexts:\n- name: fixture-one\n  context: {namespace: namespace-one}\n").unwrap();
            let mut environment: Vec<(String, String)> = DISCOVERY_ENV_KEYS
                .iter()
                .map(|name| ((*name).into(), String::new()))
                .collect();
            environment.extend([
                ("HOME".into(), home.to_string_lossy().into()),
                ("USERPROFILE".into(), home.to_string_lossy().into()),
                ("APPDATA".into(), home.to_string_lossy().into()),
                ("LOCALAPPDATA".into(), home.to_string_lossy().into()),
                ("TEMP".into(), temporary.to_string_lossy().into()),
                ("TMP".into(), temporary.to_string_lossy().into()),
                ("DOCKER_HOST".into(), String::new()),
                ("KUBECONFIG".into(), kube.to_string_lossy().into()),
                (
                    "AMX_NATIVE_HELPER_FIRST".into(),
                    first.to_string_lossy().into(),
                ),
                (
                    "AMX_NATIVE_HELPER_SECOND".into(),
                    second.to_string_lossy().into(),
                ),
                (
                    "AMX_NATIVE_HELPER_KUBE".into(),
                    kube.to_string_lossy().into(),
                ),
            ]);
            let pty = teletypewriter::create_pty(
                Some(helper.to_str().unwrap()),
                vec!["--session-v1".into(), shell.into(), "3".into(), "7".into()],
                &Some(first.to_string_lossy().into()),
                Some(environment),
                100,
                30,
            )
            .unwrap_or_else(|_| panic!("native helper ConPTY launch failed: {shell}"));
            let (tx, events) = mpsc::sync_channel(32);
            let listener = Events(tx, true);
            let terminal = Arc::new(FairMutex::new(Crosswords::new(
                CrosswordsSize::new(100, 30),
                CursorShape::Block,
                listener.clone(),
                WindowId::from(0),
                0,
                2_000,
            )));
            let secret = [0x42; 32];
            let digest: String = Sha256::digest(secret)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            let begin = bootstrap::user_var_frame(
                "terminal_scope_v1",
                &format!("AMXSCOPE1|begin|{digest}|3|7|{shell}"),
            )
            .unwrap();
            {
                let mut terminal = terminal.lock();
                terminal.current_directory = Some("C:\\fixture\\local".into());
                Processor::default().advance(&mut *terminal, begin.as_bytes());
            }
            let machine =
                Machine::new(terminal.clone(), pty, listener, WindowId::from(0), 0)
                    .unwrap();
            let session = Session {
                sender: machine.channel(),
                handle: Some(machine.spawn()),
            };
            wait(
                &terminal,
                &session,
                &events,
                "native helper readiness",
                |terminal| {
                    terminal.integration_scope_prompt_active()
                        && terminal.user_vars.contains_key("automexia_ssh_ready")
                },
            );
            {
                let terminal = terminal.lock();
                let cwd = RemoteDirectoryUpdate::decode(
                    key(&terminal),
                    terminal.user_vars.get("automexia_ssh_cwd").unwrap(),
                )
                .unwrap()
                .unwrap();
                assert!(
                    cwd.path.as_ref().is_some_and(|path| path
                        .remote_text()
                        .eq_ignore_ascii_case(&first.to_string_lossy())),
                    "native helper must publish its actual isolated working directory"
                );
                assert!(
                    terminal
                        .user_vars
                        .get("automexia_ssh_user")
                        .is_some_and(|value| value
                            .strip_prefix("AMXSSHUSER1|3|7|")
                            .is_some_and(|user| !user.is_empty())),
                    "native helper must publish bounded scoped identity"
                );
            }
            // Fixed fixture commands override selector changes from normal native
            // startup; profiles themselves are neither edited nor logged.
            input(&session, b"Set-Location -LiteralPath $env:AMX_NATIVE_HELPER_FIRST; $env:KUBECONFIG=$env:AMX_NATIVE_HELPER_KUBE; $env:GIT_BRANCH=''; $env:KUBECONTEXT=''; $env:KUBE_CONTEXT=''; $env:KUBE_NAMESPACE=''; $env:TF_WORKSPACE=''\r");
            wait(
                &terminal,
                &session,
                &events,
                "native helper passive discovery",
                |terminal| {
                    helper_context(terminal).is_some_and(|context| {
                        context.value(ContextField::GitBranch) == Some("fixture-first")
                            && context.value(ContextField::KubernetesContext)
                                == Some("fixture-one")
                            && context.value(ContextField::KubernetesNamespace)
                                == Some("namespace-one")
                            && context.value(ContextField::TerraformWorkspace)
                                == Some("workspace-first")
                    })
                },
            );
            assert!(terminal.lock().current_directory.is_none());
            let revision = helper_context(&terminal.lock()).unwrap().revision();
            std::fs::write(
                first.join(".git/HEAD"),
                "ref: refs/heads/fixture-refreshed\n",
            )
            .unwrap();
            std::fs::write(&kube, "current-context: fixture-two\ncontexts:\n- name: fixture-two\n  context: {namespace: namespace-two}\n").unwrap();
            std::fs::write(first.join(".terraform/environment"), "workspace-refreshed")
                .unwrap();
            wait(
                &terminal,
                &session,
                &events,
                "native helper idle file refresh",
                |terminal| {
                    helper_context(terminal).is_some_and(|context| {
                        context.revision() == revision
                            && context.value(ContextField::GitBranch)
                                == Some("fixture-refreshed")
                            && context.value(ContextField::KubernetesContext)
                                == Some("fixture-two")
                            && context.value(ContextField::TerraformWorkspace)
                                == Some("workspace-refreshed")
                    })
                },
            );
            input(&session, b"Set-Location -LiteralPath $env:AMX_NATIVE_HELPER_SECOND; $env:TF_WORKSPACE='workspace-selected'\r");
            wait(
                &terminal,
                &session,
                &events,
                "native helper cwd and selector revision",
                |terminal| {
                    helper_context(terminal).is_some_and(|context| {
                        context.revision() > revision
                            && context.value(ContextField::GitBranch)
                                == Some("fixture-second")
                            && context.value(ContextField::TerraformWorkspace)
                                == Some("workspace-selected")
                    })
                },
            );
            input(&session, b"Write-Error fixture-failure\r");
            wait(
                &terminal,
                &session,
                &events,
                "native helper failed command",
                |terminal| {
                    terminal.integration_scope_prompt_active() && has_result(terminal, 1)
                },
            );
            input(&session, b"[Console]::Write(\"$([char]27)]2;HELPER-SLEEP:$([Console]::TreatControlCAsInput)$([char]7)\"); Start-Sleep -Seconds 30\r");
            wait(
                &terminal,
                &session,
                &events,
                "native helper running command",
                |terminal| {
                    terminal.title.starts_with("HELPER-SLEEP:")
                        && !terminal.integration_scope_prompt_active()
                },
            );
            input(&session, b"\x03");
            wait(
                &terminal,
                &session,
                &events,
                "native helper interrupt returns prompt",
                |terminal| terminal.integration_scope_prompt_active(),
            );
            terminal.lock().resize(CrosswordsSize::new(119, 41));
            session
                .sender
                .send(Msg::Resize(WindowSize {
                    cols: 119,
                    rows: 41,
                    width: 0,
                    height: 0,
                }))
                .unwrap();
            input(&session, b"[Console]::Write(\"$([char]27)]2;HELPER-SIZE:$([Console]::WindowHeight) $([Console]::WindowWidth)$([char]7)\")\r");
            wait(
                &terminal,
                &session,
                &events,
                "native helper terminal resize",
                |terminal| {
                    terminal.title == "HELPER-SIZE:41 119"
                        && terminal.integration_scope_prompt_active()
                },
            );
            input(&session, b"exit 7\r");
            let deadline = Instant::now() + Duration::from_secs(20);
            loop {
                match events
                    .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                    .expect("native helper exit before deadline")
                {
                    RioEvent::ChildExited(_, status) => {
                        assert_eq!(status, Some(7));
                        break;
                    }
                    RioEvent::PtyWrite(_, bytes) => session
                        .sender
                        .send(Msg::Input(Cow::Owned(bytes.into_bytes())))
                        .unwrap(),
                    _ => {
                        let mut terminal = terminal.lock();
                        terminal.damage_event_in_flight = false;
                        terminal.reset_damage();
                    }
                }
            }
            session.close();
            assert!(
                !std::fs::read_dir(&temporary).unwrap().any(|entry| entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("automexia-ssh-shell-")),
                "native helper must retire its exact temporary startup files"
            );
            // The real wrapper owns scope controls. This helper-only fixture
            // models its verified end after the process has been retired.
            let end = bootstrap::user_var_frame(
                "terminal_scope_v1",
                &format!("AMXSCOPE1|end|{}", "42".repeat(32)),
            )
            .unwrap();
            let mut terminal = terminal.lock();
            Processor::default().advance(&mut *terminal, end.as_bytes());
            assert!(!terminal.integration_scope_active());
            assert_eq!(
                terminal.current_directory.as_deref(),
                Some(std::path::Path::new("C:\\fixture\\local"))
            );
        }
    }

    #[test]
    #[ignore = "requires disposable pinned loopback SSH config via AUTOMEXIA_SSH_NATIVE_CONFIG; run the selected test explicitly"]
    fn inline_pipeline_native_ssh_conpty_preserves_remote_terminal_contract() {
        let config = std::env::var_os("AUTOMEXIA_SSH_NATIVE_CONFIG")
            .map(std::path::PathBuf::from)
            .expect("explicit disposable native SSH config is required");
        assert!(
            config.is_absolute() && config.is_file(),
            "native SSH config must be an existing absolute fixture file"
        );
        let binary = std::env::var_os("AUTOMEXIA_SSH_NATIVE_BINARY")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_amx").into());
        assert!(
            binary.is_absolute() && binary.is_file(),
            "native SSH application must be an existing absolute executable"
        );
        // Use the product's console-subsystem entry point, which preserves
        // standard handles and waits for the sibling GUI-subsystem executable.
        let pty = teletypewriter::create_pty(
            Some(binary.to_str().expect("native fixture launcher path")),
            vec![
                "+ssh".into(),
                "--shell".into(),
                "bash".into(),
                "--integration".into(),
                "required".into(),
                "--".into(),
                "-F".into(),
                config.to_str().expect("native fixture config path").into(),
                "fixture-host".into(),
            ],
            &None,
            None,
            100,
            30,
        )
        .unwrap_or_else(|_| panic!("native SSH ConPTY launch failed"));
        let (tx, events) = mpsc::sync_channel(32);
        let listener = Events(tx, true);
        let terminal = Arc::new(FairMutex::new(Crosswords::new(
            CrosswordsSize::new(100, 30),
            CursorShape::Block,
            listener.clone(),
            WindowId::from(0),
            0,
            2_000,
        )));
        {
            let mut terminal = terminal.lock();
            terminal.current_directory = Some("C:\\fixture\\local".into());
            // Seed unrelated local state through the real VT owner; remote
            // admission must suspend it and verified return must restore it.
            Processor::default().advance(
                &mut *terminal,
                b"\x1b]1337;SetUserVar=automexia_shell_user=bG9jYWwtZml4dHVyZQ==\x07",
            );
        }
        let machine = Machine::new(terminal.clone(), pty, listener, WindowId::from(0), 0)
            .expect("native SSH PTY worker");
        let session = Session {
            sender: machine.channel(),
            handle: Some(machine.spawn()),
        };
        wait(&terminal, &session, &events, "remote ready", |terminal| {
            terminal.integration_scope_active()
                && terminal.integration_scope_prompt_active()
                && [
                    "automexia_ssh_ready",
                    "automexia_ssh_user",
                    "automexia_ssh_cwd",
                    "automexia_ssh_context",
                ]
                .iter()
                .all(|name| terminal.user_vars.contains_key(*name))
        });
        {
            let terminal = terminal.lock();
            let key = key(&terminal);
            let mut negotiation = Negotiation::new(key, 0, 30_000).unwrap();
            negotiation
                .receive(terminal.user_vars["automexia_ssh_ready"].as_bytes(), 0)
                .unwrap();
            assert_eq!(negotiation.phase(), Phase::Ready);
            assert!(negotiation.capabilities().contains(Capabilities::PROMPT));
            assert_eq!(
                terminal.user_vars["automexia_ssh_user"],
                format!("AMXSSHUSER1|{}|{}|fixture", key.pane(), key.generation())
            );
            let directory = RemoteDirectoryUpdate::decode(
                key,
                &terminal.user_vars["automexia_ssh_cwd"],
            )
            .unwrap()
            .unwrap()
            .path
            .unwrap();
            assert_eq!(directory.remote_text(), "/home/fixture");
            assert!(RemoteContext::decode(
                key,
                &terminal.user_vars["automexia_ssh_context"]
            )
            .unwrap()
            .is_some());
            assert!(
                terminal.current_directory.is_none(),
                "remote cwd must not become a local filesystem path"
            );
            assert!(!terminal.user_vars.contains_key("automexia_shell_user"));
        }
        input(&session, b"export GIT_BRANCH=fixture-branch KUBECONTEXT=fixture-cluster TF_WORKSPACE=fixture-workspace\r");
        wait(
            &terminal,
            &session,
            &events,
            "remote context update",
            |terminal| {
                terminal
                    .user_vars
                    .get("automexia_ssh_context")
                    .and_then(|value| {
                        RemoteContext::decode(key(terminal), value).ok().flatten()
                    })
                    .is_some_and(|context| {
                        context.value(RemoteContextField::GitBranch)
                            == Some("fixture-branch")
                            && context.value(RemoteContextField::KubernetesContext)
                                == Some("fixture-cluster")
                            && context.value(RemoteContextField::TerraformWorkspace)
                                == Some("fixture-workspace")
                    })
                    && terminal.integration_scope_prompt_active()
            },
        );
        input(&session, b"false\r");
        wait(
            &terminal,
            &session,
            &events,
            "typed command exit status",
            |terminal| {
                terminal.integration_scope_prompt_active() && has_result(terminal, 1)
            },
        );
        input(&session, b"sleep 30\r");
        wait(
            &terminal,
            &session,
            &events,
            "running command",
            |terminal| !terminal.integration_scope_prompt_active(),
        );
        input(&session, b"\x03");
        wait(
            &terminal,
            &session,
            &events,
            "interrupt returns prompt",
            |terminal| {
                terminal.integration_scope_prompt_active() && has_result(terminal, 130)
            },
        );
        terminal.lock().resize(CrosswordsSize::new(119, 41));
        session
            .sender
            .send(Msg::Resize(WindowSize {
                cols: 119,
                rows: 41,
                width: 0,
                height: 0,
            }))
            .expect("native SSH resize");
        input(
            &session,
            b"printf '\\033]2;SSH-SIZE:%s\\007' \"$(stty size)\"\r",
        );
        wait(
            &terminal,
            &session,
            &events,
            "remote terminal resize",
            |terminal| {
                terminal.title == "SSH-SIZE:41 119"
                    && terminal.integration_scope_prompt_active()
            },
        );
        input(&session, b"exit 7\r");
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            match events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("native SSH exit before deadline")
            {
                RioEvent::ChildExited(_, status) => {
                    assert_eq!(status, Some(7));
                    break;
                }
                RioEvent::PtyWrite(_, bytes) => session
                    .sender
                    .send(Msg::Input(Cow::Owned(bytes.into_bytes())))
                    .expect("native SSH final protocol response"),
                _ => {
                    let mut terminal = terminal.lock();
                    terminal.damage_event_in_flight = false;
                    terminal.reset_damage();
                }
            }
        }
        {
            let terminal = terminal.lock();
            assert!(
                !terminal.integration_scope_active(),
                "verified scope end must precede child exit"
            );
            assert_eq!(
                terminal
                    .user_vars
                    .get("automexia_shell_user")
                    .map(String::as_str),
                Some("local-fixture")
            );
            assert_eq!(
                terminal.current_directory.as_deref(),
                Some(std::path::Path::new("C:\\fixture\\local"))
            );
            assert!(!terminal.user_vars.contains_key("automexia_ssh_context"));
        }
        session.close();
    }
}
