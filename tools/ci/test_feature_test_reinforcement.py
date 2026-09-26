#!/usr/bin/env python3
"""Mutation tests for the feature test reinforcement contract."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("check_feature_test_reinforcement.py")
SPEC = importlib.util.spec_from_file_location("automexia_feature_test_reinforcement", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load feature test reinforcement checker")
REINFORCEMENT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(REINFORCEMENT)


class FeatureTestReinforcementTests(unittest.TestCase):
    def test_search_completion_must_precede_nonbinary_publication(self):
        REINFORCEMENT._validate_local_tool_sources(self.native_sources)
        source = self.native_sources["local_tools"]
        completion = "ended.iter().any(Option::is_none)"
        publication = "result.push_str(&row)"
        mutation = source.replace(completion, "ORDER_PLACEHOLDER", 1).replace(publication, completion, 1).replace("ORDER_PLACEHOLDER", publication, 1)
        with self.assertRaises(REINFORCEMENT.ReinforcementError):
            REINFORCEMENT._validate_local_tool_sources(dict(self.native_sources, local_tools=mutation))

    def test_guest_path_validation_precedes_lookup_and_cannot_be_bypassed(self):
        REINFORCEMENT._validate_local_tool_sources(self.native_sources)
        source = self.native_sources["local_session"]
        validation = 'let path = guest_tool_path(self.path.as_deref().unwrap())?;'
        lookup = 'let mut command = Command::new(super::resolve_tool("wsl")?);'
        binding = 'command.arg(format!("PATH={path}"));'
        mutations = [
            source.replace(validation, 'ORDER_PLACEHOLDER', 1).replace(lookup, validation, 1).replace('ORDER_PLACEHOLDER', lookup, 1),
            source.replace(binding, 'command.arg(format!("PATH={}", self.path.as_deref().unwrap()));', 1),
            source.replace(binding, binding + '\ncommand.arg(format!("PATH={}", self.path.as_deref().unwrap()));', 1),
        ]
        for mutation in mutations:
            with self.subTest(mutation=mutations.index(mutation)):
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_local_tool_sources(dict(self.native_sources, local_session=mutation))

    def test_editor_disable_precedes_override_and_unc_identity_is_retained(self):
        REINFORCEMENT._validate_local_tool_sources(self.native_sources)
        first = "configured.scheme()?;"
        second = "selected.scheme()?;"
        source = self.native_sources["editor"]
        mutations = [source.replace(first, "POLICY_PLACEHOLDER", 1).replace(second, first, 1).replace("POLICY_PLACEHOLDER", second, 1),
                     source.replace("uri.insert(boundary,", "removed_unc_boundary(", 1)]
        for mutation in mutations:
            sources = dict(self.native_sources, editor=mutation)
            with self.assertRaises(REINFORCEMENT.ReinforcementError):
                REINFORCEMENT._validate_local_tool_sources(sources)

    def test_native_member_identity_must_be_pinned_before_termination(self):
        sources = self.native_sources.copy()
        source = sources["cli_process"]
        pin = "self.completion.pin_members()"
        kill = "self.inner.start_kill()"
        self.assertIn(pin, source)
        self.assertIn(kill, source)
        sources["cli_process"] = source.replace(pin, "PIN_PLACEHOLDER", 1).replace(kill, pin, 1).replace("PIN_PLACEHOLDER", kill, 1)
        with self.assertRaises(REINFORCEMENT.ReinforcementError):
            REINFORCEMENT._validate_local_tool_sources(sources)

    def test_local_tool_limits_directory_handoff_native_cancellation_and_guest_import_isolation_cannot_disappear(self):
        REINFORCEMENT._validate_local_tool_sources(self.native_sources)
        for owner, fragments in REINFORCEMENT.LOCAL_TOOL_CONTRACTS.items():
            for fragment in fragments:
                with self.subTest(owner=owner, contract=fragment):
                    sources = self.native_sources.copy()
                    self.assertIn(fragment, sources[owner])
                    sources[owner] = sources[owner].replace(fragment, "removed contract")
                    with self.assertRaises(REINFORCEMENT.ReinforcementError):
                        REINFORCEMENT._validate_local_tool_sources(sources)

    def test_google_encoding_privacy_limits_and_real_dispatch_cannot_disappear(self) -> None:
        REINFORCEMENT._validate_google_sources(self.native_sources)
        for owner, fragment in (
            ("google", "MAX_QUERY_BYTES: usize = 4096"),
            ("google", "argument.chars().any(char::is_control)"),
            ("google", 'append_pair("q", &query)'),
            ("google", "if command.print_url"),
            ("browser_search", "super::google::validated_query(arguments)?"),
            ("browser_search", 'append_pair("as_sitesearch", site)'),
            ("browser_search", "if command.search.print_url"),
            ("browser_search", "fn amx_browser_search_benchmark_checked_routing()"),
            ("google_native", "def test_real_search_and_docs_routes_are_offline_and_bounded(self):"),
            ("cli_main", "automexia::browser_search::execute(command, true)"),
            ("google_native", "timeout_seconds=25"),
            ("google_native", "actual == expected"),
            ("session_cli", '"AUTOMEXIA_CLI/up"'),
            ("desktop_open", "with_com_apartment(|| open_windows(target))"),
            ("desktop_open", "impl Drop for ApartmentGuard"),
            ("desktop_open", "if result >= 0"),
            ("xtask", "smoke_google_command()?;"),
            ("screen", "crate::automexia::desktop_open::open(target)"),
        ):
            with self.subTest(owner=owner, contract=fragment):
                sources = self.native_sources.copy()
                self.assertIn(fragment, sources[owner])
                sources[owner] = sources[owner].replace(fragment, "removed contract")
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_google_sources(sources)

    def test_hyperlink_capture_intent_pixels_and_privacy_guards_cannot_disappear(self) -> None:
        REINFORCEMENT._validate_hyperlink_sources(self.native_sources)
        for owner, fragment in (
            ("hyperlinks", "MAX_HINT_MATCHES: usize = 256"),
            ("hyperlinks", "if !pressed || repeat"),
            ("hyperlinks", "if label.len() > columns"),
            ("hyperlinks", "occupied.contains(&(target.start.row.0, col))"),
            ("hyperlink_preview", "state.focused_label()"),
            ("screen", ".label_cells()"),
            ("hyperlink_tests", "fn keyboard_links_right_edge_labels_are_whole_and_do_not_overlap()"),
            ("hyperlink_tests", "assert_eq!(state.label_cells().len(), 300)"),
            ("hyperlink_scan", "self.dimensions !="),
            ("hyperlink_scan", "grid.extras_table.get(*id) != Some(value)"),
            ("hyperlink_scan", "limits.set_retry_limit_in_match(10_000)"),
            ("hyperlink_scan", "previous_link == &link"),
            ("hyperlink_scan", "usize::from(cell.is_wide())"),
            ("hyperlink_scan", "end: positions[start + matched.len() - 1].1"),
            ("hyperlink_tests", "fn keyboard_links_cover_both_cells_of_final_wide_grapheme()"),
            ("hyperlink_tests", "fn keyboard_links_label_navigation_resets_destination_pan()"),
            ("hyperlink_preview", "text_fit::fit_end"),
            ("hyperlink_tests", "assert_eq!(original, preview_pixels(&state, 760, 180))"),
            ("hyperlink_tests", "damaged[68 * 760] ^= 1"),
            ("hyperlink_tests", "assert_eq!(state.matches().len(), 100)"),
            ("screen", "if !self.validate_hint_snapshot()"),
            ("screen", "self.remember_hint_key(key)"),
            ("application", ".handle_hint_window_event(&event, &mut self.router.clipboard)"),
        ):
            with self.subTest(owner=owner, contract=fragment):
                sources = self.native_sources.copy()
                self.assertIn(fragment, sources[owner])
                sources[owner] = sources[owner].replace(fragment, "removed contract")
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_hyperlink_sources(sources)
        for disclosure in ('args {:?}', 'could not open {target}'):
            sources = self.native_sources.copy()
            sources["screen"] += disclosure
            with self.assertRaises(REINFORCEMENT.ReinforcementError):
                REINFORCEMENT._validate_hyperlink_sources(sources)

    def test_core_table_capture_input_pixels_and_benchmarks_cannot_disappear(self) -> None:
        REINFORCEMENT._validate_table_sources(self.native_sources)
        for owner, fragment in (
            ("table_model", "MAX_TABLE_BYTES: usize = 256 * 1024"),
            ("table_capture", ".bounds_to_display_string_bounded("),
            ("table_view", "WindowEvent::Ime(_)"),
            ("table_view", "self.viewport.horizontal_thumb("),
            ("table_pixels", "assert_eq!(pixels, after.pixels(760, 260));"),
            ("table_pixels", "line.0.y += 1.0;"),
            ("screen", "self.consume_overlay_key_release(event)"),
            ("screen", "self.handle_settings_key(key, clipboard)"),
            ("screen_settings", "self.consume_overlay_key_release(key)"),
            ("screen_settings", "self.dispatch_settings_key(key, clipboard, true)"),
            ("application_bench", "    core_table_view,"),
        ):
            with self.subTest(owner=owner, contract=fragment):
                sources = self.native_sources.copy()
                self.assertIn(fragment, sources[owner])
                sources[owner] = sources[owner].replace(fragment, "removed contract")
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_table_sources(sources)

    def test_shared_modal_release_must_precede_settings_dispatch(self) -> None:
        REINFORCEMENT._validate_table_sources(self.native_sources)
        owner = self.native_sources["screen_settings"]
        release = "self.consume_overlay_key_release(key)"
        dispatch = "self.dispatch_settings_key(key, clipboard, true)"
        self.assertIn(release, owner)
        self.assertIn(dispatch, owner)
        mutation = owner.replace(release, "ORDER_PLACEHOLDER", 1).replace(dispatch, release, 1).replace("ORDER_PLACEHOLDER", dispatch, 1)
        with self.assertRaises(REINFORCEMENT.ReinforcementError):
            REINFORCEMENT._validate_table_sources(dict(self.native_sources, screen_settings=mutation))

    def test_inline_style_owner_chain_accepts_live_refactored_source(self) -> None:
        REINFORCEMENT._validate_inline_table_sources(self.native_sources)
        self.assertEqual(
            REINFORCEMENT.NATIVE_CONTRACT_SOURCES["inline_pipeline"],
            "apps/automexia-terminal/src/automexia/inline_pipeline.rs",
        )

    def test_inline_style_owner_chain_preserves_formatted_equivalents(self) -> None:
        for newline in ("\n", "\r\n"):
            source = self.native_sources["inline_pipeline"].replace("\r\n", "\n")
            self.assertIn("StyleFlags::STRIKEOUT | StyleFlags::ALL_UNDERLINES", source)
            source = source.replace("StyleFlags::STRIKEOUT | StyleFlags::ALL_UNDERLINES",
                                    "StyleFlags::STRIKEOUT\n                            | StyleFlags::ALL_UNDERLINES")
            if newline == "\r\n":
                source = source.replace("\n", "\r\n")
            with self.subTest(newline=repr(newline)):
                REINFORCEMENT._validate_inline_style_source_chain(dict(self.native_sources, inline_pipeline=source))

    def test_inline_style_owner_chain_rejects_guard_mutations(self) -> None:
        original = self.native_sources["inline_pipeline"]
        flags = "StyleFlags::STRIKEOUT | StyleFlags::ALL_UNDERLINES"
        self.assertIn(flags, original)
        replacements = (
            (flags, "StyleFlags::STRIKEOUT"),
            (flags, "StyleFlags::ALL_UNDERLINES"),
            (flags, "StyleFlags::STRIKEOUT & StyleFlags::ALL_UNDERLINES"),
            ("if source.iter().any(|line| {", "if !source.iter().any(|line| {"),
            ("line.styles.iter().any(|(_, style)| {", "line.styles.iter().all(|(_, style)| {"),
            ("Some(InlineFallbackReason::UnsupportedStyle);", "Some(InlineFallbackReason::NotTable);"),
            ("style.flags.intersects(", "style.flags.contains("),
        )
        for old, new in replacements:
            self.assertIn(old, original)
            changed = original.replace(old, new, 1)
            with self.subTest(old=old, new=new):
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_inline_style_source_chain(dict(self.native_sources, inline_pipeline=changed))
        first = original.index("                if source.iter().any(|line| {")
        end = original.index("                let Some(table) = pipeline_detect(", first)
        guard = original[first:end]
        self.assertIn("continue;", guard)
        variants = {
            "removed": original[:first] + original[end:],
            "no-continue": original[:first] + guard.replace("continue;", "", 1) + original[end:],
            "comment-only": original[:first] + "/*" + guard + "*/\n" + original[end:],
            "literal-only": original[:first] + "let unused = " + json.dumps(guard) + ";\n" + original[end:],
            "outside-refresh": original[:first] + original[end:] + "\nfn unrelated() {\n" + guard + "}\n",
        }
        without = original[:first] + original[end:]
        insert = without.index("                let layout = match table.wrap(", first)
        variants["after-detection"] = without[:insert] + guard + without[insert:]
        for kind, changed in variants.items():
            with self.subTest(kind=kind):
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_inline_style_source_chain(dict(self.native_sources, inline_pipeline=changed))

    def test_inline_style_owner_chain_rejects_disconnected_dispatch(self) -> None:
        source = self.native_sources["inline_capture"]
        mutations = (
            ('include!("inline_pipeline.rs");', ''),
            ('include!("inline_pipeline.rs");', '// include!("inline_pipeline.rs");'),
            ('include!("inline_pipeline.rs");', '#[cfg(test)]\ninclude!("inline_pipeline.rs");'),
            ('self.refresh_pipeline(snapshot)', 'false'),
            ('self.refresh_pipeline(snapshot)', 'self.other_pipeline(snapshot)'),
        )
        for old, new in mutations:
            self.assertIn(old, source)
            with self.subTest(new=new):
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_inline_style_source_chain(dict(self.native_sources, inline_capture=source.replace(old, new, 1)))
        missing = dict(self.native_sources)
        del missing["inline_pipeline"]
        with self.assertRaises(REINFORCEMENT.ReinforcementError):
            REINFORCEMENT._validate_inline_style_source_chain(missing)

    def test_inline_tables_keep_bounded_capture_projection_and_glyph_evidence(self) -> None:
        REINFORCEMENT._validate_inline_table_sources(self.native_sources)
        for owner, fragment in (
            ("inline_capture", "MAX_SCAN_CELLS: usize = 64 * 1024"),
            ("inline_capture", "pub fn source_position"),
            ("inline_renderer", "canvas.text().draw_cells_clipped("),
            ("inline_renderer", "selection.contains(cell)"),
            ("inline_pixels", "fn inline_table_visual_scroll_round_trip_preserves_partial_soft_wrapped_rows()"),
            ("text_clip_tests", "fn cell_clipping_contains_real_glyph_ink_at_fractional_scales()"),
            ("renderer", ".needs_snapshot(&*terminal)"),
            ("command_wrap_renderer", "content.inline_tables.bands()"),
            ("screen", "self.native_mouse_position(display_offset)"),
            ("application_bench", '"inline_wrap_checked"'),
        ):
            with self.subTest(owner=owner, contract=fragment):
                sources = self.native_sources.copy()
                self.assertIn(fragment, sources[owner])
                sources[owner] = sources[owner].replace(fragment, "removed contract")
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_inline_table_sources(sources)

    def test_command_wrapping_scenarios_remain_cross_owner(self) -> None:
        for feature_id in REINFORCEMENT.WRAPPING_FEATURES:
            for field, phrase in (
                ("needed_tests", "Shared command-information wrapping"),
                ("verification_reinforcements", "actual glyph pixels, complete label bytes"),
                ("checker_reinforcements", "Reject loss of shared wrapping dispatch"),
            ):
                with self.subTest(feature=feature_id, field=field):
                    document = copy.deepcopy(self.document)
                    feature = next(row for row in document["features"] if row["id"] == feature_id)
                    feature[field] = [value.replace(phrase, "removed evidence") for value in feature[field]]
                    with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "required scenario detail"):
                        self.validate(document)

    def test_command_wrapping_dispatch_pixels_and_benchmarks_cannot_disappear(self) -> None:
        REINFORCEMENT._validate_command_wrapping_sources(self.native_sources)
        for owner, old, new in (
            ("renderer", "command_info::layout(", "skip_command_layout("),
            ("renderer", "command_info::project_images(", "skip_image_projection("),
            ("screen", "p.command_rows.source_row(y)", "Some(y)"),
            ("screen", ".native_row(position.row.0.max(0) as usize)", ".visual_row(position.row.0.max(0) as usize)"),
            ("command_wrap_model", "remaining.grapheme_indices(true)", "remaining.char_indices()"),
            ("command_wrap_model", "assert!(previous.is_none_or(|previous| source > previous));", "assert!(true);"),
            ("command_wrap_renderer", "projected_grid_pixels(&content, width, height)", "vec![0; 32]"),
            ("command_wrap_renderer", "text.render_cpu_modal(&mut pixels, width, height);", "text.instance_count();"),
            ("command_wrap_renderer", "assert_eq!(restored, completion.text);", "assert!(!restored.is_empty());"),
            ("command_wrap_renderer", "pixels.iter().zip(original).filter(|(a, b)| a != b).count(),\n                        0,", "pixels.len(), pixels.len(),"),
            ("application_bench", "    command_information\n);", ");"),
            ("application_bench", "assert_eq!(ends, values.map(str::len));", "assert!(true);"),
            ("application_bench", "assert_eq!(projection.origin(1), band.rows);", "assert!(true);"),
        ):
            with self.subTest(owner=owner, mutation=old):
                sources = self.native_sources.copy()
                self.assertIn(old, sources[owner])
                sources[owner] = sources[owner].replace(old, new)
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_command_wrapping_sources(sources)

    def test_colour_setup_scenarios_cannot_be_removed(self) -> None:
        for field, phrase in (
            ("needed_tests", "Unicode colour digit panic"),
            ("needed_tests", "exact RGB/RGBA"),
            ("verification_reinforcements", "first-use zero-allocation palette"),
            ("verification_reinforcements", "same-host palette setup"),
            ("checker_reinforcements", "colour grammar and allocation evidence"),
        ):
            with self.subTest(phrase=phrase):
                document = copy.deepcopy(self.document)
                feature = document["features"][0]
                feature[field] = [text.replace(phrase, "removed evidence") for text in feature[field]]
                with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "required scenario detail"):
                    self.validate(document)

    def test_search_scoring_and_allocation_scenarios_cannot_be_removed(self) -> None:
        for field, phrase in (
            ('needed_tests', 'Independent scalar score oracle'),
            ('needed_tests', 'contextual Unicode lowercase'),
            ('verification_reinforcements', 'maximum-input scoring allocation ceiling'),
            ('verification_reinforcements', 'allocating canary and unwind reset'),
            ('checker_reinforcements', 'scalar-score and allocation evidence'),
        ):
            with self.subTest(phrase=phrase):
                document = copy.deepcopy(self.document)
                feature = next(row for row in document['features'] if row['id'] == 'command-productivity-cp22-quick-actions')
                feature[field] = [text.replace(phrase, 'removed evidence') for text in feature[field]]
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    self.validate(document)

    def test_qa_admission_cleanup_and_consumers_cannot_drift(self) -> None:
        REINFORCEMENT._validate_qa_process_sources(self.native_sources)
        for owner, old, new in (
            ('qa_process', 'job.assign(process)', 'pass'),
            ('qa_process', "process.stdin.write(b'G')", 'pass'),
            ('qa_process', 'reader.join(timeout=', 'reader.join_without_deadline(timeout='),
            ('qa_process', 'os.killpg(process.pid, signal.SIGKILL)', 'process.kill()'),
            ('qa_process', '_quarantine = (process, job, readers)', '_quarantine = None'),
            ('qa_process', 'CLEANUP_SECONDS = 5.0', 'CLEANUP_SECONDS = 500.0'),
            ('qa_process', '0x2000', '0'),
            ('qa', 'qa_process.run(', 'other_owner.run('),
            ('compiler_probe', 'qa_process.run(', 'other_owner.run('),
        ):
            with self.subTest(owner=owner, mutation=old):
                sources = self.native_sources.copy()
                self.assertIn(old, sources[owner])
                sources[owner] = sources[owner].replace(old, new, 1)
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_qa_process_sources(sources)

    def test_confirmed_child_exit_owner_and_order_cannot_drift(self) -> None:
        REINFORCEMENT._validate_child_exit_sources(self.native_sources)
        for old, new in [
            ("&& self.finish_child_exit(&mut state, &mut buf)", "&& false"),
            ("self.pty.next_child_event()", "None"),
            ("self.finish_child_exit(&mut state, &mut buf);", "// omitted"),
            ("RioEvent::ChildExited(self.route_id, status)", "RioEvent::Render"),
            ("self.terminal.lock().exit();", "// omitted"),
            ("remaining > 0 && !self.sender.shutdown_requested()", "true"),
            ("4 * READ_BUFFER_SIZE", "8 * READ_BUFFER_SIZE"),
            ("buf.len().min(byte_limit - processed)", "buf.len()"),
        ]:
            with self.subTest(mutation=old):
                sources = self.native_sources.copy()
                self.assertIn(old, sources["pty_worker"])
                sources["pty_worker"] = sources["pty_worker"].replace(old, new, 1)
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_child_exit_sources(sources)

    def test_buffered_native_probe_receipt_cannot_be_removed(self) -> None:
        document = copy.deepcopy(self.document)
        feature = next(row for row in document['features'] if row['id'] == 'pty-scheduler-process-lifecycle')
        feature['needed_tests'] = [text.replace('Buffered native probes verify every consumed key value and ordinal', 'removed evidence') for text in feature['needed_tests']]
        with self.assertRaises(REINFORCEMENT.ReinforcementError):
            self.validate(document)

    def test_shortcut_cross_owner_guards_cannot_be_deleted_or_reordered(self) -> None:
        for owner, old, new in [
            ("screen", "if self.renderer.command_palette.is_enabled() {", "if false {"),
            ("application", ".update_bindings(&self.config, snapshot.clone())", ".update_config(&self.config, snapshot.clone())"),
            ("shortcut_preferences", "!state.write_failed", "state.last_error.is_none()"),
            ("shortcut_preferences", "state.completed = Some((revision, result));", "// state.completed = Some((revision, result));"),
            ("shortcut_editor", "revision >= expected", "revision <= expected"),
            ("palette", "self.shortcut_change.take().is_some()", "self.shortcut_change.is_some()"),
        ]:
            with self.subTest(owner=owner, mutation=old):
                sources = self.native_sources.copy()
                self.assertIn(old, sources[owner])
                sources[owner] = sources[owner].replace(old, new)
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_shortcut_editor_sources(sources)

    def test_shortcut_editor_scenarios_cannot_be_removed(self) -> None:
        for phrase in ["Shortcut editor double-click and F2", "stale queued edits", "separate-process restart"]:
            with self.subTest(phrase=phrase):
                document = copy.deepcopy(self.document)
                feature = next(row for row in document["features"] if row["id"] == "windows-tabs-sessions-input")
                feature["needed_tests"] = [text.replace(phrase, "removed evidence") for text in feature["needed_tests"]]
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    self.validate(document)

    def test_resize_stress_cannot_hide_native_suites_behind_the_gui_flag(self) -> None:
        for old, new in [
            ('run_resize_regressions(|args| run("cargo", args))?;', ''),
            ('run_resize_regressions(|args| run("cargo", args))?;',
             '// run_resize_regressions(|args| run("cargo", args))?;'),
            ('run_resize_regressions(|args| run("cargo", args))?;',
             'if false { run_resize_regressions(|args| run("cargo", args))?; }'),
            ('run_resize_regressions(|args| run("cargo", args))?;',
             'run_resize_regressions(|args| run("cargo", args)).ok();'),
            ('"live_resize",', ''),
            ('"resize_repaint",', '"resize_repaint", "resize_repaint",'),
            ('"pane_editor_resize",', '"--ignored", "pane_editor_resize",'),
        ]:
            with self.subTest(mutation=old):
                sources = self.native_sources.copy()
                prefix, suffix = sources["xtask"].split("fn run_resize_regressions(", 1)
                self.assertIn(old, suffix)
                sources["xtask"] = prefix + "fn run_resize_regressions(" + suffix.replace(old, new, 1)
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_native_contract_sources(sources)

    def test_status_colour_evidence_cannot_be_replaced_by_command_success(self) -> None:
        for phrase in ["Lifecycle versus readiness", "condition polarity", "mixed failure counts", "parser-to-grid status colours", "kind-prefixed pods", "zero-count log prefixes", "literal status-colour oracles", "explicit ANSI", "disabled extension", "lifecycle/readiness semantics"]:
            with self.subTest(phrase=phrase):
                document = copy.deepcopy(self.document)
                feature = next(row for row in document["features"] if row["id"] == "prompt-context-devops-semantics")
                for field in ["needed_tests", "verification_reinforcements", "checker_reinforcements"]:
                    feature[field] = [text.replace(phrase, "removed assurance") for text in feature[field]]
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    self.validate(document)

    def test_configuration_batch_and_creation_scenarios_cannot_disappear(self) -> None:
        for phrase in ["Whole-batch environment rejection", "every platform override", "Concurrent no-overwrite publication", "isolated fixture ownership"]:
            with self.subTest(phrase=phrase):
                document = copy.deepcopy(self.document)
                feature = next(row for row in document["features"] if row["id"] == "identity-config-migration")
                self.assertTrue(any(phrase in text for text in feature["needed_tests"]))
                feature["needed_tests"] = [text.replace(phrase, "removed assurance") for text in feature["needed_tests"]]
                with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "required scenario detail"):
                    self.validate(document)

    def test_cache_admission_and_streaming_scenarios_cannot_disappear(self) -> None:
        for phrase in ["native child admission during deletion", "retained named locks", "iterator closure", "128-lease overflow"]:
            with self.subTest(phrase=phrase):
                document = copy.deepcopy(self.document)
                feature = next(row for row in document["features"] if row["id"] == "contributor-automation-quality-policy")
                feature["needed_tests"] = [text.replace(phrase, "removed assurance") for text in feature["needed_tests"]]
                with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "required scenario detail"):
                    self.validate(document)

    def test_windows_pipe_storage_scenarios_cannot_disappear(self) -> None:
        for phrase in ["Windows pipe storage", "independent FIFO model", "bounded cross-thread publication", "joined endpoint drops"]:
            with self.subTest(phrase=phrase):
                document = copy.deepcopy(self.document)
                feature = next(row for row in document["features"] if row["id"] == "pty-scheduler-process-lifecycle")
                feature["needed_tests"] = [text.replace(phrase, "removed assurance") for text in feature["needed_tests"]]
                with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "required scenario detail"):
                    self.validate(document)

    def test_native_editor_cursor_and_deadline_scenarios_cannot_disappear(self) -> None:
        for owner, phrases in {
            "terminal-protocols-grid-history": ["Real ConsoleHost editor", "native protocol cursor", "Long padded table rows", "No-resize probe invariance", "Real WSL multi-column listings", "exact-margin cursor cell", "blank soft-wrap fragments"],
            "pty-scheduler-process-lifecycle": ["fake-clock input-settle deadlines", "buffered-input ordering", "broken-pipe error-then-drop", "Caller-handle recovery", "Final-output lock contention"],
        }.items():
            for phrase in phrases:
                with self.subTest(owner=owner, phrase=phrase):
                    document = copy.deepcopy(self.document)
                    feature = next(row for row in document["features"] if row["id"] == owner)
                    feature["needed_tests"] = [text.replace(phrase, "removed assurance") for text in feature["needed_tests"]]
                    with self.assertRaises(REINFORCEMENT.ReinforcementError):
                        self.validate(document)

    def test_listing_fixture_oracles_cleanup_and_benchmark_cannot_disappear(self) -> None:
        REINFORCEMENT._validate_resize_listing_sources(self.native_sources)
        for owner, old, new in (
            ("resize_listing_tests", "for height in [8, 2]", "for height in [8]"),
            ("resize_listing_tests", "run_worker_output_sizes(", "skip_worker_output_sizes("),
            ("resize_listing_tests", ".close()", ".keep()"),
            ("resize_listing_tests", "worker probe cannot repair output", "removed baseline"),
            ("resize_listing_tests", 'text.matches(&format!("entry-{index:02}-")).count(),\n                1,', 'text.matches(&format!("entry-{index:02}-")).count(),\n                2,'),
            ("resize_listing_tests", "for seed in 1..=8u64", "for seed in 1..=0u64"),
            ("resize_listing_tests", "(512, 96)", "(100, 24)"),
            ("resize_listing_tests", "(1, 1)", "(2, 2)"),
            ("resize_listing_tests", "std::fs::create_dir(path)", "skip_directory(path)"),
            ("resize_listing_fixture", "eza --icons=always", "printf --icons=always"),
            ("resize_listing_fixture", "steps=48", "steps=1"),
            ("resize_bench", "terminal.grid.history_size() < 64", "true"),
            ("resize_bench", "terminal.grid.history_size() < 100", "true"),
            ("resize_bench", "terminal.selection_to_string()", "fake_copy()"),
        ):
            with self.subTest(owner=owner, mutation=old):
                sources = self.native_sources.copy()
                self.assertIn(old, sources[owner])
                sources[owner] = sources[owner].replace(old, new)
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_resize_listing_sources(sources)

    def test_back_arrow_and_all_shortcut_label_consumers_are_enforced(self) -> None:
        # These source seams complement runtime table/override/geometry tests;
        # they prove that the checked model is still used by the drawing owner.
        for owner, old, new in [
            ("palette", "icon: CommandIcon::Back,", "icon: CommandIcon::TabPrevious,"),
            ("palette", "for [x1, y1, x2, y2] in BACK_ARROW_STROKES", "for [x1, y1, x2, y2] in []"),
            ("palette", "legacy_binding_target(action)", "legacy_binding_target(PaletteAction::SplitRight)"),
            ("screen", ".set_effective_bindings(", ".skip_effective_bindings("),
        ]:
            with self.subTest(mutation=old):
                sources = self.native_sources.copy()
                self.assertIn(old, sources[owner])
                sources[owner] = sources[owner].replace(old, new)
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_native_contract_sources(sources)

    def test_dispatch_cannot_normalize_or_clone_for_every_candidate(self) -> None:
        # A source cost guard complements the actual platform binding tests:
        # normalization belongs to the event, action ownership to a match.
        for old, new in [
            ("let logical_key =", "let repeated_logical_key ="),
            ("if binding.is_triggered_by", "if unchecked_candidate"),
            ("let action = binding.action.clone();",
             "let action = binding.action.clone(); let action = binding.action.clone();"),
        ]:
            with self.subTest(mutation=old):
                sources = self.native_sources.copy()
                prefix, dispatch = sources["screen"].split("pub fn process_key_bindings(", 1)
                sources["screen"] = prefix + "pub fn process_key_bindings(" + dispatch.replace(old, new, 1)
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_native_contract_sources(sources)

    def test_responsive_close_and_persistent_back_guards_cannot_disappear(self) -> None:
        cases = {
            "pty-scheduler-process-lifecycle": ["Nonblocking pane retirement", "thread-local destruction gates", "capacity recovery", "actual join acknowledgements", "no UI-thread joins", "Confirmed window dismissal", "Saturated native output", "500 ms desktop ceiling"],
            "windows-tabs-sessions-input": ["Persistent header Back", "independent legacy scoring"],
        }
        for owner, phrases in cases.items():
            for phrase in phrases:
                with self.subTest(owner=owner, phrase=phrase):
                    document = copy.deepcopy(self.document)
                    feature = next(row for row in document["features"] if row["id"] == owner)
                    for field in ["needed_tests", "verification_reinforcements"]:
                        feature[field] = [text.replace(phrase, "removed assurance") for text in feature[field]]
                    with self.assertRaises(REINFORCEMENT.ReinforcementError):
                        self.validate(document)

    def test_palette_category_and_shortcut_guards_cannot_disappear(self) -> None:
        for phrase in ["exhaustive category coverage", "held Enter", "shifted punctuation", "strict-profile isolation", "shared palette activation owner", "Alt+R/D clone", "all effective shortcut labels", "left-arrow Back"]:
            with self.subTest(phrase=phrase):
                document = copy.deepcopy(self.document)
                feature = next(row for row in document["features"] if row["id"] == "windows-tabs-sessions-input")
                for field in ["needed_tests", "verification_reinforcements"]:
                    feature[field] = [text.replace(phrase, "removed assurance") for text in feature[field]]
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    self.validate(document)

    @classmethod
    def setUpClass(cls) -> None:
        # Keep the canonical ledger immutable; every test mutates a deep copy and
        # proves the checker rejects one missing assurance dimension.
        cls.document = json.loads(REINFORCEMENT.DEFAULT_CONTRACT.read_text(encoding="utf-8"))
        cls.native_sources = REINFORCEMENT._load_native_contract_sources(
            REINFORCEMENT.ROOT
        )

    def validate(self, document: dict) -> dict[str, int]:
        return REINFORCEMENT.validate_document(document)

    def test_canonical_contract_covers_every_feature_with_detailed_reinforcement(self) -> None:
        counts = self.validate(copy.deepcopy(self.document))
        self.assertEqual(counts["features"], 40)
        self.assertGreaterEqual(counts["needed_tests"], counts["features"] * 3)
        self.assertGreaterEqual(counts["evidence_owners"], counts["features"])
        self.assertEqual(counts["plan_anchors"], counts["features"])

    def test_missing_reordered_or_extra_feature_is_rejected(self) -> None:
        missing = copy.deepcopy(self.document)
        missing["features"].pop()
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "order/coverage"):
            self.validate(missing)

        reordered = copy.deepcopy(self.document)
        reordered["features"][0], reordered["features"][1] = reordered["features"][1], reordered["features"][0]
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "order/coverage"):
            self.validate(reordered)

    def test_visual_feature_cannot_lose_pixel_or_accessibility_proof(self) -> None:
        for field, value, message in (
            ("test_layers", "visual-pixel", "visual and accessibility layers"),
            ("oracles", "pixel-exact", "exact pixel/accessibility oracles"),
            ("oracles", "accessibility-tree-event", "exact pixel/accessibility oracles"),
        ):
            with self.subTest(field=field, value=value):
                document = copy.deepcopy(self.document)
                feature = next(item for item in document["features"] if item["flags"]["visual"])
                feature[field].remove(value)
                with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, message):
                    self.validate(document)

    def test_native_security_persistence_and_performance_layers_are_mandatory(self) -> None:
        mutations = (
            ("native", "test_layers", "native-e2e", "native end-to-end"),
            ("security", "oracles", "side-effect-absence", "absence proof"),
            ("persistence", "test_layers", "recovery-migration", "recovery/round-trip"),
            ("performance", "oracles", "latency-ratchet", "resource and latency"),
        )
        for flag, field, value, message in mutations:
            with self.subTest(flag=flag, value=value):
                document = copy.deepcopy(self.document)
                feature = next(item for item in document["features"] if item["flags"][flag])
                feature[field].remove(value)
                with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, message):
                    self.validate(document)

    def test_feature_requires_negative_tests_exit_scope_and_real_evidence_owner(self) -> None:
        document = copy.deepcopy(self.document)
        feature = document["features"][0]
        feature["needed_tests"] = ["Valid scenario A", "Valid scenario B", "Valid scenario C"]
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "negative or boundary"):
            self.validate(document)

        document = copy.deepcopy(self.document)
        document["features"][0]["exit_criteria"] = ["The model passes exactly", "The checker mutations fail"]
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "scope native/external"):
            self.validate(document)

        document = copy.deepcopy(self.document)
        document["features"][0]["evidence_owners"] = ["tests/missing-owner.rs"]
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "missing path"):
            self.validate(document)

    def test_plan_anchor_and_interaction_partner_cannot_drift(self) -> None:
        document = copy.deepcopy(self.document)
        document["features"][0]["plan_anchor"] = "missing-plan-section"
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "exact plan section"):
            self.validate(document)

        document = copy.deepcopy(self.document)
        document["features"][0]["interaction_partners"] = ["unknown-feature"]
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "unknown interaction"):
            self.validate(document)

    def test_duplicate_json_keys_and_absolute_claims_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "duplicate.json"
            path.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "duplicate key"):
                REINFORCEMENT._load_json(path, "fixture")

        document = copy.deepcopy(self.document)
        document["features"][0]["needed_tests"][0] = "Prove that no issues can escape this feature"
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "absolute claim"):
            self.validate(document)

    def test_default_visual_policy_cannot_tolerate_one_pixel_or_mask_it(self) -> None:
        original = REINFORCEMENT.DEFAULT_CONTRACT
        with tempfile.TemporaryDirectory(dir=REINFORCEMENT.ROOT) as directory:
            root = Path(directory)
            # Test channel tolerance and masking separately: either mechanism could
            # otherwise conceal the exact one-pixel regression this policy guards.
            policy = root / "visual.json"
            policy.write_text(
                json.dumps(
                    {
                        "schema": 1,
                        "max_channel_delta": 1,
                        "max_changed_pixel_ratio": 0.0,
                        "masks": [],
                    }
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "one changed pixel"):
                REINFORCEMENT._validate_exact_visual_policy(policy)

            policy.write_text(
                json.dumps(
                    {
                        "schema": 1,
                        "max_channel_delta": 0,
                        "max_changed_pixel_ratio": 0.0,
                        "masks": [{"x": 0, "y": 0, "width": 1, "height": 1}],
                    }
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "cannot mask pixels"):
                REINFORCEMENT._validate_exact_visual_policy(policy)
        self.assertEqual(REINFORCEMENT.DEFAULT_CONTRACT, original)

    def test_renderer_chrome_specific_scenario_details_cannot_be_weakened(self) -> None:
        for field, detail in (
            ("needed_tests", "threshold-minus-one"),
            ("needed_tests", "40 logical-pixel interaction targets"),
            ("needed_tests", "zero PTY input"),
            ("needed_tests", "WGPU and CPU"),
            ("needed_tests", "fractional trackpad"),
            ("needed_tests", "physical-to-logical scaling"),
            ("needed_tests", "responsive reclamping"),
            ("verification_reinforcements", "42-pixel header"),
            ("verification_reinforcements", "full-shelf RGBA"),
            ("verification_reinforcements", "ownership before pane selection"),
            ("verification_reinforcements", "1,024-row event bound"),
            ("verification_reinforcements", "persistent idle indicator"),
            ("checker_reinforcements", "184-pixel default tab cap"),
            ("checker_reinforcements", "modal event ownership"),
        ):
            with self.subTest(field=field, detail=detail):
                document = copy.deepcopy(self.document)
                feature = next(
                    item
                    for item in document["features"]
                    if item["id"] == "renderer-fonts-responsive-ui"
                )
                feature[field] = [
                    item.replace(detail, "generic visual coverage")
                    for item in feature[field]
                ]
                with self.assertRaisesRegex(
                    REINFORCEMENT.ReinforcementError, "required scenario detail"
                ):
                    self.validate(document)

    def test_command_timestamp_scenario_details_cannot_be_weakened(self) -> None:
        mutations = {
            "ecosystem-d7-cp6-proposal": (
                ("needed_tests", "pre-arming interrupts"),
                ("needed_tests", "shared-engine ticks"),
                ("needed_tests", "reused cancellation tokens"),
                ("needed_tests", "worker-unwind cleanup"),
                ("verification_reinforcements", "invocation-local completion"),
                ("verification_reinforcements", "joined watchdogs"),
            ),
            "terminal-protocols-grid-history": (
                ("needed_tests", "viewport identity journal"),
                ("verification_reinforcements", "first visible cell and snapshot styles"),
                ("needed_tests", "parser-created selection journal"),
                ("verification_reinforcements", "unselected control grid"),
                ("needed_tests", "hard-line journal"),
                ("verification_reinforcements", "text, hard-break, whitespace and style faults"),
                ("needed_tests", "boundary-only CMD D"),
                ("needed_tests", "pre-epoch"),
                ("needed_tests", "timezone or DST transitions"),
                ("verification_reinforcements", "source prompt, following-prompt boundary"),
                ("verification_reinforcements", "no shell-provided timestamp text"),
                ("checker_reinforcements", "local timezone conversion"),
                ("checker_reinforcements", "no-PTY side-effect coverage"),
            ),
            "renderer-fonts-responsive-ui": (
                ("needed_tests", "full ISO local date and time"),
                ("needed_tests", "compact date-time fallbacks"),
                ("verification_reinforcements", "painted command datetime label"),
                ("verification_reinforcements", "terminal cells, PTY bytes"),
                ("checker_reinforcements", "no-fabrication behavior"),
            ),
        }
        for feature_id, details in mutations.items():
            for field, detail in details:
                with self.subTest(feature_id=feature_id, field=field, detail=detail):
                    document = copy.deepcopy(self.document)
                    feature = next(
                        item for item in document["features"] if item["id"] == feature_id
                    )
                    feature[field] = [
                        item.replace(detail, "generic timestamp coverage")
                        for item in feature[field]
                    ]
                    with self.assertRaisesRegex(
                        REINFORCEMENT.ReinforcementError, "required scenario detail"
                    ):
                        self.validate(document)

    def test_command_result_resize_navigation_details_cannot_be_weakened(self) -> None:
        mutations = {
            "terminal-protocols-grid-history": (
                ("needed_tests", "resize followed by previous/next command navigation"),
                ("needed_tests", "Live child resize acknowledgments"),
                ("needed_tests", "worker-owned coalesced resize transactions"),
                ("needed_tests", "ConPTY history/live seams"),
                ("verification_reinforcements", "native repaint content before child exit"),
                ("checker_reinforcements", "never substitute a post-exit captured stream"),
                ("verification_reinforcements", "duplicate IDs"),
                ("verification_reinforcements", "successful frame presentation"),
                ("checker_reinforcements", "resize-navigation assurance"),
                ("checker_reinforcements", "post-present publication"),
            ),
            "renderer-fonts-responsive-ui": (
                ("needed_tests", "one badge per result identity and display row"),
                ("needed_tests", "measured label rectangles"),
                ("needed_tests", "prompt-context paint rectangles"),
                ("needed_tests", "frozen command duration"),
                ("needed_tests", "post-present checkpoint publication"),
                ("needed_tests", "two consecutive identical full-frame pixel digests"),
                ("needed_tests", "physical surface width and height"),
                ("needed_tests", "successful-present-only cache recording"),
                ("verification_reinforcements", "every command-result draw rectangle"),
                ("verification_reinforcements", "cross-owner non-intersection"),
                ("verification_reinforcements", "zero changed channel tolerance"),
                ("verification_reinforcements", "successful present completion"),
                ("verification_reinforcements", "native dialog occlusion"),
                ("verification_reinforcements", "two identical full-frame digests"),
                ("verification_reinforcements", "full opaque on-screen client pixels"),
                ("verification_reinforcements", "skippable only after successful presentation"),
                ("checker_reinforcements", "one-badge-per-row ownership"),
                ("checker_reinforcements", "prompt-context paint rectangles"),
                ("checker_reinforcements", "frozen command duration"),
                ("checker_reinforcements", "resize-navigation sequencing"),
                ("checker_reinforcements", "forced next-frame control handling"),
                ("checker_reinforcements", "dialog-occlusion rejection"),
                ("checker_reinforcements", "independent frame inspection"),
                ("checker_reinforcements", "physical extent identity"),
                ("checker_reinforcements", "fixed unsent editor input"),
            ),
            "windows-tabs-sessions-input": (
                ("needed_tests", "Complete classic palette defaults"),
                ("needed_tests", "source-free shortcut chips"),
                ("needed_tests", "palette-only Enter fallback"),
                ("needed_tests", "Escape cancellation"),
                ("needed_tests", "resizes immediately before Ctrl+Shift+Up/Down"),
                ("needed_tests", "intersecting badge rectangles"),
                ("needed_tests", "intersecting prompt-context rectangles"),
                ("needed_tests", "broadcast-first teardown"),
                ("needed_tests", "ConPTY reparenting"),
                ("verification_reinforcements", "all command-result paint rectangles"),
                ("verification_reinforcements", "visible snapshot publication"),
                ("verification_reinforcements", "six-second controlled Windows"),
                ("verification_reinforcements", "sequential deadline multiplication"),
                ("checker_reinforcements", "resize-before-shortcut ordering"),
                ("checker_reinforcements", "no-PTY proof"),
                ("checker_reinforcements", "pre-close owned process identities"),
                ("checker_reinforcements", "application and descendant exit"),
            ),
        }
        for feature_id, details in mutations.items():
            for field, detail in details:
                with self.subTest(feature_id=feature_id, field=field, detail=detail):
                    document = copy.deepcopy(self.document)
                    feature = next(
                        item for item in document["features"] if item["id"] == feature_id
                    )
                    feature[field] = [
                        item.replace(detail, "generic resize coverage")
                        for item in feature[field]
                    ]
                    with self.assertRaisesRegex(
                        REINFORCEMENT.ReinforcementError, "required scenario detail"
                    ):
                        self.validate(document)

    def test_pty_shutdown_scenario_details_cannot_be_weakened(self) -> None:
        details = (
            ("needed_tests", "parked split/local-tab exit journal"),
            ("verification_reinforcements", "surviving channels remain empty and connected"),
            ("verification_reinforcements", "restore refresh precedes visibility"),
            ("needed_tests", "Ordinary and exact Windows ConPTY"),
            ("needed_tests", "broadcast-first teardown"),
            ("needed_tests", "parked"),
            ("verification_reinforcements", "exact temporary-fixture process identities before close"),
            ("verification_reinforcements", "ConPTY reparenting"),
            ("verification_reinforcements", "multi-session wall-clock ceiling"),
            ("verification_reinforcements", "idempotent broadcasts"),
            ("verification_reinforcements", "no sequential deadline multiplication"),
            ("checker_reinforcements", "ordinary Job ownership"),
            ("checker_reinforcements", "broadcast-before-join ordering"),
            ("checker_reinforcements", "exact pre-close process identity"),
            ("checker_reinforcements", "repeated-request idempotence"),
        )
        for field, detail in details:
            with self.subTest(field=field, detail=detail):
                document = copy.deepcopy(self.document)
                feature = next(
                    item
                    for item in document["features"]
                    if item["id"] == "pty-scheduler-process-lifecycle"
                )
                feature[field] = [
                    item.replace(detail, "generic lifecycle coverage")
                    for item in feature[field]
                ]
                with self.assertRaisesRegex(
                    REINFORCEMENT.ReinforcementError, "required scenario detail"
                ):
                    self.validate(document)

    def test_shell_control_scenarios_cannot_be_replaced_by_generic_key_tests(self) -> None:
        for field, detail in (
            ("needed_tests", "shell-owned Ctrl+R and Ctrl+D"),
            ("needed_tests", "single-message captured-target paste"),
            ("verification_reinforcements", "sibling silence and unchanged selection on rejection"),
            ("needed_tests", "typed fallback and reset"),
            ("verification_reinforcements", "no clone action"),
            ("verification_reinforcements", "explicit user mappings"),
        ):
            with self.subTest(field=field, detail=detail):
                document = copy.deepcopy(self.document)
                feature = next(item for item in document["features"] if item["id"] == "windows-tabs-sessions-input")
                self.assertTrue(any(detail in item for item in feature[field]))
                feature[field] = [item.replace(detail, "generic key coverage") for item in feature[field]]
                with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "required scenario detail"):
                    self.validate(document)

    def test_qa_source_identity_requirements_cannot_be_dropped(self) -> None:
        for field, detail in (
            ("needed_tests", "content-bound dirty fingerprints"),
            ("needed_tests", "logical artifact announcements"),
            ("needed_tests", "pre-descent cache pruning"),
            ("needed_tests", "unreadable source"),
            ("verification_reinforcements", "before/after source identity drift"),
        ):
            with self.subTest(field=field):
                document = copy.deepcopy(self.document)
                feature = next(item for item in document["features"] if item["id"] == "stabilization-release-assurance-s1-s2")
                feature[field] = [item.replace(detail, "generic evidence") for item in feature[field]]
                with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "required scenario detail"):
                    self.validate(document)

    def test_chrome_and_output_readability_require_independent_render_tests(self) -> None:
        for field, detail in (
            ("needed_tests", "Short inset command markers"),
            ("verification_reinforcements", "exact marker pixel coverage"),
            ("checker_reinforcements", "marker geometry benchmarks and native inset checks"),
            ("needed_tests", "quantized contrast on all surfaces"),
            ("verification_reinforcements", "decorative and focus roles"),
            ("checker_reinforcements", "all-surface contrast requirements"),
            ("needed_tests", "parser-created table row bands"),
            ("verification_reinforcements", "exact row-gap pixel coverage"),
            ("checker_reinforcements", "row-band benchmark and unchanged copy bytes"),
        ):
            with self.subTest(field=field):
                document = copy.deepcopy(self.document)
                feature = next(item for item in document["features"] if item["id"] == "renderer-fonts-responsive-ui")
                self.assertTrue(any(detail in item for item in feature[field]))
                feature[field] = [item.replace(detail, "generic rendering") for item in feature[field]]
                with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "required scenario detail"):
                    self.validate(document)

    def test_public_sbom_privacy_requirements_cannot_be_dropped(self) -> None:
        for field, detail in (
            ("needed_tests", "recomputed-checksum SBOM privacy"),
            ("needed_tests", "Minimal publication rejects full inventories"),
            ("needed_tests", "rehashed private paths and identifiers"),
            ("needed_tests", "non-private retention"),
            ("needed_tests", "real ephemeral-key Minisign tamper coverage"),
            ("verification_reinforcements", "complete graph, license and file-hash preservation"),
        ):
            document = copy.deepcopy(self.document)
            feature = next(item for item in document["features"] if item["id"] == "packaging-release-provenance")
            feature[field] = [item.replace(detail, "generic metadata") for item in feature[field]]
            with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "required scenario detail"):
                self.validate(document)

    def test_native_renderer_and_lifecycle_source_mutations_fail_closed(self) -> None:
        mutations = (
            ("application", "route.window.winit_window.set_visible(false);", ""),
            ("application", "self.router.hide_windows_for_exit();", ""),
            ("router", "for route in self.routes.values()", "for route in self.routes.values().take(1)"),
            ("router", "self.window.screen.context_manager.quit();", "self.window.screen.shutdown_connection_hub(); self.window.screen.context_manager.quit();"),
            ("windows_pipes", "self.inner.discard.store(true", "self.inner.discard.store(false"),
            ("windows_pipes", "self.inner.sig_buffer_not_full.notify_one();", ""),
            ("windows_pipes", "&& !inner.discard.load(Ordering::SeqCst)", ""),
            ("windows_pipes", "if self.consumer.is_empty()", "if false"),
            ("windows_pty", "self.conout.discard_remaining();", ""),
            ("native_driver", "$shutdownTimer.ElapsedMilliseconds -lt 500", "$shutdownTimer.ElapsedMilliseconds -lt 6000"),
            ("native_driver", "$windowCloseTimer.ElapsedMilliseconds -lt 500", "$windowCloseTimer.ElapsedMilliseconds -lt 6000"),
            ("screen", "if !frame_dropped", "if frame_dropped"),
            ("application", "manager.request_pty_shutdown()", "manager.route_ids()"),
            ("context", "self.shutdown_requested.swap(true", "self.shutdown_requested.load("),
            ("context", "self.current_grid_mut().update_dimensions(sugarloaf);", ""),
            ("context", "self.current_grid_mut().update_dimensions(sugarloaf);\n        self.keep_only_active_context_visible(sugarloaf);", "self.keep_only_active_context_visible(sugarloaf);\n        self.current_grid_mut().update_dimensions(sugarloaf);"),
            ("router", "context_manager.quit()", "context_manager.route_ids()"),
            (
                "windows_pty",
                "env,\n        true,\n        true,\n        columns,",
                "env,\n        true,\n        false,\n        columns,",
            ),
            (
                "windows_conpty",
                "limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;",
                "limits.BasicLimitInformation.LimitFlags = 0;",
            ),
            (
                "sugarloaf_cpu",
                "hash_surface_extent(&mut h, ctx.width_px, ctx.height_px);",
                "h.write_u8(0);",
            ),
            (
                "sugarloaf_cpu",
                "Ok(()) => cache.record_presented_frame(frame_hash),",
                "Ok(()) => {},",
            ),
            ("native_driver", "previousFrame.PixelDigest", "previousFrame.DistinctColorBuckets"),
            ("native_driver", "NonOpaquePixelCount = nonOpaquePixels", "NonOpaquePixelCount = 0"),
            ("native_driver", "AMX_CAPTURE_INPUT_59217", ""),
            ("native_driver", "$process.CloseMainWindow()", "$process.Kill()"),
        )
        for owner, old, new in mutations:
            with self.subTest(owner=owner, removed=old):
                sources = copy.deepcopy(self.native_sources)
                self.assertIn(old, sources[owner])
                sources[owner] = sources[owner].replace(old, new, 1)
                with self.assertRaises(REINFORCEMENT.ReinforcementError):
                    REINFORCEMENT._validate_native_contract_sources(sources)

    def test_explicit_native_shutdown_must_retire_output_before_waiting(self) -> None:
        sources = self.native_sources.copy()
        prefix, shutdown = sources["windows_pty"].split("fn shutdown_owned_process_tree", 1)
        self.assertIn("self.conout.discard_remaining();", shutdown)
        sources["windows_pty"] = prefix + "fn shutdown_owned_process_tree" + shutdown.replace("self.conout.discard_remaining();", "", 1)
        with self.assertRaises(REINFORCEMENT.ReinforcementError):
            REINFORCEMENT._validate_native_contract_sources(sources)

    def test_final_callback_dismissal_and_broadcast_must_precede_waits(self) -> None:
        for old in ["self.router.hide_windows_for_exit();", "self.router.routes.clear();"]:
            sources = self.native_sources.copy()
            prefix, final = sources["application"].split("fn exiting(&mut self", 1)
            self.assertIn(old, final)
            final = final.replace(old, "", 1)
            sources["application"] = prefix + "fn exiting(&mut self" + final
            with self.assertRaises(REINFORCEMENT.ReinforcementError):
                REINFORCEMENT._validate_native_contract_sources(sources)


if __name__ == "__main__":
    unittest.main(verbosity=2)
