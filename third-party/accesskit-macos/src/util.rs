// Copyright 2022 The AccessKit Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0 (found in
// the LICENSE-APACHE file) or the MIT license (found in
// the LICENSE-MIT file), at your option.

use accesskit::{Color, Point, Rect};
use accesskit_consumer::{NodeRef, TextPosition, TextRange};
use objc2::encode::{Encoding, RefEncode};
use objc2::{msg_send, rc::Id, runtime::AnyObject};
use objc2_app_kit::*;
use objc2_foundation::{NSPoint, NSRange, NSRect, NSSize};

pub(crate) fn from_ns_range<'a>(
    node: &'a NodeRef<'a>,
    ns_range: NSRange,
) -> Option<TextRange<'a>> {
    let end_index = crate::boundary::range_end(ns_range.location, ns_range.length)?;
    let pos = node.text_position_from_global_utf16_index(ns_range.location)?;
    let mut range = pos.to_degenerate_range();
    if ns_range.length > 0 {
        let end = node.text_position_from_global_utf16_index(end_index)?;
        range.set_end(end);
    }
    Some(range)
}

pub(crate) fn to_ns_range(range: &TextRange) -> NSRange {
    let start = range.start().to_global_utf16_index();
    let end = range.end().to_global_utf16_index();
    NSRange::from(start..end)
}

pub(crate) fn to_ns_range_for_character(pos: &TextPosition) -> NSRange {
    let mut range = pos.to_degenerate_range();
    if !pos.is_document_end() {
        range.set_end(pos.forward_to_character_end());
    }
    to_ns_range(&range)
}

pub(crate) fn from_ns_point(
    view: &NSView,
    node: &NodeRef,
    point: NSPoint,
) -> Option<Point> {
    let window = view.window()?;
    if !point.x.is_finite() || !point.y.is_finite() {
        return None;
    }
    let point = window.convertPointFromScreen(point);
    let point = view.convertPoint_fromView(point, None);
    // AccessKit coordinates are in physical (DPI-dependent) pixels, but
    // macOS provides logical (DPI-independent) coordinates here.
    let factor = window.backingScaleFactor();
    if !factor.is_finite() || factor <= 0.0 {
        return None;
    }
    let point = Point::new(
        point.x * factor,
        if view.isFlipped() {
            point.y * factor
        } else {
            let view_bounds = view.bounds();
            (view_bounds.size.height - point.y) * factor
        },
    );
    let point = node.transform().inverse() * point;
    (point.x.is_finite() && point.y.is_finite()).then_some(point)
}

pub(crate) fn to_ns_rect(view: &NSView, rect: Rect) -> NSRect {
    let Some(window) = view.window() else {
        return NSRect::ZERO;
    };
    // AccessKit coordinates are in physical (DPI-dependent)
    // pixels, but macOS expects logical (DPI-independent)
    // coordinates here.
    let factor = window.backingScaleFactor();
    if !factor.is_finite()
        || factor <= 0.0
        || ![rect.x0, rect.y0, rect.x1, rect.y1]
            .iter()
            .all(|v| v.is_finite())
    {
        return NSRect::ZERO;
    }
    let rect = NSRect {
        origin: NSPoint {
            x: rect.x0 / factor,
            y: if view.isFlipped() {
                rect.y0 / factor
            } else {
                let view_bounds = view.bounds();
                view_bounds.size.height - rect.y1 / factor
            },
        },
        size: NSSize {
            width: rect.width() / factor,
            height: rect.height() / factor,
        },
    };
    let rect = view.convertRect_toView(rect, None);
    window.convertRectToScreen(rect)
}

fn color_channel_to_f64(channel: u8) -> f64 {
    (channel as f64) / 255.0
}

// TODO: can be removed after updating objc2 to 0.6 which has proper `CGColor` support
#[repr(C)]
struct CGColor {
    _private: [u8; 0],
}

unsafe impl RefEncode for CGColor {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct("CGColor", &[]));
}

pub(crate) fn to_color_attribute(color: Color) -> Id<AnyObject> {
    let ns_color = unsafe {
        NSColor::colorWithSRGBRed_green_blue_alpha(
            color_channel_to_f64(color.red),
            color_channel_to_f64(color.green),
            color_channel_to_f64(color.blue),
            color_channel_to_f64(color.alpha),
        )
    };
    let cg_color: *const CGColor = unsafe { msg_send![&ns_color, CGColor] };
    unsafe { Id::retain(cg_color as *mut AnyObject).unwrap() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use accesskit::{Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate};
    use accesskit_consumer::Tree;

    #[test]
    fn native_text_range_rejects_overflow_and_preserves_surrogate_pairs() {
        let root_id = NodeId(1);
        let text_id = NodeId(2);
        let mut root = Node::new(Role::TextInput);
        root.set_children(vec![text_id]);
        let mut text = Node::new(Role::TextRun);
        text.set_value("a\u{1f600}b");
        text.set_character_lengths(vec![1, 4, 1]);
        let tree = Tree::new(
            TreeUpdate {
                nodes: vec![(root_id, root), (text_id, text)],
                tree: Some(TreeInfo::new(root_id)),
                tree_id: TreeId::ROOT,
                focus: root_id,
            },
            true,
        );
        let node = tree.state().root();
        // A valid start plus a hostile length used to overflow before lookup.
        assert!(from_ns_range(&node, NSRange::new(1, usize::MAX)).is_none());
        assert!(from_ns_range(&node, NSRange::new(usize::MAX, 1)).is_none());
        assert!(from_ns_range(&node, NSRange::new(0, 5)).is_none());
        assert_eq!(
            from_ns_range(&node, NSRange::new(1, 2)).unwrap().text(),
            "\u{1f600}"
        );
        assert_eq!(from_ns_range(&node, NSRange::new(4, 0)).unwrap().text(), "");
    }
}
