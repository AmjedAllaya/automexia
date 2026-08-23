use super::*;
use automexia_keybindings::{
    ActionDamage, ActionInvocation, ActionOutcome, ActionStatus, BindingScope,
    Consumption, ModeFlags, SequenceResolution, TableActivation,
};

impl Screen<'_> {
    pub(super) fn publish_compatibility_indicators(&mut self) {
        use crate::renderer::session_footer::CompatibilityIndicator;

        let route_ids = self.context_manager.route_ids();
        let live = route_ids
            .iter()
            .copied()
            .collect::<rustc_hash::FxHashSet<_>>();
        self.binding_states.retain(|route_id, state| {
            if live.contains(route_id) {
                true
            } else {
                let _ = state
                    .cancel(automexia_keybindings::CancellationReason::SurfaceClosed);
                false
            }
        });

        let (profile, diagnostics) = self
            .binding_registry
            .as_ref()
            .map(|snapshot| {
                let label = match snapshot.profile.requested {
                    automexia_keybindings::ProfileId::Automexia => {
                        Some("CUSTOM KEYS".to_string())
                    }
                    automexia_keybindings::ProfileId::Ghostty => {
                        Some("GHOSTTY 1.3 ↗".to_string())
                    }
                    automexia_keybindings::ProfileId::Ghostty13 => {
                        Some("GHOSTTY 1.3".to_string())
                    }
                };
                (label, snapshot.diagnostics.len())
            })
            .unwrap_or((None, 0));
        let current_route = self.context_manager.current().route_id;
        let zoomed = self.context_manager.is_split_zoomed();
        let indicators = route_ids
            .into_iter()
            .map(|route_id| {
                let state = self.binding_states.get(&route_id);
                let table = state.and_then(|state| {
                    state.active_tables().next_back().map(|(name, one_shot)| {
                        if one_shot {
                            format!("{name} · 1×")
                        } else {
                            name.to_string()
                        }
                    })
                });
                (
                    route_id,
                    CompatibilityIndicator {
                        profile: profile.clone(),
                        pending: state
                            .is_some_and(|state| !state.pending_bytes().is_empty()),
                        table,
                        diagnostics,
                        zoomed: zoomed && route_id == current_route,
                    },
                )
            })
            .collect();
        self.renderer
            .session_footer
            .replace_compatibility_indicators(indicators);
        self.publish_compatibility_inspector();
    }

    fn publish_compatibility_inspector(&mut self) {
        use crate::renderer::compatibility_inspector::InspectorSnapshot;

        if !self.renderer.compatibility_inspector.is_active() {
            return;
        }
        let route_id = self.context_manager.current().route_id;
        let state = self.binding_states.get(&route_id);
        let pending_bytes = state.map_or(0, |state| state.pending_bytes().len());
        let active_table = state
            .and_then(|state| state.active_tables().next_back())
            .map(|(name, one_shot)| {
                if one_shot {
                    format!("{name} (one-shot)")
                } else {
                    name.to_string()
                }
            })
            .unwrap_or_else(|| "default".into());
        let mut parser_diagnostics = self
            .binding_registry
            .as_ref()
            .into_iter()
            .flat_map(|snapshot| snapshot.diagnostics.iter())
            .map(|diagnostic| {
                format!("binding_{:?}", diagnostic.code).to_ascii_lowercase()
            })
            .take(7)
            .collect::<Vec<_>>();
        if let Some(reason) = state.and_then(|state| state.last_cancellation()) {
            parser_diagnostics.push(format!("sequence_{reason:?}").to_ascii_lowercase());
        }
        parser_diagnostics.truncate(8);
        let (last_binding, binding_origin) = self
            .last_compatibility_bindings
            .get(&route_id)
            .map(|(binding, origin)| (binding.clone(), format!("{origin:?}")))
            .unwrap_or_else(|| ("none".into(), "none".into()));
        let profile = self.binding_registry.as_ref().map_or_else(
            || "automexia".into(),
            |snapshot| snapshot.profile.requested.to_string(),
        );
        let context = self.context_manager.current();
        let terminal = context.terminal.lock();
        let snapshot = InspectorSnapshot {
            route_id,
            session_id: context.environment_capsule.session_id.get(),
            columns: terminal.columns(),
            rows: terminal.screen_lines(),
            viewport_offset: terminal.display_offset(),
            history_lines: terminal.history_size(),
            terminal_modes: format!("{:?}", terminal.mode()),
            keyboard_modes: format!("{:?}", terminal.keyboard_mode()),
            profile,
            binding_origin,
            last_binding,
            pending_bytes,
            active_table,
            parser_diagnostics,
        };
        drop(terminal);
        self.renderer
            .compatibility_inspector
            .replace_snapshot(snapshot);
    }
    /// Resolve an opt-in typed binding before the legacy Automexia table.
    ///
    /// Returning `false` deliberately falls through to the legacy table and
    /// then to ordinary PTY encoding. The default Automexia profile therefore
    /// remains behavior-identical when no typed user entries are configured.
    pub(super) fn process_compatibility_key_binding(
        &mut self,
        key: &rio_window::event::KeyEvent,
        mode: Mode,
        modifiers: ModifiersState,
        clipboard: &mut Clipboard,
    ) -> bool {
        let Some(snapshot) = self.binding_registry.clone() else {
            return false;
        };
        let text = key.text_with_all_modifiers().unwrap_or_default();
        let encoded = self.encode_pressed_key_event(key, text, mode, modifiers);
        let event = crate::bindings::registry::RegistryKeyEvent::from_window_event(
            key, modifiers,
        );
        let active_modes = compatibility_mode_flags(mode, self.search_active());
        let route_id = self.context_manager.current().route_id;

        let (resolution, candidates) = {
            let state = self.binding_states.entry(route_id).or_default();
            let resolution = state.resolve_event(
                &snapshot.registry,
                &event.as_normalized(),
                &encoded,
                active_modes,
            );
            let candidates = state
                .matching_bindings(&snapshot.registry, &resolution, active_modes)
                .cloned()
                .collect::<Vec<_>>();
            (resolution, candidates)
        };

        match &resolution {
            SequenceResolution::NoMatch => {
                if snapshot.suppresses_legacy(&event.as_normalized(), active_modes) {
                    // An explicit typed unbind removes the lower legacy owner,
                    // not the terminal's input. Forward the exact normal key
                    // encoding and stop before the classic shortcut scan.
                    if !encoded.is_empty() {
                        self.send_compatibility_bytes(route_id, encoded);
                    }
                    return true;
                }
                return false;
            }
            SequenceResolution::Pending => {
                self.mark_dirty();
                return true;
            }
            SequenceResolution::Flush { bytes, .. } => {
                self.send_compatibility_bytes(route_id, bytes.clone());
                self.mark_dirty();
                return true;
            }
            SequenceResolution::Matched { .. } => {}
        }

        let prefix_bytes = match &resolution {
            SequenceResolution::Matched { prefix_bytes, .. } => prefix_bytes.clone(),
            _ => Vec::new(),
        };

        for binding in candidates {
            if !binding
                .actions
                .iter()
                .all(|action| self.can_perform_compatibility_action(action))
            {
                continue;
            }

            let all_surface_routes = (binding.scope == BindingScope::AllSurfaces)
                .then(|| self.context_manager.route_ids());
            let mut chain = ActionOutcome::performed(false, ActionDamage::default());
            for action in &binding.actions {
                let outcome = if let Some(route_ids) = &all_surface_routes {
                    self.execute_all_surface_compatibility_action(action, route_ids)
                } else {
                    self.execute_compatibility_action(
                        action,
                        route_id,
                        &prefix_bytes,
                        clipboard,
                    )
                };
                chain.merge_chain(outcome);
                if matches!(
                    chain.status,
                    ActionStatus::Unavailable | ActionStatus::Failed
                ) {
                    break;
                }
            }

            if chain.status == ActionStatus::Unavailable && binding.policy.performable {
                continue;
            }

            let performed = chain.status == ActionStatus::Performed;
            if performed {
                let targets = all_surface_routes
                    .as_deref()
                    .unwrap_or(std::slice::from_ref(&route_id));
                for target in targets {
                    self.last_compatibility_bindings
                        .insert(*target, (binding.trigger_label(), binding.origin));
                }
            }
            self.binding_states
                .entry(route_id)
                .or_default()
                .complete_match(&resolution, performed);

            if chain.damage.needs_redraw() {
                self.mark_dirty();
            }

            if performed && binding.policy.consumption == Consumption::Unconsumed {
                if !prefix_bytes.is_empty() {
                    self.send_compatibility_bytes(route_id, prefix_bytes.clone());
                }
                return false;
            }

            return chain.consumed || performed;
        }

        // No candidate could run. A completed sequence's earlier keys were
        // withheld from the PTY, so release them now; the completing chord
        // falls through to the ordinary input path.
        if !prefix_bytes.is_empty() {
            self.send_compatibility_bytes(route_id, prefix_bytes);
        }
        false
    }

    fn send_compatibility_bytes(&mut self, route_id: usize, bytes: Vec<u8>) {
        if bytes.is_empty() {
            return;
        }
        if let Some(context) = self.context_manager.get_by_route_id(route_id) {
            context.messenger.send_write(bytes);
        }
    }

    fn can_perform_compatibility_action(&self, action: &ActionInvocation) -> bool {
        match action.id.as_str() {
            "copy_to_clipboard"
            | "search_selection"
            | "scroll_to_selection"
            | "write_selection_file" => self.has_nonempty_selection(),
            "end_search" | "navigate_search" => self.search_active(),
            "previous_tab" => {
                self.context_manager.len() > 1 && self.context_manager.current_index() > 0
            }
            "next_tab" => {
                self.context_manager.current_index() + 1 < self.context_manager.len()
            }
            "last_tab" => {
                self.context_manager.len() > 1
                    && self.context_manager.current_index() + 1
                        < self.context_manager.len()
            }
            "goto_tab" => action
                .parameter
                .as_deref()
                .and_then(|value| value.parse::<usize>().ok())
                .is_some_and(|one_based| {
                    one_based > 0
                        && one_based <= self.context_manager.len()
                        && one_based - 1 != self.context_manager.current_index()
                }),
            "goto_split" | "resize_split" => {
                self.context_manager.current_grid_len() > 1
                    && !self.context_manager.is_split_zoomed()
            }
            "toggle_split_zoom" | "equalize_splits" => {
                self.context_manager.current_grid_len() > 1
            }
            "undo" => self.context_manager.can_undo_topology(),
            "redo" => self.context_manager.can_redo_topology(),
            _ => true,
        }
    }

    fn execute_all_surface_compatibility_action(
        &mut self,
        action: &ActionInvocation,
        route_ids: &[usize],
    ) -> ActionOutcome {
        let damage = ActionDamage {
            terminal: true,
            ..ActionDamage::default()
        };
        let parameter = action.parameter.as_deref().unwrap_or_default();
        let static_bytes = match action.id.as_str() {
            "ignore" => return ActionOutcome::performed(true, ActionDamage::default()),
            "text" => Some(parameter.as_bytes().to_vec()),
            "esc" => {
                let mut bytes = vec![b'\x1b'];
                bytes.extend_from_slice(parameter.as_bytes());
                Some(bytes)
            }
            "csi" => {
                let mut bytes = b"\x1b[".to_vec();
                bytes.extend_from_slice(parameter.as_bytes());
                Some(bytes)
            }
            "reset" => Some(b"\x1bc".to_vec()),
            _ => None,
        };
        if let Some(bytes) = static_bytes {
            for route_id in route_ids.iter().copied() {
                self.send_compatibility_bytes(route_id, bytes.clone());
            }
            return ActionOutcome::performed(true, damage);
        }

        match action.id.as_str() {
            "cursor_key" => {
                let suffix = match parameter {
                    "up" => b'A',
                    "down" => b'B',
                    "right" => b'C',
                    "left" => b'D',
                    "home" => b'H',
                    "end" => b'F',
                    _ => return unavailable(false, "unsupported_cursor_key"),
                };
                for route_id in route_ids.iter().copied() {
                    let application = self
                        .context_manager
                        .get_by_route_id(route_id)
                        .is_some_and(|context| {
                            context.terminal.lock().mode().contains(Mode::APP_CURSOR)
                        });
                    self.send_compatibility_bytes(
                        route_id,
                        vec![b'\x1b', if application { b'O' } else { b'[' }, suffix],
                    );
                }
                ActionOutcome::performed(true, damage)
            }
            "clear_screen" | "clear_history" | "clear_screen_and_history" => {
                let mut performed = false;
                for route_id in route_ids.iter().copied() {
                    let Some(context) = self.context_manager.get_by_route_id(route_id)
                    else {
                        continue;
                    };
                    let mut terminal = context.terminal.lock();
                    match action.id.as_str() {
                        "clear_screen" if terminal.mode().contains(Mode::ALT_SCREEN) => {
                            continue
                        }
                        // Ghostty 1.3 defines `clear_screen` as clearing both
                        // the primary viewport and its saved scrollback.
                        "clear_screen" => terminal.clear_screen_and_history(),
                        "clear_history" => terminal.clear_saved_history(),
                        "clear_screen_and_history" => terminal.clear_screen_and_history(),
                        _ => unreachable!(),
                    }
                    performed = true;
                }
                if performed {
                    ActionOutcome::performed(true, damage)
                } else {
                    unavailable(false, "alternate_screen_active")
                }
            }
            _ => unavailable(false, "all_surface_action_unavailable"),
        }
    }
    fn execute_compatibility_action(
        &mut self,
        action: &ActionInvocation,
        route_id: usize,
        prefix_bytes: &[u8],
        clipboard: &mut Clipboard,
    ) -> ActionOutcome {
        let terminal_damage = ActionDamage {
            terminal: true,
            ..ActionDamage::default()
        };
        let layout_damage = ActionDamage {
            layout: true,
            ..ActionDamage::default()
        };
        let window_damage = ActionDamage {
            window: true,
            ..ActionDamage::default()
        };
        let configuration_damage = ActionDamage {
            configuration: true,
            ..ActionDamage::default()
        };
        let parameter = action.parameter.as_deref();

        match action.id.as_str() {
            "ignore" | "unbind" => {
                ActionOutcome::performed(true, ActionDamage::default())
            }
            "text" => {
                self.send_compatibility_bytes(
                    route_id,
                    parameter.unwrap_or_default().as_bytes().to_vec(),
                );
                ActionOutcome::performed(true, terminal_damage)
            }
            "esc" => {
                let mut bytes = vec![b'\x1b'];
                bytes.extend_from_slice(parameter.unwrap_or_default().as_bytes());
                self.send_compatibility_bytes(route_id, bytes);
                ActionOutcome::performed(true, terminal_damage)
            }
            "csi" => {
                let mut bytes = b"\x1b[".to_vec();
                bytes.extend_from_slice(parameter.unwrap_or_default().as_bytes());
                self.send_compatibility_bytes(route_id, bytes);
                ActionOutcome::performed(true, terminal_damage)
            }
            "cursor_key" => {
                let application = self.get_mode().contains(Mode::APP_CURSOR);
                let suffix = match parameter.unwrap_or_default() {
                    "up" => b'A',
                    "down" => b'B',
                    "right" => b'C',
                    "left" => b'D',
                    "home" => b'H',
                    "end" => b'F',
                    _ => return unavailable(false, "unsupported_cursor_key"),
                };
                self.send_compatibility_bytes(
                    route_id,
                    vec![b'\x1b', if application { b'O' } else { b'[' }, suffix],
                );
                ActionOutcome::performed(true, terminal_damage)
            }
            "reset" => {
                self.send_compatibility_bytes(route_id, b"\x1bc".to_vec());
                ActionOutcome::performed(true, terminal_damage)
            }
            "copy_to_clipboard" => {
                if !self.has_nonempty_selection() {
                    return unavailable(false, "selection_empty");
                }
                self.copy_selection(ClipboardType::Clipboard, clipboard);
                ActionOutcome::performed(true, terminal_damage)
            }
            "paste_from_clipboard" => {
                let content = clipboard.get(ClipboardType::Clipboard);
                if content.is_empty() {
                    return unavailable(false, "clipboard_empty");
                }
                self.paste(&content, true);
                ActionOutcome::performed(true, terminal_damage)
            }
            "paste_from_selection" => {
                let content = clipboard.get(ClipboardType::Selection);
                if content.is_empty() {
                    return unavailable(false, "selection_clipboard_empty");
                }
                self.paste(&content, true);
                ActionOutcome::performed(true, terminal_damage)
            }
            "increase_font_size" => {
                let Some(delta) = parse_font_delta(parameter) else {
                    return unavailable(false, "invalid_font_delta");
                };
                self.change_font_size_delta(delta, true);
                ActionOutcome::performed(true, layout_damage)
            }
            "decrease_font_size" => {
                let Some(delta) = parse_font_delta(parameter) else {
                    return unavailable(false, "invalid_font_delta");
                };
                self.change_font_size_delta(delta, false);
                ActionOutcome::performed(true, layout_damage)
            }
            "reset_font_size" => {
                self.change_font_size(FontSizeAction::Reset);
                ActionOutcome::performed(true, layout_damage)
            }
            "set_font_size" => {
                let Some(points) = parse_font_points(parameter) else {
                    return unavailable(false, "invalid_font_size");
                };
                self.set_compatibility_font_size(points);
                ActionOutcome::performed(true, layout_damage)
            }
            "search_selection" => {
                let selected = {
                    let terminal = self.context_manager.current().terminal.lock();
                    terminal
                        .selection_to_string()
                        .filter(|text| !text.is_empty())
                };
                let Some(selected) = selected else {
                    return unavailable(false, "selection_empty");
                };
                self.start_search(Direction::Right);
                for character in selected.chars().take(4_096) {
                    self.search_input(character);
                }
                ActionOutcome::performed(true, terminal_damage)
            }
            "navigate_search" => {
                if !self.search_active() {
                    return unavailable(false, "search_inactive");
                }
                let direction = match parameter.unwrap_or_default() {
                    "previous" | "left" | "up" => self.search_state.direction.opposite(),
                    _ => self.search_state.direction,
                };
                self.advance_search_origin(direction);
                ActionOutcome::performed(true, terminal_damage)
            }
            "start_search" => {
                self.start_search(Direction::Right);
                self.resize_top_or_bottom_line();
                ActionOutcome::performed(true, terminal_damage)
            }
            "end_search" => {
                if !self.search_active() {
                    return unavailable(false, "search_inactive");
                }
                self.cancel_search(clipboard);
                self.resize_top_or_bottom_line();
                ActionOutcome::performed(true, terminal_damage)
            }
            "clear_screen" => {
                let mut terminal = self.context_manager.current_mut().terminal.lock();
                if terminal.mode().contains(Mode::ALT_SCREEN) {
                    return unavailable(false, "alternate_screen_active");
                }
                // Ghostty 1.3's action clears both primary-screen content and
                // saved scrollback. The Automexia-only helpers below retain
                // their explicit visible-only and history-only meanings.
                terminal.clear_screen_and_history();
                drop(terminal);
                ActionOutcome::performed(true, terminal_damage)
            }
            "clear_history" => {
                let mut terminal = self.context_manager.current_mut().terminal.lock();
                terminal.clear_saved_history();
                drop(terminal);
                ActionOutcome::performed(true, terminal_damage)
            }
            "clear_screen_and_history" => {
                let mut terminal = self.context_manager.current_mut().terminal.lock();
                terminal.clear_screen_and_history();
                drop(terminal);
                ActionOutcome::performed(true, terminal_damage)
            }
            "select_all" => {
                self.select_all();
                ActionOutcome::performed(true, terminal_damage)
            }
            "adjust_selection" => {
                let motion = match parameter.unwrap_or_default() {
                    "left" => SelectionMotion::Left,
                    "right" => SelectionMotion::Right,
                    "up" => SelectionMotion::Up,
                    "down" => SelectionMotion::Down,
                    "word_left" => SelectionMotion::WordLeft,
                    "word_right" => SelectionMotion::WordRight,
                    "page_up" => SelectionMotion::PageUp,
                    "page_down" => SelectionMotion::PageDown,
                    "home" => SelectionMotion::Home,
                    "end" => SelectionMotion::End,
                    "line_start" => SelectionMotion::LineStart,
                    "line_end" => SelectionMotion::LineEnd,
                    _ => return unavailable(false, "unsupported_selection_motion"),
                };
                self.extend_selection(motion);
                ActionOutcome::performed(true, terminal_damage)
            }
            "scroll_to_top" => {
                self.compatibility_scroll(Scroll::Top);
                ActionOutcome::performed(true, terminal_damage)
            }
            "scroll_to_bottom" => {
                self.compatibility_scroll(Scroll::Bottom);
                ActionOutcome::performed(true, terminal_damage)
            }
            "scroll_page_up" => {
                self.compatibility_scroll(Scroll::PageUp);
                ActionOutcome::performed(true, terminal_damage)
            }
            "scroll_page_down" => {
                self.compatibility_scroll(Scroll::PageDown);
                ActionOutcome::performed(true, terminal_damage)
            }
            "scroll_to_row" => {
                let Some(row) = parameter.and_then(|value| value.parse::<usize>().ok())
                else {
                    return unavailable(false, "invalid_scroll_row");
                };
                let delta = {
                    let terminal = self.context_manager.current().terminal.lock();
                    ghostty_absolute_row_delta(
                        row,
                        terminal.history_size(),
                        terminal.display_offset(),
                    )
                };
                self.compatibility_scroll(Scroll::Delta(delta));
                ActionOutcome::performed(true, terminal_damage)
            }
            "scroll_page_fractional" => {
                let screen_lines = self
                    .context_manager
                    .current()
                    .terminal
                    .lock()
                    .screen_lines();
                let Some(delta) =
                    ghostty_fractional_scroll_delta(parameter, screen_lines)
                else {
                    return unavailable(false, "invalid_scroll_fraction");
                };
                self.compatibility_scroll(Scroll::Delta(delta));
                ActionOutcome::performed(true, terminal_damage)
            }
            "scroll_page_lines" => {
                let Some(lines) = parameter.and_then(|value| value.parse::<i32>().ok())
                else {
                    return unavailable(false, "invalid_scroll_lines");
                };
                self.compatibility_scroll(Scroll::Delta(ghostty_relative_scroll_delta(
                    lines,
                )));
                ActionOutcome::performed(true, terminal_damage)
            }
            "scroll_to_selection" => {
                let point = {
                    let terminal = self.context_manager.current().terminal.lock();
                    terminal.selection.as_ref().and_then(|selection| {
                        selection.to_range(&terminal).map(|range| range.start)
                    })
                };
                let Some(point) = point else {
                    return unavailable(false, "selection_empty");
                };
                let mut terminal = self.context_manager.current_mut().terminal.lock();
                terminal.scroll_to_pos(point);
                drop(terminal);
                ActionOutcome::performed(true, terminal_damage)
            }
            "jump_to_prompt" => {
                let forward = parameter
                    .and_then(|value| value.parse::<i32>().ok())
                    .unwrap_or(1)
                    > 0;
                let mut terminal = self.context_manager.current_mut().terminal.lock();
                terminal.scroll_to_prompt(forward);
                drop(terminal);
                ActionOutcome::performed(true, terminal_damage)
            }
            "write_screen_file" => {
                self.compatibility_export(ExportScope::Visible, parameter, clipboard)
            }
            "write_scrollback_file" => {
                self.compatibility_export(ExportScope::Scrollback, parameter, clipboard)
            }
            "write_selection_file" => {
                self.compatibility_export(ExportScope::Selection, parameter, clipboard)
            }
            "new_window" => {
                self.context_manager.invalidate_topology_redo();
                self.context_manager.create_new_window();
                ActionOutcome::performed(true, window_damage)
            }
            "new_tab" => {
                self.create_tab(clipboard);
                ActionOutcome::performed(true, layout_damage)
            }
            "previous_tab" => self.compatibility_select_tab_relative(false, clipboard),
            "next_tab" => self.compatibility_select_tab_relative(true, clipboard),
            "last_tab" => {
                self.compatibility_select_tab(self.context_manager.len() - 1, clipboard)
            }
            "goto_tab" => {
                let index = parameter
                    .and_then(|value| value.parse::<usize>().ok())
                    .and_then(|value| value.checked_sub(1));
                match index {
                    Some(index) if index < self.context_manager.len() => {
                        self.compatibility_select_tab(index, clipboard)
                    }
                    _ => unavailable(false, "tab_out_of_range"),
                }
            }
            "move_tab" => {
                self.context_manager.invalidate_topology_redo();
                match parameter.unwrap_or_default() {
                    "left" | "previous" => self.context_manager.move_current_to_prev(),
                    "right" | "next" => self.context_manager.move_current_to_next(),
                    _ => return unavailable(false, "unsupported_tab_direction"),
                }
                ActionOutcome::performed(true, layout_damage)
            }
            "new_split" => {
                match parameter.unwrap_or("right") {
                    "down" | "below" => self.split_down(),
                    "right" | "auto" => self.split_right(),
                    _ => return unavailable(false, "unsupported_split_direction"),
                }
                ActionOutcome::performed(true, layout_damage)
            }
            "goto_split" => {
                let moved = match parameter.unwrap_or_default() {
                    "previous" => self.context_manager.select_prev_split_no_loop(),
                    "next" => self.context_manager.select_next_split_no_loop(),
                    "left" => self
                        .context_manager
                        .select_split_direction(crate::layout::PaneDirection::Left),
                    "right" => self
                        .context_manager
                        .select_split_direction(crate::layout::PaneDirection::Right),
                    "up" => self
                        .context_manager
                        .select_split_direction(crate::layout::PaneDirection::Up),
                    "down" => self
                        .context_manager
                        .select_split_direction(crate::layout::PaneDirection::Down),
                    _ => return unavailable(false, "unsupported_split_direction"),
                };
                if !moved {
                    return unavailable(false, "split_boundary");
                }
                self.resize_top_or_bottom_line();
                ActionOutcome::performed(true, layout_damage)
            }
            "resize_split" => {
                let Some((direction, amount)) =
                    parameter.and_then(|value| value.split_once(','))
                else {
                    return failed(true, "invalid_resize_parameter");
                };
                let amount = amount.parse::<f32>().unwrap_or_default();
                let changed = match direction {
                    "up" => self
                        .context_manager
                        .move_divider_down(amount, &mut self.sugarloaf),
                    "down" => self
                        .context_manager
                        .move_divider_up(amount, &mut self.sugarloaf),
                    "left" => self
                        .context_manager
                        .move_divider_left(amount, &mut self.sugarloaf),
                    "right" => self
                        .context_manager
                        .move_divider_right(amount, &mut self.sugarloaf),
                    _ => false,
                };
                if !changed {
                    return unavailable(false, "split_resize_boundary");
                }
                ActionOutcome::performed(true, layout_damage)
            }
            "toggle_split_zoom" => {
                if !self.context_manager.toggle_split_zoom(&mut self.sugarloaf) {
                    return unavailable(false, "split_zoom_unavailable");
                }
                self.resize_top_or_bottom_line();
                ActionOutcome::performed(true, layout_damage)
            }
            "equalize_splits" => {
                if !self.context_manager.equalize_splits(&mut self.sugarloaf) {
                    return unavailable(false, "split_equalize_unavailable");
                }
                self.resize_top_or_bottom_line();
                ActionOutcome::performed(true, layout_damage)
            }
            "undo" => {
                if !self.context_manager.undo_topology(&mut self.sugarloaf) {
                    return unavailable(false, "topology_undo_empty");
                }
                self.resize_top_or_bottom_line();
                ActionOutcome::performed(true, layout_damage)
            }
            "redo" => {
                if !self.context_manager.redo_topology(&mut self.sugarloaf) {
                    return unavailable(false, "topology_redo_empty");
                }
                self.resize_top_or_bottom_line();
                ActionOutcome::performed(true, layout_damage)
            }
            "open_config" => {
                self.context_manager.switch_to_settings();
                self.resize_top_or_bottom_line();
                ActionOutcome::performed(true, configuration_damage)
            }
            "reload_config" => {
                self.context_manager.reload_config();
                ActionOutcome::performed(true, configuration_damage)
            }
            "close_surface" => {
                self.close_split_or_tab(clipboard);
                ActionOutcome::performed(true, layout_damage)
            }
            "close_tab" => {
                self.close_window_tab(clipboard);
                ActionOutcome::performed(true, layout_damage)
            }
            "close_window" => {
                self.context_manager.close_window();
                ActionOutcome::performed(true, window_damage)
            }
            "toggle_fullscreen" => {
                self.context_manager.toggle_full_screen();
                ActionOutcome::performed(true, window_damage)
            }
            "toggle_command_palette" => {
                let enabled = !self.renderer.command_palette.is_enabled();
                self.renderer.command_palette.set_enabled(enabled);
                ActionOutcome::performed(true, window_damage)
            }
            "inspector" => {
                self.renderer
                    .compatibility_inspector
                    .set_visibility(parameter.unwrap_or("toggle"));
                self.publish_compatibility_inspector();
                ActionOutcome::performed(true, window_damage)
            }
            "toggle_quick_terminal" => {
                self.context_manager.toggle_quake();
                ActionOutcome::performed(true, window_damage)
            }
            "end_key_sequence" => {
                self.send_compatibility_bytes(route_id, prefix_bytes.to_vec());
                ActionOutcome::performed(true, terminal_damage)
            }
            "activate_key_table" | "activate_key_table_once" => {
                let Some(table) = parameter else {
                    return failed(true, "missing_key_table");
                };
                let Some(snapshot) = self.binding_registry.clone() else {
                    return unavailable(false, "registry_unavailable");
                };
                let activation = if action.id.as_str() == "activate_key_table_once" {
                    TableActivation::OneShot
                } else {
                    TableActivation::Persistent
                };
                match self
                    .binding_states
                    .entry(route_id)
                    .or_default()
                    .activate_table(&snapshot.registry, table, activation)
                {
                    Ok(()) => ActionOutcome::performed(true, window_damage),
                    Err(_) => failed(true, "key_table_activation_failed"),
                }
            }
            "deactivate_key_table" => {
                if self
                    .binding_states
                    .entry(route_id)
                    .or_default()
                    .deactivate_table()
                {
                    ActionOutcome::performed(true, window_damage)
                } else {
                    unavailable(false, "key_table_stack_empty")
                }
            }
            "deactivate_all_key_tables" => {
                self.binding_states
                    .entry(route_id)
                    .or_default()
                    .deactivate_all_tables();
                ActionOutcome::performed(true, window_damage)
            }
            "quit" => {
                self.context_manager.quit();
                ActionOutcome::performed(true, window_damage)
            }
            _ => unavailable(true, "action_adapter_unavailable"),
        }
    }

    fn compatibility_export(
        &mut self,
        scope: ExportScope,
        parameter: Option<&str>,
        clipboard: &mut Clipboard,
    ) -> ActionOutcome {
        let text = match self.compatibility_export_text(scope) {
            Ok(text) => text,
            Err(error) => return export_failed(error),
        };
        let mut parts = parameter.unwrap_or("copy,plain").split(',');
        let disposition = parts.next().unwrap_or("copy");
        let format = match parts.next().unwrap_or("plain") {
            "escape" => crate::automexia::export::ExportFormat::Escaped,
            _ => crate::automexia::export::ExportFormat::Plain,
        };
        let path = match self.export_manager.write(&text, format) {
            Ok(path) => path,
            Err(error) => return export_failed(error),
        };

        match disposition {
            "copy" => {
                clipboard.set(
                    ClipboardType::Clipboard,
                    path.to_string_lossy().into_owned(),
                );
                ActionOutcome::performed(true, ActionDamage::default())
            }
            "paste" => {
                self.paste(path.to_string_lossy().as_ref(), true);
                ActionOutcome::performed(
                    true,
                    ActionDamage {
                        terminal: true,
                        ..ActionDamage::default()
                    },
                )
            }
            "open" => match open_export_path(&path) {
                Ok(()) => ActionOutcome::performed(true, ActionDamage::default()),
                Err(()) => failed(true, "export_open_failed"),
            },
            _ => failed(true, "invalid_export_disposition"),
        }
    }

    fn compatibility_export_text(
        &self,
        scope: ExportScope,
    ) -> Result<String, crate::automexia::export::ExportErrorCode> {
        if scope == ExportScope::Selection {
            let terminal = self.context_manager.current().terminal.lock();
            let text = terminal
                .selection_to_string_bounded(crate::automexia::export::MAX_EXPORT_BYTES)
                .map_err(|_| crate::automexia::export::ExportErrorCode::CapacityExceeded)?
                .filter(|text| !text.is_empty())
                .ok_or(crate::automexia::export::ExportErrorCode::Io)?;
            return Ok(text);
        }

        // Clone only the bounded row snapshot while holding the terminal lock;
        // UTF-8 normalization and filesystem work happen after the lock drops.
        let rows = {
            let terminal = self.context_manager.current().terminal.lock();
            let columns = terminal.grid.columns();
            let top = if scope == ExportScope::Scrollback {
                terminal.grid.topmost_line()
            } else {
                Line(-(terminal.display_offset() as i32))
            };
            let bottom = if scope == ExportScope::Scrollback {
                terminal.grid.bottommost_line()
            } else {
                let visible = i32::try_from(terminal.grid.screen_lines())
                    .unwrap_or(i32::MAX)
                    .saturating_sub(1);
                std::cmp::min(
                    Line(top.0.saturating_add(visible)),
                    terminal.grid.bottommost_line(),
                )
            };
            let row_count =
                bottom.0.saturating_sub(top.0).saturating_add(1).max(0) as usize;
            if row_count.saturating_mul(columns).saturating_mul(4)
                > crate::automexia::export::MAX_EXPORT_BYTES
            {
                return Err(crate::automexia::export::ExportErrorCode::CapacityExceeded);
            }
            (0..row_count)
                .map(|offset| terminal.grid[top + offset].clone())
                .collect::<Vec<_>>()
        };

        let mut output = String::new();
        for (index, row) in rows.iter().enumerate() {
            let wraps = row.inner.last().is_some_and(|square| square.wrapline());
            let mut line = row
                .inner
                .iter()
                .map(|square| square.c())
                .collect::<String>();
            if wraps {
                while line.ends_with('\0') {
                    line.pop();
                }
            } else {
                line.truncate(line.trim_end_matches(['\0', ' ']).len());
            }
            output.push_str(&line);
            if !wraps && index + 1 < rows.len() {
                output.push('\n');
            }
            if output.len() > crate::automexia::export::MAX_EXPORT_BYTES {
                return Err(crate::automexia::export::ExportErrorCode::CapacityExceeded);
            }
        }
        Ok(output)
    }

    fn compatibility_scroll(&mut self, scroll: Scroll) {
        let current = self.context_manager.current_mut();
        let rich_text_id = current.rich_text_id;
        let mut terminal = current.terminal.lock();
        terminal.scroll_display(scroll);
        drop(terminal);
        self.renderer.scrollbar.notify_scroll(rich_text_id);
    }

    fn compatibility_select_tab_relative(
        &mut self,
        next: bool,
        clipboard: &mut Clipboard,
    ) -> ActionOutcome {
        let current = self.context_manager.current_index();
        let target = if next {
            current.checked_add(1)
        } else {
            current.checked_sub(1)
        };
        match target {
            Some(target) if target < self.context_manager.len() => {
                self.compatibility_select_tab(target, clipboard)
            }
            _ => unavailable(false, "tab_boundary"),
        }
    }

    fn compatibility_select_tab(
        &mut self,
        target: usize,
        clipboard: &mut Clipboard,
    ) -> ActionOutcome {
        if target == self.context_manager.current_index()
            || target >= self.context_manager.len()
        {
            return unavailable(false, "tab_boundary");
        }
        let old = self.context_manager.current_index();
        self.context_manager.select_tab(target);
        let new = self.context_manager.current_index();
        self.context_manager
            .switch_context_visibility(&mut self.sugarloaf, old, new);
        self.cancel_search(clipboard);
        self.resize_top_or_bottom_line();
        ActionOutcome::performed(
            true,
            ActionDamage {
                layout: true,
                ..ActionDamage::default()
            },
        )
    }

    fn change_font_size_delta(&mut self, delta: f32, increase: bool) {
        let current = self.context_manager.current().dimension.font_size;
        let target = if increase {
            current + delta
        } else {
            current - delta
        }
        .clamp(MIN_FONT_POINTS, MAX_FONT_POINTS);
        self.set_compatibility_font_size(target);
    }

    fn set_compatibility_font_size(&mut self, points: f32) {
        let dim = &mut self.context_manager.current_mut().dimension;
        if (points - dim.font_size).abs() <= f32::EPSILON {
            return;
        }
        dim.update_font_size(points);
        self.context_manager
            .current_grid_mut()
            .update_dimensions(&mut self.sugarloaf);
        self.resize_all_contexts();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExportScope {
    Visible,
    Scrollback,
    Selection,
}

fn open_export_path(path: &std::path::Path) -> Result<(), ()> {
    #[cfg(windows)]
    {
        super::shell_execute_open(path.to_string_lossy().as_ref());
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|_| ())
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|_| ())
    }
}

