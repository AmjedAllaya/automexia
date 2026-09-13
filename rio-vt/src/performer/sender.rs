use super::{channel, Msg, Ready};
use corcovado::{Registration, SetReadiness};

/// A PTY command channel with a separate, one-way cancellation wakeup.
/// Clones retain the same worker's cancellation signal, not a second owner.
#[derive(Clone)]
pub struct PtySender {
    channel: channel::Sender<Msg>,
    shutdown: Option<SetReadiness>,
}

impl PtySender {
    pub(super) fn managed(channel: channel::Sender<Msg>) -> (Self, Registration) {
        let (registration, shutdown) = Registration::new2();
        (
            Self {
                channel,
                shutdown: Some(shutdown),
            },
            registration,
        )
    }

    pub fn send(&self, message: Msg) -> Result<(), channel::SendError<Msg>> {
        if matches!(message, Msg::Shutdown) {
            if let Some(shutdown) = &self.shutdown {
                // This wakeup is independent of the ordinary channel's pending
                // count. A blocked paste must not hide Shutdown behind Resize.
                shutdown
                    .set_readiness(Ready::readable())
                    .map_err(channel::SendError::Io)?;
            }
        }
        self.channel.send(message)
    }

    pub(super) fn shutdown_requested(&self) -> bool {
        self.shutdown
            .as_ref()
            .is_some_and(|signal| signal.readiness().is_readable())
    }
}

// Disconnected/dead contexts and recording adapters keep ordinary message
// delivery. Live Machine channels always use the paired cancellation wakeup.
impl From<channel::Sender<Msg>> for PtySender {
    fn from(channel: channel::Sender<Msg>) -> Self {
        Self {
            channel,
            shutdown: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;
    use std::time::Duration;

    #[test]
    fn shutdown_wakes_independently_of_a_full_input_backlog() {
        let (raw, receiver) = channel::channel();
        let (sender, registration) = PtySender::managed(raw);
        let poll = corcovado::Poll::new().unwrap();
        poll.register(
            &registration,
            corcovado::Token(19),
            Ready::readable(),
            corcovado::PollOpt::edge(),
        )
        .unwrap();
        for _ in 0..256 {
            sender.send(Msg::Input(Cow::Borrowed(b"blocked"))).unwrap();
        }
        let mut events = corcovado::Events::with_capacity(4);
        poll.poll(&mut events, Some(Duration::ZERO)).unwrap();
        assert!(events.is_empty());
        let clone = sender.clone();
        clone.send(Msg::Shutdown).unwrap();
        poll.poll(&mut events, Some(Duration::from_secs(1)))
            .unwrap();
        assert!(events
            .iter()
            .any(|event| event.token() == corcovado::Token(19)));
        assert!(sender.shutdown_requested());
        assert!(
            matches!(receiver.try_recv(), Ok(Msg::Input(_))),
            "cancellation did not drain or reorder the input channel"
        );
        clone.send(Msg::Shutdown).unwrap();
        assert!(sender.shutdown_requested());
    }
}
