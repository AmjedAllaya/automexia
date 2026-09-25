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
#[derive(Clone)]
struct Events(mpsc::SyncSender<RioEvent>);
impl EventListener for Events {
    fn send_event(&self, event: RioEvent, _: WindowId) {
        if matches!(&event, RioEvent::Title(title) if title == "INLINE-PIPELINE-READY")
            || matches!(&event, RioEvent::PtyWrite(..) | RioEvent::ChildExited(..))
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
        self.sender
            .send(Msg::Shutdown)
            .expect("request fixture shutdown");
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
fn posix_script() -> String {
    let mut script = String::from("printf '%s\\n' ");
    for row in fixture::pipeline_rows() {
        assert!(!row.contains('\''));
        script.push('\'');
        script.push_str(&row);
        script.push_str("' ");
    }
    script
        .push_str("''; printf '\\033]2;INLINE-PIPELINE-READY\\007'; IFS= read -r answer");
    script
}
fn command(wsl: bool) -> (String, Vec<String>) {
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
            return (
                exe.to_string_lossy().into_owned(),
                vec![
                    "--distribution".into(),
                    distro,
                    "--exec".into(),
                    "/bin/sh".into(),
                    "-c".into(),
                    posix_script(),
                ],
            );
        }
        let exe = std::path::PathBuf::from(windows)
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut script = String::new();
        for row in fixture::pipeline_rows() {
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
        assert!(
            !wsl,
            "WSL/ConPTY test must run from a Windows-host test binary"
        );
        ("/bin/sh".into(), vec!["-c".into(), posix_script()])
    }
}
fn run(wsl: bool) {
    for columns in [80usize, 320] {
        let (program, args) = command(wsl);
        let pty = teletypewriter::create_pty(
            Some(&program),
            args,
            &None,
            None,
            columns as u16,
            64,
        )
        .unwrap_or_else(|error| panic!("native fixture launch: {error}"));
        let (tx, rx) = mpsc::sync_channel(32);
        let listener = Events(tx);
        let terminal = Arc::new(FairMutex::new(Crosswords::new(
            CrosswordsSize::new(columns, 64),
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
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let event = rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("native table fixture readiness timed out");
            match event {
                RioEvent::Title(title) if title == "INLINE-PIPELINE-READY" => break,
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
        let mut tables = InlineTables::default();
        tables.refresh(snapshot);
        assert_eq!(tables.surfaces.len(), 1,
            "native transport did not preserve a recognized table at width {columns}: {:?}. Inspect native wrap provenance; do not weaken this assertion.", tables.diagnostics());
        assert_eq!(
            tables.surfaces[0].table.column_starts(),
            fixture::PIPELINE_STARTS
        );
        assert_eq!(tables.surfaces[0].table.source(), fixture::pipeline_rows());
        session.close();
    }
}
#[test]
fn inline_pipeline_native_transport_preserves_wide_records() {
    run(false);
}
#[cfg(windows)]
#[test]
#[ignore = "requires an explicitly selected installed WSL distro; run through --wsl-distro"]
fn inline_pipeline_native_wsl_conpty_preserves_wide_records() {
    run(true);
}
