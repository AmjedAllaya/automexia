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
