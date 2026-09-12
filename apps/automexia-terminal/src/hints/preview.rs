//! Compact destination review; never changes the terminal grid or starts I/O.
use super::HintState;
use crate::renderer::ui_theme::{color_u8, UiTheme};
use rio_backend::sugarloaf::{text::DrawOpts, Sugarloaf};
use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn lines(state: &HintState) -> [String; 4] {
    let Some(selected) = state.focused() else {
        return [
            "Links".into(),
            "No links in this view".into(),
            "Scroll or resize, then try again".into(),
            "Esc  Back".into(),
        ];
    };
    let action = match &selected.hint.action {
        rio_backend::config::hints::HintAction::Action {
            action: rio_backend::config::hints::HintInternalAction::Open,
        } if !super::safe_open_target(&selected.text) => {
            "Copy only · blocked destination".into()
        }
        rio_backend::config::hints::HintAction::Action { action } => {
            format!("{action:?}")
        }
        _ => "Run configured handler".into(),
    };
    let mut destination: String = selected
        .text
        .graphemes(true)
        .skip(state.preview_offset)
        .take(256)
        .collect();
    if state.preview_offset > 0 {
        destination.insert(0, '…');
    }
    if selected.text.graphemes(true).count() > state.preview_offset + 256 {
        destination.push('…');
    }
    [
        format!(
            "Links  {}/{}  ·  [{}]  ·  {action}",
            state.focused_index() + 1,
            state.matches().len(),
            state.focused_label()
        ),
        destination,
        "Tab / Shift+Tab  Navigate   ← →  Inspect URL".into(),
        "Enter  Activate   Ctrl+Shift+C  Copy   Esc  Back".into(),
    ]
}

pub(super) struct Preview {
    pub rect: [f32; 4],
    pub labels: Vec<(f32, f32, String, DrawOpts)>,
}

pub(super) fn layout(
    text: &mut rio_backend::sugarloaf::text::Text,
    state: &HintState,
    width: f32,
    height: f32,
    theme: UiTheme,
) -> Option<Preview> {
    if !state.is_active()
        || !width.is_finite()
        || !height.is_finite()
        || width < 2.0
        || height < 2.0
    {
        return None;
    }
    let panel_width = width.min(820.0);
    let x = (width - panel_width) * 0.5;
    let panel_height = height.min(112.0);
    // Keep the destination card away from the selected terminal rows.
    let y = if state.focus_in_lower_half() {
        0.0
    } else {
        height - panel_height
    };
    let mut preview = Preview {
        rect: [x, y, panel_width, panel_height],
        labels: Vec::with_capacity(4),
    };
    let available = (panel_width - 24.0).max(1.0);
    let mut opts = DrawOpts {
        font_size: 13.0,
        color: color_u8(theme.text),
        ..Default::default()
    };
    for (row, line) in lines(state).iter().enumerate() {
        let top = y + 10.0 + row as f32 * 24.0;
        if top + 16.0 > y + panel_height {
            break;
        }
        opts.color = color_u8(if row < 2 {
            theme.text
        } else {
            theme.muted_text
        });
        let fitted =
            crate::renderer::text_fit::fit_end(line, available, "…", |candidate, _| {
                text.measure(candidate, &opts)
            });
        if !fitted.display.is_empty() && panel_width >= 24.0 {
            preview
                .labels
                .push((x + 12.0, top, fitted.display.into_owned(), opts));
        }
    }
    Some(preview)
}

pub(crate) fn draw(sugarloaf: &mut Sugarloaf, state: &HintState, theme: UiTheme) {
    if !state.is_active() {
        return;
    }
    let size = sugarloaf.window_size();
    let scale = sugarloaf.scale_factor().max(0.1);
    let Some(preview) = layout(
        sugarloaf.text_mut(),
        state,
        size.width / scale,
        size.height / scale,
        theme,
    ) else {
        return;
    };
    let [x, y, w, h] = preview.rect;
    sugarloaf.begin_modal_layer();
    sugarloaf.rect(None, x, y, w, h, theme.background, 0.0, 49);
    sugarloaf.rect(None, x, y, w, h.min(2.0), theme.outline, 0.0, 49);
    for (x, y, label, opts) in &preview.labels {
        sugarloaf.text_mut().draw(*x, *y, label, opts);
    }
    sugarloaf.end_modal_layer();
}
