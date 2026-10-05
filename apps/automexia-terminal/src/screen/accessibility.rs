//! Accessibility reads the same visible snapshot and pane geometry as paint.
use super::Screen;
use accesskit::{Action, Live, Node, Rect, Role};
use automexia_ui_model::accessibility::{
    sanitize_accessible_text, text_run, Element, Key, ROOT,
};
use rio_backend::crosswords::style::StyleFlags;

impl Screen<'_> {
    pub(crate) fn accessibility_frame(&self, welcome: bool) -> (Vec<Element>, Key) {
        let size = self.sugarloaf.window_size();
        let bounds = Rect::new(0.0, 0.0, size.width as f64, size.height as f64);
        let mut root = Node::new(Role::Window);
        root.set_label("Automexia");
        root.set_bounds(bounds);
        let mut elements = vec![Element {
            key: ROOT,
            parent: None,
            node: root,
        }];
        let overlay = if welcome {
            Some((
                1,
                "Welcome to Automexia. Press Enter to get started.".into(),
            ))
        } else if self.renderer.confirm_quit.is_active() {
            Some((2, self.renderer.confirm_quit.accessibility_summary()))
        } else if self.renderer.assistant.is_active() {
            Some((3, "Diagnostic dialog. Escape closes. Keyboard focus remains in this dialog.".into()))
        } else if self.renderer.compatibility_inspector.is_active() {
            Some((
                4,
                self.renderer
                    .compatibility_inspector
                    .accessibility_summary(),
            ))
        } else if self.renderer.connection_hub.is_active() {
            Some((5, self.renderer.connection_hub.accessibility_summary()))
        } else if self.settings_view.is_open() {
            Some((6, self.settings_view.accessibility_summary()))
        } else if self.renderer.command_palette.is_enabled() {
            self.renderer
                .command_palette
                .accessibility_summary()
                .map(|text| (7, text))
        } else if self.table_view.is_open() {
            Some((
                8,
                "Table viewer. Arrow keys navigate. Escape closes.".into(),
            ))
        } else if self.image_preview.is_visible() {
            Some((
                10,
                "Image preview. Escape closes. The covered terminal is not exposed."
                    .into(),
            ))
        } else if self.search_active() {
            Some((
                9,
                "Search terminal output. Type a query. Enter advances. Escape closes."
                    .into(),
            ))
        } else {
            None
        };
        if let Some((item, label)) = overlay {
            let surface = match item {
                2 => Some(self.renderer.confirm_quit.accessibility_surface((
                    size.width,
                    size.height,
                    self.sugarloaf.scale_factor(),
                ))),
                5 => Some(self.renderer.connection_hub.accessibility_surface((
                    size.width,
                    size.height,
                    self.sugarloaf.scale_factor(),
                ))),
                6 => Some(
                    self.settings_view
                        .accessibility_surface(self.sugarloaf.scale_factor(), bounds),
                ),
                7 => Some(self.renderer.command_palette.accessibility_surface((
                    size.width,
                    size.height,
                    self.sugarloaf.scale_factor(),
                ))),
                _ => None,
            };
            if let Some(surface) = surface {
                elements.extend(surface.elements);
                return (elements, surface.focus);
            }
            let key = Key {
                owner: u64::MAX,
                item,
            };
            let mut dialog = Node::new(Role::Dialog);
            dialog.set_label(sanitize_accessible_text(&label));
            dialog.set_modal();
            dialog.set_bounds(bounds);
            dialog.add_action(Action::Focus);
            elements.push(Element {
                key,
                parent: Some(ROOT),
                node: dialog,
            });
            return (elements, key);
        }

        let grid = self.context_manager.current_grid();
        let Some(item) = grid.current_item() else {
            return (elements, ROOT);
        };
        let context = &item.val;
        // Route zero is valid; reserve owner zero exclusively for the window.
        let owner = context.route_id as u64 + 1;
        let key = Key { owner, item: 0 };
        let mut terminal = Node::new(Role::Terminal);
        terminal.set_label("Terminal output");
        terminal.set_description("Visible terminal, in cell order. Use terminal keyboard shortcuts to scroll or change panes.");
        terminal.add_action(Action::Focus);
        terminal.set_read_only();
        let margin = grid.get_scaled_margin();
        let panel = crate::layout::pane_terminal_rect(
            item.layout_rect,
            self.sugarloaf.scale_factor(),
            item.tab_count(),
        );
        let x = (panel[0] + margin.left) as f64;
        let y = (panel[1] + margin.top) as f64;
        let cell_width = context.dimension.cell.cell_width as f64;
        let cell_height = context.dimension.cell.cell_height as f64;
        let panel_bounds = Rect::new(
            x,
            y,
            ((panel[0] + panel[2]) as f64).min(bounds.x1),
            ((panel[1] + panel[3]) as f64).min(bounds.y1),
        );
        terminal.set_bounds(panel_bounds);
        elements.push(Element {
            key,
            parent: Some(ROOT),
            node: terminal,
        });
        let content = &context.renderable_content;
        let mut remaining_bytes = 48 * 1024usize;
        for (index, row) in content.visible_rows.iter().take(256).enumerate() {
            let visual_row = content.command_rows.visual_row(index);
            let top = y + visual_row as f64 * cell_height;
            if visual_row < 0 || top >= panel_bounds.y1 {
                continue;
            }
            let mut text = String::new();
            for square in row.inner.iter().take(content.columns) {
                if square.is_spacer() || square.is_leading_spacer() {
                    continue;
                }
                let hidden = content
                    .style_table
                    .get(usize::from(square.style_id()))
                    .is_some_and(|style| style.flags.contains(StyleFlags::HIDDEN));
                if hidden {
                    text.push(' ');
                    continue;
                }
                text.push(square.c());
                if let Some(extra) =
                    square.extras_id().and_then(|id| content.extras.get(&id))
                {
                    text.extend(extra.zerowidth.iter().take(64));
                }
                if text.len() >= 4096 {
                    break;
                }
            }
            let mut text = sanitize_accessible_text(text.trim_end_matches(['\0', ' ']));
            text.push('\n');
            if text.len() > remaining_bytes {
                break;
            }
            remaining_bytes -= text.len();
            let mut run = text_run(&text);
            run.set_bounds(Rect::new(
                x,
                top,
                (x + content.columns as f64 * cell_width).min(panel_bounds.x1),
                (top + cell_height).min(panel_bounds.y1),
            ));
            elements.push(Element {
                key: Key {
                    owner,
                    item: index as u64 + 1,
                },
                parent: Some(key),
                node: run,
            });
            if let Some(result) = row.semantic_command_result {
                let mut status = Node::new(Role::Label);
                status.set_label(match result.exit_code {
                    Some(0) => "Command succeeded".into(),
                    Some(code) => format!("Command failed, exit code {code}"),
                    None => "Command completed, exit status unavailable".into(),
                });
                // History is readable without repeatedly announcing completion
                // every time a row is redrawn, resized or scrolled into view.
                status.set_live(Live::Off);
                elements.push(Element {
                    key: Key {
                        owner,
                        item: index as u64 + 1024,
                    },
                    parent: Some(key),
                    node: status,
                });
            }
        }
        if let Some(preedit) = context.ime.preedit() {
            let mut composition = Node::new(Role::TextInput);
            composition.set_label("Input method composition");
            composition.set_value(sanitize_accessible_text(&preedit.text));
            composition.set_description(
                "Uncommitted composition; no command has been submitted.",
            );
            composition.set_read_only();
            elements.push(Element {
                key: Key { owner, item: 4096 },
                parent: Some(key),
                node: composition,
            });
        }
        (elements, key)
    }
}