fn export_failed(error: crate::automexia::export::ExportErrorCode) -> ActionOutcome {
    match error {
        crate::automexia::export::ExportErrorCode::CapacityExceeded => {
            failed(true, "export_capacity_exceeded")
        }
        crate::automexia::export::ExportErrorCode::InvalidTemporaryRoot => {
            failed(true, "export_temporary_root_invalid")
        }
        crate::automexia::export::ExportErrorCode::PrivatePermissions => {
            failed(true, "export_private_permissions_failed")
        }
        crate::automexia::export::ExportErrorCode::Io => failed(true, "export_io_failed"),
    }
}

fn compatibility_mode_flags(mode: Mode, search_active: bool) -> ModeFlags {
    let mut flags = ModeFlags::empty();
    if search_active {
        flags = flags.union(ModeFlags::SEARCH);
    }
    if mode.contains(Mode::VI) {
        flags = flags.union(ModeFlags::VI);
    }
    if mode.contains(Mode::ALT_SCREEN) {
        flags = flags.union(ModeFlags::ALT_SCREEN);
    }
    if mode.contains(Mode::APP_CURSOR) {
        flags = flags.union(ModeFlags::APP_CURSOR);
    }
    if mode.contains(Mode::APP_KEYPAD) {
        flags = flags.union(ModeFlags::APP_KEYPAD);
    }
    if mode.contains(Mode::DISAMBIGUATE_ESC_CODES) {
        flags = flags.union(ModeFlags::KITTY_DISAMBIGUATE);
    }
    if mode.contains(Mode::REPORT_ALL_KEYS_AS_ESC) {
        flags = flags.union(ModeFlags::KITTY_ALL_KEYS);
    }
    flags
}

