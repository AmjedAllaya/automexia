use super::*;
use crate::automexia::connections::{
    ProfileAction as Action, ProfileEditor, ProfileFocus as Focus,
};

struct Control {
    rect: Rect,
    label: String,
    action: Action,
    focus: Focus,
    selected: bool,
}
struct ProfileLayout {
    card: Rect,
    controls: Vec<Control>,
    note_y: f32,
    bottom: f32,
}

fn layout(editor: &ProfileEditor, dimensions: (f32, f32, f32)) -> ProfileLayout {
    let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
    let width = (viewport.width - 24.0).clamp(1.0, 780.0);
    let preferred_height = if editor.draft.is_some() {
        500.0
    } else if editor.confirming_remove {
        240.0
    } else {
        (220.0 + editor.library.profiles.profiles.len().min(8) as f32 * 48.0).min(620.0)
    };
    let height = (viewport.height - 24.0).clamp(1.0, preferred_height);
    let card = Rect {
        x: (viewport.width - width) * 0.5,
        y: (viewport.height - height) * 0.5,
        width,
        height,
    };
    let left = card.x + 16.0;
    let inner_width = (width - 32.0).max(1.0);
    let mut controls = Vec::new();
    let close = Rect {
        x: card.x + width - 112.0,
        y: card.y + 12.0,
        width: 96.0,
        height: 32.0,
    };
    controls.push(Control {
        rect: bounded_to(close, card),
        label: "Esc Back".into(),
        action: Action::Key(automexia_ui_model::connection_hub::HubKey::Escape),
        focus: Focus::Close,
        selected: false,
    });
    let footer_y = (card.y + height - 76.0).max(card.y + 48.0);
    let footer = |index: usize, count: usize| {
        bounded_to(
            Rect {
                x: left + (inner_width + 8.0) * index as f32 / count as f32,
                y: footer_y,
                width: ((inner_width - 8.0 * (count - 1) as f32) / count as f32).max(1.0),
                height: 34.0,
            },
            card,
        )
    };
    let mut note_y = card.y + 62.0;
    if editor.confirming_remove {
        controls.push(Control {
            rect: footer(0, 2),
            label: "Cancel".into(),
            action: Action::Cancel,
            focus: Focus::Cancel,
            selected: false,
        });
        controls.push(Control {
            rect: footer(1, 2),
            label: "Remove connection".into(),
            action: Action::ConfirmRemove,
            focus: Focus::Remove,
            selected: false,
        });
    } else if let Some(draft) = &editor.draft {
        let available = (footer_y - card.y - 78.0).max(1.0);
        let count = ((available / 32.0).floor() as usize).clamp(1, 5);
        let focused: usize = match editor.focus {
            Focus::Name => 0,
            Focus::Host => 1,
            Focus::User => 2,
            Focus::Port => 3,
            _ => 4,
        };
        let start = focused.saturating_sub(count - 1);
        let row_height = (available / count as f32).min(58.0);
        for (index, (label, value, focus, action)) in [
            (
                "Name",
                draft.name.clone(),
                Focus::Name,
                Action::Focus(Focus::Name),
            ),
            (
                "Host",
                draft.host.clone(),
                Focus::Host,
                Action::Focus(Focus::Host),
            ),
            (
                "User (optional)",
                draft.user.clone(),
                Focus::User,
                Action::Focus(Focus::User),
            ),
            (
                "Port (optional)",
                draft.port.clone(),
                Focus::Port,
                Action::Focus(Focus::Port),
            ),
            (
                "Credential source",
                format!("< {} >", editor.source_label()),
                Focus::Source,
                Action::CycleSource(true),
            ),
        ]
        .into_iter()
        .enumerate()
        .skip(start)
        .take(count)
        {
            controls.push(Control {
                rect: bounded_to(
                    Rect {
                        x: left,
                        y: card.y + 62.0 + row_height * (index - start) as f32,
                        width: inner_width,
                        height: row_height - 6.0,
                    },
                    card,
                ),
                label: format!("{label}: {value}"),
                action,
                focus,
                selected: false,
            });
        }
        note_y = card.y + 64.0 + row_height * count as f32;
        controls.push(Control {
            rect: footer(0, 2),
            label: "Save".into(),
            action: Action::Save,
            focus: Focus::Save,
            selected: false,
        });
        controls.push(Control {
            rect: footer(1, 2),
            label: "Cancel".into(),
            action: Action::Cancel,
            focus: Focus::Cancel,
            selected: false,
        });
    } else {
        let count = ((footer_y - card.y - 112.0) / 48.0).floor().max(1.0) as usize;
        let start = editor.selected.saturating_sub(count.saturating_sub(1));
        for (index, source) in editor
            .library
            .profiles
            .profiles
            .iter()
            .enumerate()
            .skip(start)
            .take(count)
        {
            controls.push(Control {
                rect: bounded_to(
                    Rect {
                        x: left,
                        y: card.y + 64.0 + (index - start) as f32 * 48.0,
                        width: inner_width,
                        height: 42.0,
                    },
                    card,
                ),
                label: format!("{}  ·  {}", source.display_name, source.public_target),
                action: Action::Select(index),
                focus: Focus::List,
                selected: editor.selected == index,
            });
        }
        note_y = card.y
            + 66.0
            + editor.library.profiles.profiles.len().min(count) as f32 * 48.0;
        for (index, (label, action, focus)) in [
            ("N  Add", Action::Add, Focus::Add),
            ("Edit", Action::Edit, Focus::Edit),
            ("Remove", Action::Remove, Focus::Remove),
        ]
        .into_iter()
        .enumerate()
        {
            controls.push(Control {
                rect: footer(index, 3),
                label: label.into(),
                action,
                focus,
                selected: false,
            });
        }
    }
    ProfileLayout {
        card,
        controls,
        note_y,
        bottom: footer_y - 8.0,
    }
}

