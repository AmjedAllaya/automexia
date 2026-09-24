use super::*;
use crate::Events;
use std::time::Duration;

fn race_registration<T>(
    mut receiver: Receiver<T>,
    action: impl FnOnce() + Send,
) -> (Receiver<T>, Events) {
    let (reached, snapshot) = mpsc::channel();
    let (resume, released) = mpsc::channel();
    receiver.ctl.registration_checkpoint = Some((reached, released));
    let poll = Poll::new().unwrap();
    std::thread::scope(|scope| {
        let actor = scope.spawn(move || {
            snapshot
                .recv_timeout(Duration::from_secs(5))
                .expect("registration reached pending snapshot");
            action();
            resume.send(()).unwrap();
        });
        poll.register(&receiver, Token(73), Ready::readable(), PollOpt::edge())
            .unwrap();
        actor.join().unwrap();
    });
    let mut events = Events::with_capacity(8);
    poll.poll(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    (receiver, events)
}

fn assert_readable(events: &Events) {
    assert!(
        events.iter().any(|event| {
            event.token() == Token(73) && event.readiness().is_readable()
        }),
        "registration lost the sole pending notification"
    );
}

#[test]
fn send_during_registration_wakes_without_later_traffic() {
    let (sender, receiver) = channel::<usize>();
    // Keep this sender alive through polling: dropping it would add another
    // notification and could conceal a lost message wakeup.
    let (receiver, events) = race_registration(receiver, || sender.send(41).unwrap());
    assert_eq!(receiver.try_recv(), Ok(41));
    assert_readable(&events);
}

#[test]
fn bounded_send_during_registration_wakes_without_later_traffic() {
    let (sender, receiver) = sync_channel::<usize>(1);
    let (receiver, events) = race_registration(receiver, || sender.send(42).unwrap());
    assert_eq!(receiver.try_recv(), Ok(42));
    assert_readable(&events);
}

#[test]
fn try_send_during_registration_wakes_without_later_traffic() {
    let (sender, receiver) = sync_channel::<usize>(1);
    let (receiver, events) = race_registration(receiver, || sender.try_send(43).unwrap());
    assert_eq!(receiver.try_recv(), Ok(43));
    assert_readable(&events);
}

#[test]
fn last_sender_drop_during_registration_wakes_without_later_traffic() {
    let (sender, receiver) = channel::<usize>();
    let (receiver, events) = race_registration(receiver, || drop(sender));
    assert_eq!(receiver.try_recv(), Err(mpsc::TryRecvError::Disconnected));
    assert_readable(&events);
}

#[test]
fn messages_before_and_after_registration_remain_observable() {
    let (sender, receiver) = channel::<usize>();
    sender.send(11).unwrap();
    let poll = Poll::new().unwrap();
    poll.register(&receiver, Token(73), Ready::readable(), PollOpt::edge())
        .unwrap();
    let mut events = Events::with_capacity(8);
    for expected in [11, 12] {
        poll.poll(&mut events, Some(Duration::from_millis(100)))
            .unwrap();
        assert_readable(&events);
        assert_eq!(receiver.try_recv(), Ok(expected));
        assert_eq!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty));
        if expected == 11 {
            sender.send(12).unwrap();
        }
    }
}

#[test]
fn empty_registration_stays_quiet_and_rejects_a_second_registration() {
    let (_sender, receiver) = channel::<usize>();
    let poll = Poll::new().unwrap();
    poll.register(&receiver, Token(73), Ready::readable(), PollOpt::edge())
        .unwrap();
    let mut events = Events::with_capacity(8);
    assert_eq!(poll.poll(&mut events, Some(Duration::ZERO)).unwrap(), 0);
    let error = poll
        .register(&receiver, Token(74), Ready::readable(), PollOpt::edge())
        .unwrap_err();
    assert_eq!(error.to_string(), "receiver already registered");
}
