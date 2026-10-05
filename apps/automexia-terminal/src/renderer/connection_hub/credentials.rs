use super::*;
use crate::automexia::connections::{
    CredentialAction as Action, CredentialEditor, CredentialFocus as Focus,
};
use automexia_connectivity::connections::SshAgentEndpoint;

struct Control {
    rect: Rect,
    label: String,
    action: Action,
    focus: Focus,
    selected: bool,
}
struct CredentialLayout {
    card: Rect,
    controls: Vec<Control>,
    note_y: f32,
    bottom: f32,
}

fn layout(editor: &CredentialEditor, dimensions: (f32, f32, f32)) -> CredentialLayout {
    let viewport = Viewport::from_physical(dimensions.0, dimensions.1, dimensions.2);
    let width = (viewport.width - 24.0).clamp(1.0, 780.0);
    let preferred_height = if editor.draft.is_some() {
        380.0
    } else if editor.confirming_remove {
        240.0
    } else {
        (220.0 + editor.sources.len().min(8) as f32 * 48.0).min(620.0)
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
            label: "Remove source".into(),
            action: Action::ConfirmRemove,
            focus: Focus::Remove,
            selected: false,
        });
    } else if let Some(draft) = &editor.draft {
        let row_height = ((footer_y - card.y - 78.0) / 4.0).clamp(28.0, 62.0);
        let endpoint = match &draft.endpoint {
            SshAgentEndpoint::System => "System agent (default)".to_owned(),
            SshAgentEndpoint::UnixSocket { path } => path.clone(),
        };
        for (index, (label, value, focus, action)) in [
            (
                "Name",
                draft.display_name.clone(),
                Focus::Name,
                Action::Focus(Focus::Name),
            ),
            (
                "Vault",
                format!("< {} >", draft.provider.label()),
                Focus::Provider,
                Action::NextProvider,
            ),
            (
                "Agent socket",
                endpoint,
                Focus::Endpoint,
                Action::Focus(Focus::Endpoint),
            ),
        ]
        .into_iter()
        .filter(|(_, _, focus, _)| !cfg!(windows) || *focus != Focus::Endpoint)
        .enumerate()
        {
            controls.push(Control {
                rect: bounded_to(
                    Rect {
                        x: left,
                        y: card.y + 62.0 + row_height * index as f32,
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
        note_y = card.y + 64.0 + row_height * if cfg!(windows) { 2.0 } else { 3.0 };
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
        for (index, source) in editor.sources.iter().enumerate().skip(start).take(count) {
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
                label: format!("{}  ·  {}", source.display_name, source.provider.label()),
                action: Action::Select(index),
                focus: Focus::List,
                selected: editor.selected == index,
            });
        }
        note_y = card.y + 66.0 + editor.sources.len().min(count) as f32 * 48.0;
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
    CredentialLayout {
        card,
        controls,
        note_y,
        bottom: footer_y - 8.0,
    }
}

pub(super) fn hit_test(
    editor: &CredentialEditor,
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
            ConnectionHubHit::Credential(control.action)
        })
}

pub(super) fn render(
    editor: &CredentialEditor,
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
        "Vaults",
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
        "Remove this source? The vault and its credentials will remain unchanged. Sources used by saved connections cannot be removed."
    } else if let Some(draft) = &editor.draft {
        draft.provider.setup_summary()
    } else if editor.sources.is_empty() {
        "Add your vault's SSH agent. Vaults keep passwords and private keys; Automexia saves only this source's settings."
    } else {
        "Configured sources are not proof of authentication. Your vault app controls key access."
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
    crate::renderer::ui_theme::draw_shortcut_hint(
        sugarloaf.text_mut(),
        [
            layout.card.x + 16.0,
            layout.card.y + layout.card.height - 38.0,
            (layout.card.width - 32.0).max(0.0),
            32.0,
        ],
        "Tab: focus | Enter: select | Esc / Backspace / Alt+Left: back",
        10.0,
        *theme,
    );
    sugarloaf.end_modal_layer();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automexia::connections::HubLibrarySnapshot;
    use automexia_connectivity::connections::{CredentialProvider, CredentialSourceV1};

    #[test]
    fn credential_controls_stay_separate_and_pointer_hits_match_at_small_sizes_and_scaling(
    ) {
        let mut library = HubLibrarySnapshot::default();
        std::sync::Arc::make_mut(&mut library.document)
            .credential_sources
            .sources = (0..64)
            .map(|index| CredentialSourceV1 {
                schema_version: 1,
                id: format!("source-{index}"),
                revision: 1,
                display_name: format!("Vault {index}"),
                provider: CredentialProvider::OnePassword,
                endpoint: SshAgentEndpoint::System,
            })
            .collect();
        let mut editor = CredentialEditor::new(&library);
        editor.selected = 63;
        for mode in 0..3 {
            match mode {
                1 => {
                    editor.apply(Action::Edit);
                }
                2 => {
                    editor.apply(Action::Cancel);
                    editor.apply(Action::Remove);
                }
                _ => {}
            }
            for dimensions in [
                (320.0, 240.0, 1.0),
                (640.0, 480.0, 2.0),
                (1600.0, 950.0, 1.0),
            ] {
                let layout = layout(&editor, dimensions);
                for (index, control) in layout.controls.iter().enumerate() {
                    let rect = control.rect;
                    assert!(rect.x >= layout.card.x && rect.y >= layout.card.y);
                    assert!(rect.x + rect.width <= layout.card.x + layout.card.width);
                    assert!(rect.y + rect.height <= layout.card.y + layout.card.height);
                    assert_eq!(
                        hit_test(
                            &editor,
                            rect.x + rect.width * 0.5,
                            rect.y + rect.height * 0.5,
                            dimensions
                        ),
                        ConnectionHubHit::Credential(control.action.clone())
                    );
                    for other in &layout.controls[index + 1..] {
                        assert!(
                            rect.x + rect.width <= other.rect.x
                                || other.rect.x + other.rect.width <= rect.x
                                || rect.y + rect.height <= other.rect.y
                                || other.rect.y + other.rect.height <= rect.y,
                            "controls overlap"
                        );
                    }
                }
            }
        }
    }
}
