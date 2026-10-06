use super::Screen;
use crate::ime::Preedit;
use rio_window::event::Ime;

impl Screen<'_> {
    /// Runs before overlay dispatch as well as before terminal input. A late
    /// commit from a dismissed terminal must not type into a new search box.
    pub(crate) fn reject_stale_terminal_ime(
        &mut self,
        event: &rio_window::event::WindowEvent,
    ) -> bool {
        if self.composition_owner.route().is_some_and(|route| {
            route != self.context_manager.current_route()
                || !self.composition_is_visible()
        }) {
            self.cancel_terminal_composition();
        }
        match event {
            rio_window::event::WindowEvent::Ime(Ime::Enabled) => {
                self.composition_owner.reset()
            }
            rio_window::event::WindowEvent::Ime(Ime::Preedit(text, _))
                if !text.is_empty() =>
            {
                // A fresh composition is allowed on the new input surface.
                if self.composition_owner.is_cancelled() {
                    self.composition_owner.reset();
                }
            }
            rio_window::event::WindowEvent::Ime(Ime::Commit(text)) => {
                return self.composition_owner.is_cancelled() || text.len() > 4096;
            }
            _ => {}
        }
        false
    }

    pub(crate) fn cancel_terminal_composition(&mut self) {
        self.composition_caret = None;
        if let Some(route) = self.composition_owner.cancel() {
            if let Some(context) = self.context_manager.get_by_route_id(route) {
                context.ime.set_preedit(None);
            }
        }
        self.context_manager.current_mut().ime.set_preedit(None);
        self.mark_dirty();
    }

    pub(crate) fn terminal_ime(&mut self, event: Ime) {
        let route = self.context_manager.current_route();
        if self
            .composition_owner
            .route()
            .is_some_and(|owner| owner != route)
        {
            self.cancel_terminal_composition();
        }
        match event {
            Ime::Enabled => {
                // Enabling the input service is not a pending composition.
                self.composition_owner.reset();
                self.context_manager.current_mut().ime.set_enabled(true);
            }
            Ime::Disabled => {
                self.cancel_terminal_composition();
                self.context_manager.current_mut().ime.set_enabled(false);
            }
            Ime::Preedit(text, cursor) => {
                if !text.is_empty() {
                    self.composition_owner.begin(route);
                }
                let preedit = (!text.is_empty())
                    .then(|| Preedit::new(text, cursor.map(|range| range.0)));
                self.context_manager.current_mut().ime.set_preedit(preedit);
            }
            Ime::Commit(text) => {
                self.context_manager.current_mut().ime.set_preedit(None);
                if self.composition_owner.commit(route) {
                    self.paste(&text, text.chars().count() > 1);
                }
            }
        }
        self.mark_dirty();
    }

    fn composition_is_visible(&self) -> bool {
        self.renderer.is_window_focused
            && !self.settings_view.is_open()
            && !self.renderer.command_palette.is_enabled()
            && !self.renderer.connection_hub.is_active()
            && !self.renderer.assistant.is_active()
            && !self.renderer.compatibility_inspector.is_active()
            && !self.renderer.confirm_quit.is_active()
            && !self.table_view.is_open()
            && !self.image_preview.is_visible()
            && !self.search_active()
    }

    pub(super) fn draw_terminal_composition(&mut self) {
        self.composition_caret = None;
        if self.composition_owner.route().is_some_and(|route| {
            route != self.context_manager.current_route()
                || !self.composition_is_visible()
        }) {
            self.cancel_terminal_composition();
        }
        if !self.composition_is_visible() {
            return;
        }
        let scale = self.sugarloaf.scale_factor();
        if !scale.is_finite() || scale <= 0.0 {
            return;
        }
        let grid = self.context_manager.current_grid();
        let Some(item) = grid.current_item() else {
            return;
        };
        let Some(preedit) = item.val.ime.preedit() else {
            return;
        };
        let layout = item.val.dimension;
        let content = &item.val.renderable_content;
        let row = content
            .command_rows
            .visual_row(content.cursor.state.pos.row.0.max(0) as usize);
        if row < 0 {
            return;
        }
        let pane = crate::layout::pane_terminal_rect_with_footer(
            item.layout_rect,
            scale,
            item.tab_count(),
            grid.footer_appearance,
        );
        let margin = grid.get_scaled_margin();
        let x = (pane[0]
            + margin.left
            + content.cursor.state.pos.col.0 as f32 * layout.cell.cell_width as f32)
            / scale;
        let y =
            (pane[1] + margin.top + row as f32 * layout.cell.cell_height as f32) / scale;
        let right = (pane[0] + pane[2]) / scale;
        let bottom = (pane[1] + pane[3]) / scale;
        let height = (layout.cell.cell_height as f32 / scale).min(bottom - y);
        let available = right - x;
        if available <= 0.0 || height <= 0.0 {
            return;
        }
        let theme =
            crate::renderer::ui_theme::UiTheme::from_colors(&self.renderer.named_colors);
        let opts = rio_backend::sugarloaf::text::DrawOpts {
            font_size: layout.font_size,
            color: crate::renderer::ui_theme::color_u8(theme.text),
            ..Default::default()
        };
        let text = &preedit.text;
        let width = self
            .sugarloaf
            .text_mut()
            .measure(text, &opts)
            .min(available);
        let caret = preedit
            .cursor_byte_offset
            .and_then(|offset| text.get(..offset))
            .map(|prefix| {
                self.sugarloaf
                    .text_mut()
                    .measure(prefix, &opts)
                    .min(available - 1.0)
            });
        self.sugarloaf.rect(
            None,
            x,
            y,
            width.max(1.0),
            height,
            theme.background,
            0.8,
            15,
        );
        self.sugarloaf.text_mut().draw_clipped(
            x,
            y,
            text,
            &opts,
            [x, y, available, height],
        );
        self.sugarloaf.rect(
            None,
            x,
            y + height - 1.0,
            width.max(1.0),
            1.0,
            theme.accent,
            0.9,
            15,
        );
        if let Some(caret) = caret {
            self.sugarloaf.rect(
                None,
                x + caret.max(0.0),
                y,
                1.0,
                height,
                theme.accent,
                0.9,
                15,
            );
            self.composition_caret =
                Some(((x + caret.max(0.0)) * scale, y * scale, height * scale));
        }
    }
}