pub(super) fn hit_test(
    editor: &ProfileEditor,
    x: f32,
    y: f32,
    dimensions: (f32, f32, f32),
) -> ConnectionHubHit {
    layout(editor, dimensions)
        .controls
        .into_iter()
        .filter(|control| editor.pending.is_none() || control.focus == Focus::Close)
        .find(|control| control.rect.contains(x, y))
        .map_or(ConnectionHubHit::Inert, |control| {
            ConnectionHubHit::Profile(control.action)
        })
}

pub(super) fn render(
    editor: &ProfileEditor,
    preedit: Option<&str>,
    sugarloaf: &mut Sugarloaf,
    dimensions: (f32, f32, f32),
    theme: &UiTheme,
) {
    let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
    let layout = layout(editor, dimensions);
    sugarloaf.begin_modal_layer();
    sugarloaf.rect(
        None,
        0.0,
        0.0,
        viewport.width,
        viewport.height,
        SCRIM,
        0.0,
        ORDER,
    );
    rounded(sugarloaf, layout.card, theme.border, CARD_RADIUS);
    rounded(
        sugarloaf,
        inset(layout.card, 1.0),
        theme.background,
        CARD_RADIUS,
    );
    let title = text(18.0, color_u8(theme.text), true);
    let body = text(13.0, color_u8(theme.text), false);
    let muted = text(12.0, color_u8(theme.muted_text), false);
    sugarloaf.text_mut().draw(
        layout.card.x + 16.0,
        layout.card.y + 18.0,
        "Saved connections",
        &title,
    );
    for control in &layout.controls {
        let focused = editor.focus == control.focus
            && (control.focus != Focus::List || control.selected);
        rounded(
            sugarloaf,
            control.rect,
            if focused { theme.accent } else { theme.border },
            7.0,
        );
        rounded(
            sugarloaf,
            inset(control.rect, 1.0),
            if control.selected {
                theme.raised
            } else {
                theme.surface
            },
            6.0,
        );
        let maximum = ((control.rect.width - 16.0) / 7.5).max(1.0) as usize;
        let label = if focused && editor.text_focused() {
            format!("{}{}|", control.label, preedit.unwrap_or_default())
        } else {
            control.label.clone()
        };
        sugarloaf.text_mut().draw(
            control.rect.x + 8.0,
            control.rect.y + (control.rect.height - 13.0) * 0.5,
            &truncated(&label, maximum),
            &body,
        );
    }
    let message = if editor.confirming_remove {
        "Remove this saved connection? Connections used by a workspace or another connection cannot be removed."
    } else if editor.draft.is_some() {
        "Save connection details only. Add agent sources in Vaults. Managed connection launching is unavailable in this build."
    } else {
        "N: add a connection. Enter: edit. Delete: remove. Passwords and private keys stay in your external agent."
    };
    let message = editor.notice.unwrap_or(message);
    let line_count = ((layout.bottom - layout.note_y) / 17.0).floor().max(0.0) as usize;
    for (index, line) in wrap_without_truncation(
        message,
        ((layout.card.width - 32.0) / 7.0).max(1.0) as usize,
    )
    .iter()
    .take(line_count)
    .enumerate()
    {
        sugarloaf.text_mut().draw(
            layout.card.x + 16.0,
            layout.note_y + index as f32 * 17.0,
            line,
            &muted,
        );
    }
    sugarloaf.text_mut().draw(
        layout.card.x + 16.0,
        layout.card.y + layout.card.height - 28.0,
        "Tab: focus   Enter: select   Esc: back",
        &muted,
    );
    sugarloaf.end_modal_layer();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automexia::connections::HubLibrarySnapshot;
    #[test]
    fn profile_focus_scrolls_small_form_without_overlapping_pointer_targets() {
        let mut editor = ProfileEditor::new(&HubLibrarySnapshot::default());
        editor.apply(Action::Add);
        for dimensions in [
            (320.0, 240.0, 1.0),
            (640.0, 480.0, 2.0),
            (1600.0, 950.0, 1.0),
        ] {
            for focus in [
                Focus::Name,
                Focus::Host,
                Focus::User,
                Focus::Port,
                Focus::Source,
                Focus::Save,
                Focus::Cancel,
            ] {
                editor.apply(Action::Focus(focus));
                let layout = layout(&editor, dimensions);
                let mut presentation = super::super::tests::presentation();
                presentation.profile_editor = Some(editor.clone());
                let mut hub = ConnectionHub::default();
                hub.set_presentation(Some(presentation));
                assert!(layout.controls.iter().any(|c| c.focus == focus));
                for (index, control) in layout.controls.iter().enumerate() {
                    let r = control.rect;
                    assert!(r.x >= layout.card.x && r.y >= layout.card.y);
                    assert!(r.x + r.width <= layout.card.x + layout.card.width);
                    assert!(r.y + r.height <= layout.card.y + layout.card.height);
                    assert_eq!(
                        hit_test(
                            &editor,
                            r.x + r.width / 2.0,
                            r.y + r.height / 2.0,
                            dimensions
                        ),
                        ConnectionHubHit::Profile(control.action.clone())
                    );
                    assert_eq!(
                        hub.hit_test(
                            r.x + r.width / 2.0,
                            r.y + r.height / 2.0,
                            dimensions
                        ),
                        Some(ConnectionHubHit::Profile(control.action.clone()))
                    );
                    for other in &layout.controls[index + 1..] {
                        let b = other.rect;
                        assert!(
                            r.x + r.width <= b.x
                                || b.x + b.width <= r.x
                                || r.y + r.height <= b.y
                                || b.y + b.height <= r.y,
                            "controls overlap"
                        );
                    }
                }
            }
        }
    }
}