const MIN_FONT_POINTS: f32 = 6.0;
const MAX_FONT_POINTS: f32 = 100.0;

fn parse_finite_f32(parameter: Option<&str>) -> Option<f32> {
    parameter
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
}

fn parse_font_delta(parameter: Option<&str>) -> Option<f32> {
    parse_finite_f32(parameter)
        .map(|value| value.clamp(0.0, MAX_FONT_POINTS - MIN_FONT_POINTS))
}

fn parse_font_points(parameter: Option<&str>) -> Option<f32> {
    parse_finite_f32(parameter).map(|value| value.clamp(MIN_FONT_POINTS, MAX_FONT_POINTS))
}

fn ghostty_relative_scroll_delta(lines: i32) -> i32 {
    // Ghostty's public action uses positive values for scrolling down. Rio's
    // grid uses a positive display offset for scrolling up into history.
    lines.saturating_neg()
}

fn ghostty_fractional_scroll_delta(
    parameter: Option<&str>,
    screen_lines: usize,
) -> Option<i32> {
    let fraction = parse_finite_f32(parameter)?;
    let lines = (fraction * screen_lines as f32).trunc();
    let ghostty_lines = if lines >= i32::MAX as f32 {
        i32::MAX
    } else if lines <= i32::MIN as f32 {
        i32::MIN
    } else {
        lines as i32
    };
    Some(ghostty_relative_scroll_delta(ghostty_lines))
}

