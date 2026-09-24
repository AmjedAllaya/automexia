//! Shared native PowerShell editor fixture ownership and input plumbing.
use std::borrow::Cow;
use std::sync::{mpsc, Arc};
use std::time::Duration;

use rio_vt::ansi::CursorShape;
use rio_vt::crosswords::{Crosswords, CrosswordsSize, Mode};
use rio_vt::event::sync::FairMutex;
use rio_vt::event::{EventListener, Msg, RioEvent, WindowId};
use rio_vt::performer::{Machine, PtyWorkerHandle};

#[derive(Clone)]
pub(crate) struct Events(pub(crate) mpsc::SyncSender<RioEvent>);

impl EventListener for Events {
    fn send_event(&self, event: RioEvent, _: WindowId) {
        if matches!(&event, RioEvent::Title(title) if title.starts_with("EDITOR-") && title.len() < 128)
            || matches!(&event, RioEvent::ChildExited(..) | RioEvent::PtyWrite(..))
        {
            self.0
                .try_send(event)
                .expect("bounded editor fixture events");
        }
    }
}

pub(crate) struct Editor {
    pub(crate) handle: PtyWorkerHandle<()>,
    pub(crate) sender: rio_vt::performer::PtySender,
    pub(crate) events: mpsc::Receiver<RioEvent>,
    pub(crate) terminal: Arc<FairMutex<Crosswords<Events>>>,
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = self.sender.send(Msg::Shutdown);
        assert!(
            self.handle.join_timeout(Duration::from_secs(10)),
            "editor worker cleanup"
        );
    }
}

impl Editor {
    pub(crate) fn launch(pty: teletypewriter::Pty, columns: usize, rows: usize) -> Self {
        let (sender, receiver) = mpsc::sync_channel(32);
        let events = Events(sender);
        let terminal = Arc::new(FairMutex::new(Crosswords::new(
            CrosswordsSize::new(columns, rows),
            CursorShape::Block,
            events.clone(),
            WindowId::from(0),
            0,
            2_000,
        )));
        let machine = Machine::new(terminal.clone(), pty, events, WindowId::from(0), 0)
            .unwrap_or_else(|_| panic!("native editor worker startup"));
        let sender = machine.channel();
        Self {
            handle: machine.spawn(),
            sender,
            events: receiver,
            terminal,
        }
    }

    pub(crate) fn key(
        &self,
        virtual_key: u16,
        scan: u16,
        character: u16,
        fallback: &[u8],
    ) {
        let bytes = if self.terminal.lock().mode().contains(Mode::WIN32_INPUT) {
            format!("\x1b[{virtual_key};{scan};{character};1;0;1_\x1b[{virtual_key};{scan};0;0;0;1_").into_bytes()
        } else {
            fallback.to_vec()
        };
        self.sender
            .send(Msg::Input(Cow::Owned(bytes)))
            .expect("editor input");
    }
}
