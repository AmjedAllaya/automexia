// Copyright 2026 Automexia contributors. Licensed under MIT OR Apache-2.0.
// Real AppKit objects and AX selectors on the process main thread. This is an
// API fixture, not a VoiceOver, OS IME, or physical-display certification.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Native macOS accessibility smoke requires macOS");
    std::process::exit(2);
}

#[cfg(target_os = "macos")]
fn main() {
    native::run();
}

#[cfg(target_os = "macos")]
mod native {
    use accesskit::{
        Action, ActionHandler, ActionRequest, ActivationHandler, Node, NodeId, Rect,
        Role, TreeId, TreeInfo, TreeUpdate,
    };
    use accesskit_macos::SubclassingAdapter;
    use objc2::{
        msg_send, msg_send_id,
        rc::{Id, autoreleasepool},
    };
    use objc2_app_kit::{
        NSApplication, NSBackingStoreType, NSView, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{
        MainThreadMarker, NSArray, NSObject, NSPoint, NSRange, NSRect, NSSize, NSString,
    };
    use std::{cell::RefCell, rc::Rc};

    const TEXT: &str = "中文 e\u{301} مرحبا שלום 👩\u{200d}💻";

    fn tree(writable: bool) -> TreeUpdate {
        let mut input = Node::new(Role::TextInput);
        input.set_label("Unicode input");
        input.set_value(TEXT);
        input.set_bounds(Rect::new(10.0, 10.0, 400.0, 50.0));
        input.set_children(vec![NodeId(2)]);
        input.add_action(Action::Focus);
        if writable {
            input.add_action(Action::SetValue);
        } else {
            input.set_read_only();
        }
        let mut text = Node::new(Role::TextRun);
        text.set_value(TEXT);
        text.set_character_lengths(vec![
            3, 3, 1, 3, 1, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 1, 11,
        ]);
        TreeUpdate {
            nodes: vec![(NodeId(1), input), (NodeId(2), text)],
            tree: Some(TreeInfo::new(NodeId(1))),
            tree_id: TreeId::ROOT,
            focus: NodeId(1),
        }
    }

    struct Activate;
    impl ActivationHandler for Activate {
        fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
            Some(tree(false))
        }
    }

    struct Actions(Rc<RefCell<Vec<ActionRequest>>>);
    impl ActionHandler for Actions {
        fn do_action(&mut self, action: ActionRequest) {
            self.0.borrow_mut().push(action);
        }
    }

    pub(super) fn run() {
        let mtm = MainThreadMarker::new().expect("smoke must run on the main thread");
        autoreleasepool(|_| {
            let _application = NSApplication::sharedApplication(mtm);
            let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(640.0, 480.0));
            // SAFETY: AppKit objects are created, queried and destroyed on the
            // real main thread. The window stays private and is never ordered front.
            let (window, view) = unsafe {
                let window = NSWindow::initWithContentRect_styleMask_backing_defer(
                    mtm.alloc(),
                    rect,
                    NSWindowStyleMask::Titled,
                    NSBackingStoreType::NSBackingStoreBuffered,
                    false,
                );
                window.setReleasedWhenClosed(false);
                (window, NSView::initWithFrame(mtm.alloc(), rect))
            };
            window.setContentView(Some(&view));
            let actions = Rc::new(RefCell::new(Vec::new()));
            // SAFETY: this retained NSView remains alive until after its single
            // subclassing adapter is explicitly dropped below.
            let mut adapter = unsafe {
                SubclassingAdapter::new(
                    Id::as_ptr(&view).cast_mut().cast(),
                    Activate,
                    Actions(Rc::clone(&actions)),
                )
            };
            // SAFETY: the adapter installs this AX selector on the live view.
            let children: Id<NSArray<NSObject>> =
                unsafe { msg_send_id![&view, accessibilityChildren] };
            assert_eq!(children.len(), 1);
            // SAFETY: the immutable array was just checked to contain one node.
            let node = unsafe { children.objectAtIndex(0) };
            // SAFETY: the adapter produced this retained PlatformNode, which
            // declares these selectors with exactly these argument/return types.
            unsafe {
                let full = NSRange::new(0, TEXT.encode_utf16().count());
                let value: Id<NSString> =
                    msg_send_id![&node, accessibilityStringForRange: full];
                assert_eq!(value.to_string(), TEXT);
                let invalid: Option<Id<NSString>> = msg_send_id![&node,
                    accessibilityStringForRange: NSRange::new(1, usize::MAX)];
                assert!(invalid.is_none());
                let frame: NSRect = msg_send![&node, accessibilityFrame];
                assert!(frame.size.width.is_finite() && frame.size.width > 0.0);
                assert!(frame.size.height.is_finite() && frame.size.height > 0.0);
                let input = NSString::from_str("fixture");
                let _: () = msg_send![&node, setAccessibilityValue: &*input];
                assert!(
                    actions.borrow().is_empty(),
                    "read-only node accepted a value action"
                );
            }
            if let Some(events) = adapter.update_if_active(|| tree(true)) {
                events.raise();
            }
            // SAFETY: same owned PlatformNode and declared AX selectors.
            unsafe {
                let oversized = NSString::from_str(&"x".repeat(8193));
                let _: () = msg_send![&node, setAccessibilityValue: &*oversized];
                assert!(
                    actions.borrow().is_empty(),
                    "oversized input reached the handler"
                );
                let value = NSString::from_str("reviewed fixture");
                let _: () = msg_send![&node, setAccessibilityValue: &*value];
                assert_eq!(actions.borrow().len(), 1);
                assert_eq!(actions.borrow()[0].action, Action::SetValue);
            }
            window.setContentView(None);
            // SAFETY: the node/view remain retained but the view is detached.
            // This directly exercises the previous native window().unwrap panic.
            unsafe {
                let frame: NSRect = msg_send![&node, accessibilityFrame];
                assert_eq!(frame, NSRect::ZERO);
                let hit: *mut NSObject = msg_send![&view,
                    accessibilityHitTest: NSPoint::new(1.0, 1.0)];
                assert!(hit.is_null());
            }
            drop(adapter);
            // Retained clients of a destroyed provider must become inert.
            // SAFETY: PlatformNode retains only a weak reference to its context.
            unsafe {
                let value: Option<Id<NSString>> = msg_send_id![&node,
                    accessibilityStringForRange: NSRange::new(0, 1)];
                assert!(value.is_none());
            }
        });
        println!(
            "PASS: native macOS AX Unicode ranges, bounds, action limits, detached view and teardown"
        );
    }
}