fn ghostty_absolute_row_delta(row: usize, history: usize, current: usize) -> i32 {
    // Ghostty numbers the complete buffer from its oldest row. Rio stores the
    // distance from the active viewport, so the coordinates are reversed.
    let target = history.saturating_sub(row.min(history));
    if target >= current {
        i32::try_from(target - current).unwrap_or(i32::MAX)
    } else {
        i32::try_from(current - target).map_or(i32::MIN, |delta| -delta)
    }
}

fn unavailable(consumed: bool, error_code: &'static str) -> ActionOutcome {
    ActionOutcome {
        status: ActionStatus::Unavailable,
        consumed,
        damage: ActionDamage::default(),
        error_code: Some(error_code),
    }
}

fn failed(consumed: bool, error_code: &'static str) -> ActionOutcome {
    ActionOutcome {
        status: ActionStatus::Failed,
        consumed,
        damage: ActionDamage::default(),
        error_code: Some(error_code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_translation_keeps_independent_predicates() {
        let mode = Mode::VI
            | Mode::ALT_SCREEN
            | Mode::APP_CURSOR
            | Mode::APP_KEYPAD
            | Mode::DISAMBIGUATE_ESC_CODES
            | Mode::REPORT_ALL_KEYS_AS_ESC;
        let flags = compatibility_mode_flags(mode, true);
        assert!(flags.contains(ModeFlags::SEARCH));
        assert!(flags.contains(ModeFlags::VI));
        assert!(flags.contains(ModeFlags::ALT_SCREEN));
        assert!(flags.contains(ModeFlags::APP_CURSOR));
        assert!(flags.contains(ModeFlags::APP_KEYPAD));
        assert!(flags.contains(ModeFlags::KITTY_DISAMBIGUATE));
        assert!(flags.contains(ModeFlags::KITTY_ALL_KEYS));
    }

    #[test]
    fn ghostty_parameter_translation_is_fractional_bounded_and_directional() {
        assert_eq!(parse_font_delta(Some("1.5")), Some(1.5));
        assert_eq!(parse_font_delta(Some("0")), Some(0.0));
        assert_eq!(parse_font_delta(Some("10000")), Some(94.0));
        assert_eq!(parse_font_delta(Some("invalid")), None);
        assert_eq!(parse_font_delta(Some("NaN")), None);

        assert_eq!(parse_font_points(Some("1")), Some(6.0));
        assert_eq!(parse_font_points(Some("13.5")), Some(13.5));
        assert_eq!(parse_font_points(Some("999")), Some(100.0));
        assert_eq!(parse_font_points(Some("-inf")), None);

        assert_eq!(ghostty_relative_scroll_delta(3), -3);
        assert_eq!(ghostty_relative_scroll_delta(-10), 10);
        assert_eq!(ghostty_relative_scroll_delta(i32::MIN), i32::MAX);
        assert_eq!(ghostty_fractional_scroll_delta(Some("0.5"), 24), Some(-12));
        assert_eq!(ghostty_fractional_scroll_delta(Some("-1.5"), 24), Some(36));
        assert_eq!(ghostty_fractional_scroll_delta(Some("NaN"), 24), None);

        assert_eq!(ghostty_absolute_row_delta(0, 100, 0), 100);
        assert_eq!(ghostty_absolute_row_delta(40, 100, 0), 60);
        assert_eq!(ghostty_absolute_row_delta(100, 100, 60), -60);
        assert_eq!(ghostty_absolute_row_delta(usize::MAX, 100, 0), 0);
    }
}
