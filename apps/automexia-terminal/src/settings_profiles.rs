//! Profiles reuse the existing native Settings controls and text/IME/color editor.
use super::*;
use crate::automexia::profiles::{ProfileOperation, ProfileSnapshot};
use automexia_ui_model::settings::{
    Availability, ChangeScope, SettingOwner, SettingsError,
};
use rio_backend::config::profiles::*;

#[derive(Clone)]
pub(crate) enum ProfileIntent {
    Load,
    Io(ProfileOperation),
    Launch(NamedProfile, bool),
}
#[derive(Clone, Copy)]
pub(super) enum ProfileConfirmation {
    Discard,
    Delete,
}
#[derive(Default)]
pub(super) struct ProfilesView {
    base: ProfileDocument,
    snapshot: ProfileSnapshot,
    entries: Vec<NamedProfile>,
    pub(super) selected: Option<NamedProfile>,
    pub(super) draft: Option<NamedProfile>,
    dirty: bool,
    pub busy: bool,
    pub generation: u64,
    revision: u64,
    pending: Option<ProfileIntent>,
}
fn row(
    id: &str,
    label: &str,
    description: &str,
    kind: SettingKind,
    value: SettingValue,
) -> Result<SettingDescriptor, SettingsError> {
    Ok(SettingDescriptor {
        id: SettingId::new(format!("profiles.{id}"))?,
        owner: SettingOwner::Core,
        section: Section::Sessions,
        label: label.into(),
        description: description.into(),
        keywords: vec![],
        kind,
        value: value.clone(),
        default: value,
        origin: ValueOrigin::User,
        availability: Availability::Available,
        scope: ChangeScope::NewSession,
    })
}
pub(crate) fn settings_entry() -> Result<SettingDescriptor, SettingsError> {
    row(
        "open",
        "Profiles",
        "Create, edit and open named terminal presets.",
        SettingKind::Action,
        SettingValue::Action,
    )
}
fn action(id: &str, label: &str, help: &str) -> Result<SettingDescriptor, SettingsError> {
    row(id, label, help, SettingKind::Action, SettingValue::Action)
}
fn text_row(
    id: &str,
    label: &str,
    value: &str,
    max: usize,
    help: &str,
) -> Result<SettingDescriptor, SettingsError> {
    row(
        id,
        label,
        help,
        SettingKind::Text {
            max_bytes: max,
            allow_empty: true,
        },
        SettingValue::Text(value.into()),
    )
}
fn choice(
    id: &str,
    label: &str,
    value: &str,
    values: &[(&str, &str)],
) -> Result<SettingDescriptor, SettingsError> {
    row(
        id,
        label,
        "Use arrows to choose.",
        SettingKind::Choice {
            options: values
                .iter()
                .map(
                    |(value, label)| automexia_ui_model::settings::ChoiceOption {
                        value: (*value).into(),
                        label: (*label).into(),
                    },
                )
                .collect(),
        },
        SettingValue::Choice(value.into()),
    )
}
impl ProfilesView {
    fn rebuild(&mut self) -> Result<(), ProfileError> {
        self.entries = self.base.merged(&self.snapshot.document)?.profiles;
        if !self.entries.iter().any(|p| p.id == "default") {
            self.entries
                .insert(0, NamedProfile::new("default".into(), "Default".into()));
        }
        for (id, name) in [("work", "Work"), ("personal", "Personal"), ("ssh", "SSH")] {
            if !self.entries.iter().any(|p| p.id == id) {
                let mut profile = NamedProfile::new(id.into(), name.into());
                if id == "ssh" {
                    profile.program = Some("ssh".into());
                }
                self.entries.push(profile);
            }
        }
        if cfg!(target_os = "windows") && !self.entries.iter().any(|p| p.id == "wsl") {
            let mut p = NamedProfile::new("wsl".into(), "WSL".into());
            p.program = Some("wsl.exe".into());
            p.platform = ProfilePlatform::Windows;
            self.entries.push(p);
        }
        Ok(())
    }
    fn catalog(&self) -> Result<Catalog, SettingsError> {
        let mut rows = vec![];
        if let Some(p) = &self.draft {
            rows.push(text_row(
                "name",
                "Name",
                &p.name,
                128,
                "A label for this profile.",
            )?);
            rows.push(text_row("program","Executable",p.program.as_deref().unwrap_or(""),MAX_TEXT_BYTES,"Exact executable path or name. Empty follows configuration. Never enter a command line.")?);
            rows.push(choice(
                "platform",
                "Operating system",
                match p.platform {
                    ProfilePlatform::Any => "any",
                    ProfilePlatform::Windows => "windows",
                    ProfilePlatform::Linux => "linux",
                    ProfilePlatform::Macos => "macos",
                    ProfilePlatform::OtherUnix => "other-unix",
                },
                &[
                    ("any", "Any"),
                    ("windows", "Windows"),
                    ("linux", "Linux"),
                    ("macos", "macOS"),
                    ("other-unix", "Other Unix"),
                ],
            )?);
            for (i, arg) in p.args.iter().enumerate() {
                rows.push(text_row(&format!("arg.a{i}"),&format!("Argument {}",i+1),arg,MAX_TEXT_BYTES,"One exact argument. No splitting or shell expansion. Do not enter credentials.")?);
            }
            rows.push(action(
                "add-arg",
                "Add argument",
                "Each entry is one exact argument.",
            )?);
            if !p.args.is_empty() {
                rows.push(action(
                    "remove-arg",
                    "Remove last argument",
                    "Remove the final argument from this draft.",
                )?);
            }
            rows.push(choice(
                "cwd",
                "Working directory",
                match p.directory {
                    ProfileDirectory::Configuration => "configuration",
                    ProfileDirectory::Inherit => "inherit",
                    ProfileDirectory::Home => "home",
                    ProfileDirectory::Fixed { .. } => "fixed",
                },
                &[
                    ("configuration", "Configuration"),
                    ("inherit", "Current terminal"),
                    ("home", "Home"),
                    ("fixed", "Fixed path"),
                ],
            )?);
            if let ProfileDirectory::Fixed { path } = &p.directory {
                rows.push(text_row("path","Directory path",path,MAX_TEXT_BYTES,"Existing absolute local directory; no shell expansion. For WSL use --cd as arguments.")?);
            }
            rows.push(text_row("theme","Theme",p.theme.as_deref().unwrap_or(""),64,"Empty follows window. Use a theme filename without .toml, such as aurora-night or solar-dusk.")?);
            rows.push(text_row(
                "icon",
                "Tab icon",
                p.icon.as_deref().unwrap_or(""),
                16,
                "Optional single symbol.",
            )?);
            rows.push(row(
                "color-enabled",
                "Custom tab color",
                "Turn off to follow the window theme.",
                SettingKind::Boolean,
                SettingValue::Boolean(p.color.is_some()),
            )?);
            if let Some(color) = &p.color {
                let rgb = u32::from_str_radix(color.trim_start_matches('#'), 16)
                    .unwrap_or(0x64beff);
                rows.push(row(
                    "color",
                    "Tab accent",
                    "Choose a color for this profile's tab.",
                    SettingKind::Color { alpha: false },
                    SettingValue::Color([
                        (rgb >> 16) as u8,
                        (rgb >> 8) as u8,
                        rgb as u8,
                        255,
                    ]),
                )?);
            }
            for (i, e) in p.environment.iter().enumerate() {
                rows.push(text_row(
                    &format!("env-name.e{i}"),
                    &format!("Environment {} · name", i + 1),
                    &e.name,
                    128,
                    "Variable to override in this new session.",
                )?);
                rows.push(text_row(&format!("env-from.e{i}"),&format!("Environment {} · source",i+1),&e.from,128,"Name of an existing environment variable. Its value is read only when launching and is never saved.")?);
            }
            rows.push(action(
                "add-env",
                "Add environment reference",
                "Use existing environment values without saving secrets.",
            )?);
            if !p.environment.is_empty() {
                rows.push(action(
                    "remove-env",
                    "Remove last reference",
                    "Remove the final environment reference from this draft.",
                )?);
            }
            rows.push(action(
                "save",
                "Save profile",
                "Validate and save. Existing terminals are unchanged.",
            )?);
        } else if let Some(p) = &self.selected {
            rows.push(action(
                "launch-tab",
                "Open in new tab",
                "Enter starts a new independent terminal using this profile.",
            )?);
            rows.push(action(
                "launch-window",
                "Open in new window",
                "A separate window uses this profile's complete theme.",
            )?);
            rows.push(action(
                "edit",
                "Edit profile",
                "Change a copy; Save applies it to future launches.",
            )?);
            rows.push(action(
                "duplicate",
                "Duplicate profile",
                "Create a new independent profile.",
            )?);
            rows.push(action("export","Export TOML","Save an editable profile file. Environment references contain no values.")?);
            if self.snapshot.document.profiles.iter().any(|s| s.id == p.id) {
                rows.push(action("delete","Delete saved profile","Confirmation required. A configured profile with this ID becomes visible again.")?);
            }
            let mut summary = text_row(
                "summary",
                "Executable",
                p.program.as_deref().unwrap_or("Use configuration"),
                MAX_TEXT_BYTES,
                "Launch settings can be inspected and changed with Edit.",
            )?;
            summary.availability = Availability::Unavailable {
                reason: "Choose Edit to change this profile.".into(),
            };
            rows.push(summary);
            for (i, arg) in p.args.iter().enumerate() {
                let mut r = text_row(
                    &format!("summary-arg.a{i}"),
                    &format!("Argument {}", i + 1),
                    arg,
                    MAX_TEXT_BYTES,
                    "Exact argument; no shell expansion.",
                )?;
                r.availability = Availability::Unavailable {
                    reason: "Choose Edit to change this argument.".into(),
                };
                rows.push(r);
            }
        } else {
            rows.push(action(
                "new",
                "New profile",
                "Start with your configured shell.",
            )?);
            rows.push(action(
                "import",
                "Import TOML",
                "Load and review; nothing launches or saves automatically.",
            )?);
            rows.push(action("refresh", "Refresh", "Reload saved profiles.")?);
            for (i, p) in self.entries.iter().enumerate() {
                let mut r = action(
                    &format!("select.p{i}"),
                    &p.name,
                    "Open details, edit, or launch this profile.",
                )?;
                if !p.platform.supports(ProfilePlatform::current()) {
                    r.description =
                        "For another operating system. Edit to adapt before launching."
                            .into();
                }
                rows.push(r);
            }
        }
        if self.busy {
            for row in &mut rows {
                row.availability = Availability::Unavailable {
                    reason: "Please wait; Escape remains available.".into(),
                };
            }
        }
        Catalog::new(self.revision, rows)
    }
    fn new_id(&self) -> String {
        (1..=MAX_PROFILES + 1)
            .map(|n| format!("profile-{n}"))
            .find(|id| !self.entries.iter().any(|p| &p.id == id))
            .unwrap_or_else(|| "profile-new".into())
    }
}
impl SettingsView {
    pub(crate) fn open_profiles(&mut self, base: ProfileDocument) {
        self.close();
        self.profile_generation = self.profile_generation.wrapping_add(1);
        let mut view = ProfilesView {
            base,
            generation: self.profile_generation,
            revision: 1,
            pending: None,
            ..ProfilesView::default()
        };
        if view.rebuild().is_err() {
            self.status =
                "Configured profiles are invalid. Correct the profiles table first."
                    .into();
        }
        self.profiles = Some(view);
        self.refresh_profile_page(false);
    }
    pub(crate) fn profile_session(&self) -> Option<u64> {
        self.profiles.as_ref().map(|p| p.generation)
    }
    pub(crate) fn profile_notice(&mut self, message: &str, busy: bool) {
        if let Some(p) = &mut self.profiles {
            p.busy = busy;
        }
        self.status = message.into();
        self.refresh_profile_page(true);
    }
    pub(crate) fn receive_profiles(&mut self, snapshot: ProfileSnapshot) {
        self.status.clear();
        if let Some(p) = &mut self.profiles {
            p.snapshot = snapshot;
            p.busy = false;
            p.draft = None;
            p.selected = None;
            p.dirty = false;
            if p.rebuild().is_err() {
                self.status =
                    "Profile limit exceeded. Remove duplicate configuration entries."
                        .into();
            }
        }
        self.refresh_profile_page(false);
    }
    pub(crate) fn import_profiles(&mut self, document: ProfileDocument) {
        if document.profiles.len() != 1 {
            self.profile_notice(
                "Import one profile at a time. Nothing was saved.",
                false,
            );
            return;
        }
        if let Some(p) = &mut self.profiles {
            p.busy = false;
            if let Some(mut imported) = document.profiles.into_iter().next() {
                imported.id = p.new_id();
                p.draft = Some(imported);
                p.dirty = true;
            }
        }
        self.status = "Imported draft. Review fields, then Save.".into();
        self.refresh_profile_page(false);
    }
    fn refresh_profile_page(&mut self, preserve: bool) {
        let Some(profiles) = &mut self.profiles else {
            return;
        };
        profiles.revision = profiles.revision.saturating_add(1);
        let Ok(catalog) = profiles.catalog() else {
            self.status = "Profile form cannot be displayed.".into();
            return;
        };
        if preserve {
            if let Some(view) = &mut self.view {
                view.refresh(&catalog);
            } else {
                self.view = Some(ViewState::new(&catalog, 6));
            }
        } else {
            self.view = Some(ViewState::new(&catalog, 6));
            self.focus = Focus::List;
            self.scroll = 0.0;
            self.preedit.clear();
            self.caret = 0;
            self.anchor = None;
        }
        self.catalog = Some(catalog);
        self.layout_dirty = true;
        self.reveal_focus = true;
    }
    pub(crate) fn take_profile_intent(&mut self) -> Option<ProfileIntent> {
        self.profiles.as_ref()?;
        if let Some(edit) = self.pending.take() {
            if self
                .catalog
                .as_ref()
                .is_some_and(|c| c.validate_edit(&edit).is_ok())
            {
                self.apply_profile_edit(edit);
            }
        }
        self.profiles.as_mut()?.pending.take()
    }
    fn apply_profile_edit(&mut self, edit: Edit) {
        let id = edit.id.as_str().strip_prefix("profiles.").unwrap_or("");
        if edit.change == Change::Activate {
            self.profile_action(id);
            return;
        }
        let Some(p) = self.profiles.as_mut().and_then(|p| p.draft.as_mut()) else {
            return;
        };
        let Change::Set(value) = edit.change else {
            return;
        };
        match (id, value) {
            ("name", SettingValue::Text(s)) => p.name = s,
            ("program", SettingValue::Text(s)) => {
                p.program = (!s.is_empty()).then_some(s)
            }
            ("theme", SettingValue::Text(s)) => p.theme = (!s.is_empty()).then_some(s),
            ("icon", SettingValue::Text(s)) => p.icon = (!s.is_empty()).then_some(s),
            ("color-enabled", SettingValue::Boolean(enabled)) => {
                p.color = enabled.then(|| "#64beff".into())
            }
            ("color", SettingValue::Color([r, g, b, _])) => {
                p.color = Some(format!("#{r:02X}{g:02X}{b:02X}"))
            }
            ("platform", SettingValue::Choice(s)) => {
                p.platform = match s.as_str() {
                    "windows" => ProfilePlatform::Windows,
                    "linux" => ProfilePlatform::Linux,
                    "macos" => ProfilePlatform::Macos,
                    "other-unix" => ProfilePlatform::OtherUnix,
                    _ => ProfilePlatform::Any,
                }
            }
            ("cwd", SettingValue::Choice(s)) => {
                p.directory = match s.as_str() {
                    "inherit" => ProfileDirectory::Inherit,
                    "home" => ProfileDirectory::Home,
                    "fixed" => ProfileDirectory::Fixed {
                        path: String::new(),
                    },
                    _ => ProfileDirectory::Configuration,
                }
            }
            ("path", SettingValue::Text(s)) => {
                p.directory = ProfileDirectory::Fixed { path: s }
            }
            (id, SettingValue::Text(s)) => {
                if let Some(i) = id
                    .strip_prefix("arg.a")
                    .and_then(|n| n.parse::<usize>().ok())
                {
                    if let Some(arg) = p.args.get_mut(i) {
                        *arg = s;
                    }
                } else if let Some(i) = id
                    .strip_prefix("env-name.e")
                    .and_then(|n| n.parse::<usize>().ok())
                {
                    if let Some(e) = p.environment.get_mut(i) {
                        e.name = s;
                    }
                } else if let Some(i) = id
                    .strip_prefix("env-from.e")
                    .and_then(|n| n.parse::<usize>().ok())
                {
                    if let Some(e) = p.environment.get_mut(i) {
                        e.from = s;
                    }
                }
            }
            _ => return,
        }
        if let Some(p) = &mut self.profiles {
            p.dirty = true;
        }
        self.refresh_profile_page(true);
    }
    fn profile_action(&mut self, id: &str) {
        let Some(p) = &mut self.profiles else {
            return;
        };
        if p.busy {
            return;
        }
        match id {
            "new" => {
                p.draft = Some(NamedProfile::new(p.new_id(), "New profile".into()));
                p.dirty = true;
            }
            "refresh" => p.pending = Some(ProfileIntent::Load),
            "import" => {
                p.pending = Some(ProfileIntent::Io(ProfileOperation::Import(
                    std::path::PathBuf::new(),
                )))
            }
            "edit" => {
                p.draft = p.selected.clone();
                p.dirty = false;
            }
            "duplicate" => {
                if let Some(mut copy) = p.selected.clone() {
                    copy.id = p.new_id();
                    copy.name = "Profile copy".into();
                    p.draft = Some(copy);
                    p.dirty = true;
                }
            }
            "launch-tab" | "launch-window" => {
                if let Some(profile) = p.selected.clone() {
                    p.pending =
                        Some(ProfileIntent::Launch(profile, id == "launch-window"));
                }
            }
            "export" => {
                if let Some(profile) = p.selected.clone() {
                    p.pending = Some(ProfileIntent::Io(ProfileOperation::Export {
                        document: ProfileDocument {
                            version: 1,
                            profiles: vec![profile],
                        },
                        destination: std::path::PathBuf::new(),
                    }));
                }
            }
            "save" => {
                if let Some(draft) = &p.draft {
                    if let Err(error) = draft.validate() {
                        self.status = error.to_string();
                        return;
                    }
                    let mut document = p.snapshot.document.clone();
                    document.profiles.retain(|old| old.id != draft.id);
                    document.profiles.push(draft.clone());
                    if let Err(e) = p.base.merged(&document) {
                        self.status = e.to_string();
                        return;
                    }
                    p.pending = Some(ProfileIntent::Io(ProfileOperation::Save {
                        document,
                        expected: p.snapshot.bytes.clone(),
                    }));
                }
            }
            "delete" => {
                self.ask_confirmation(ConfirmedSettingsAction::Profile(ProfileConfirmation::Delete),"Delete this saved profile?".into(),"Running terminals stay open. Configuration profiles remain available.","Delete");
                return;
            }
            "add-arg" => {
                if let Some(draft) = &mut p.draft {
                    if draft.args.len() < MAX_ARGUMENTS {
                        draft.args.push(String::new());
                        p.dirty = true;
                    }
                }
            }
            "remove-arg" => {
                if let Some(draft) = &mut p.draft {
                    draft.args.pop();
                    p.dirty = true;
                }
            }
            "add-env" => {
                if let Some(draft) = &mut p.draft {
                    if draft.environment.len() < 64 {
                        draft.environment.push(EnvironmentReference {
                            name: String::new(),
                            from: String::new(),
                        });
                        p.dirty = true;
                    }
                }
            }
            "remove-env" => {
                if let Some(draft) = &mut p.draft {
                    draft.environment.pop();
                    p.dirty = true;
                }
            }
            _ => {
                if let Some(i) = id
                    .strip_prefix("select.p")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    p.selected = p.entries.get(i).cloned();
                }
            }
        }
        self.refresh_profile_page(false);
    }
    pub(super) fn confirm_profile(&mut self, action: ProfileConfirmation) {
        let Some(p) = &mut self.profiles else {
            return;
        };
        match action {
            ProfileConfirmation::Discard => {
                p.draft = None;
                p.dirty = false;
            }
            ProfileConfirmation::Delete => {
                if let Some(selected) = &p.selected {
                    let mut document = p.snapshot.document.clone();
                    document.profiles.retain(|old| old.id != selected.id);
                    p.pending = Some(ProfileIntent::Io(ProfileOperation::Save {
                        document,
                        expected: p.snapshot.bytes.clone(),
                    }));
                }
            }
        }
        self.refresh_profile_page(false);
    }
    pub(super) fn profile_back(&mut self) {
        let Some(p) = &mut self.profiles else {
            return;
        };
        if p.busy && p.draft.is_some() {
            // An admitted save may already have replaced the file. Keep its
            // result owner alive instead of promising to discard that write.
            self.profile_notice("Saving profile… Please wait before going back.", true);
            return;
        }
        if p.draft.is_some() && p.dirty {
            self.ask_confirmation(
                ConfirmedSettingsAction::Profile(ProfileConfirmation::Discard),
                "Discard profile changes?".into(),
                "Your saved profiles remain unchanged.",
                "Discard",
            );
            return;
        }
        if p.draft.take().is_none() && p.selected.take().is_none() {
            self.leave_menu();
            return;
        }
        // A prepared launch belongs to the details page that requested it.
        // Back invalidates that owner even when the library itself stays open.
        self.profile_generation = self.profile_generation.wrapping_add(1);
        p.generation = self.profile_generation;
        p.pending = None;
        p.busy = false;
        self.status.clear();
        self.refresh_profile_page(false);
    }
    pub(super) fn profile_primary(&mut self) {
        let id = if self.profiles.as_ref().is_some_and(|p| p.draft.is_some()) {
            "save"
        } else if self.profiles.as_ref().is_some_and(|p| p.selected.is_some()) {
            "edit"
        } else {
            "new"
        };
        self.profile_action(id);
    }
    pub(super) fn profile_key(
        &mut self,
        key: &Key,
        modifiers: ModifiersState,
        repeat: bool,
    ) -> bool {
        if repeat {
            return false;
        }
        if matches!(key, Key::Named(NamedKey::Escape)) {
            self.profile_back();
            return true;
        }
        if matches!(key,Key::Character(s) if s.eq_ignore_ascii_case("s"))
            && (modifiers.control_key() || modifiers.super_key())
        {
            self.profile_action("save");
            return true;
        }
        if self.focus != Focus::Search
            && !modifiers.alt_key()
            && !modifiers.control_key()
            && !modifiers.super_key()
        {
            if matches!(key,Key::Character(s) if s.eq_ignore_ascii_case("c")) {
                self.profile_back();
                return true;
            }
            if matches!(key,Key::Character(s) if s.eq_ignore_ascii_case("n")||s.eq_ignore_ascii_case("r")||s.eq_ignore_ascii_case("e"))
            {
                if let (Key::Character(letter), Some(profiles)) = (key, &self.profiles) {
                    let library = profiles.draft.is_none() && profiles.selected.is_none();
                    let details = profiles.draft.is_none() && profiles.selected.is_some();
                    if letter.eq_ignore_ascii_case("n") && library {
                        self.profile_action("new");
                    } else if letter.eq_ignore_ascii_case("r") && library {
                        self.profile_action("refresh");
                    } else if letter.eq_ignore_ascii_case("e") && details {
                        self.profile_action("edit");
                    }
                }
                return true;
            }
        }
        if self.focus == Focus::Close
            && matches!(key, Key::Named(NamedKey::Enter | NamedKey::Space))
        {
            self.profile_back();
            return true;
        }
        false
    }
    pub(super) fn paint_profiles_footer(&self, canvas: &mut impl Canvas, theme: UiTheme) {
        let g = self.geometry;
        let font = self.font.max(10.0);
        let draft = self.profiles.as_ref().is_some_and(|p| p.draft.is_some());
        action_button(
            canvas,
            g.reset,
            if draft {
                ("Save", "Ctrl/Cmd+S")
            } else if self.profiles.as_ref().is_some_and(|p| p.selected.is_some()) {
                ("Edit profile", "E")
            } else {
                ("New profile", "N")
            },
            font,
            (
                self.focus == Focus::Reset,
                !self.profiles.as_ref().is_some_and(|p| p.busy),
            ),
            theme,
            g.card,
        );
        action_button(
            canvas,
            g.close,
            ("Back", "Esc"),
            font,
            (self.focus == Focus::Close, true),
            theme,
            g.card,
        );
        if self.status.is_empty() {
            shortcut_hint(canvas, g.status,
                "Tab: focus | Arrows: choose | Enter: open | Esc / Backspace / Alt+Left: back"
                , font * 0.76, theme, g.card);
        } else {
            label(
                canvas,
                g.status,
                &self.status,
                font * 0.76,
                theme.muted_text,
                false,
                g.card,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view() -> SettingsView {
        let mut v = SettingsView::default();
        v.open_profiles(ProfileDocument::default());
        v
    }
    #[test]
    fn profiles_clear_busy_status_and_reject_stale_form_edits() {
        let mut v = view();
        v.profile_notice("Working", true);
        v.receive_profiles(ProfileSnapshot::default());
        assert!(v.status.is_empty());
        assert!(!v.profiles.as_ref().unwrap().busy);
        v.profile_action("new");
        let revision = v.catalog.as_ref().unwrap().revision();
        v.profile_action("add-arg");
        v.pending = Some(Edit {
            revision,
            id: SettingId::new("profiles.name").unwrap(),
            change: Change::Set(SettingValue::Text("Stale".into())),
        });
        assert!(v.take_profile_intent().is_none());
        assert_eq!(
            v.profiles.as_ref().unwrap().draft.as_ref().unwrap().name,
            "New profile"
        );
        v.fit(1000.0, 800.0, 16.0);
        assert!(v.accessibility_summary().contains("Saving does not launch"));
    }
    #[test]
    fn profiles_edits_are_drafts_and_escape_requires_confirmation() {
        let mut v = view();
        v.profile_action("new");
        v.profile_action("add-arg");
        assert!(v
            .profiles
            .as_ref()
            .unwrap()
            .snapshot
            .document
            .profiles
            .is_empty());
        v.profile_back();
        assert!(v.confirmation.is_some());
        v.dismiss_confirmation();
        assert!(v.profiles.as_ref().unwrap().draft.is_some());
        v.confirm_profile(ProfileConfirmation::Discard);
        assert!(v.profiles.as_ref().unwrap().draft.is_none());
    }
    #[test]
    fn menu_back_profile_dirty_draft_requires_confirmation_and_does_not_close_library() {
        let mut v = view();
        v.fit(1000.0, 800.0, 16.0);
        v.profile_action("new");
        v.profile_action("add-arg");
        let back = Key::Named(NamedKey::ArrowLeft);
        v.key(&back, None, ModifiersState::ALT, false);
        assert!(v.confirmation.is_some());
        v.key(&back, None, ModifiersState::ALT, true);
        assert!(v.confirmation.is_some());
        v.key(&back, None, ModifiersState::ALT, false);
        assert!(v.confirmation.is_none());
        assert!(v.profiles.as_ref().unwrap().draft.is_some());
        assert!(v.profiles.as_ref().unwrap().dirty);
        assert!(v.take_profile_intent().is_none());
    }
    #[test]
    fn profile_save_in_progress_cannot_be_misrepresented_as_discarded() {
        let mut v = view();
        v.profile_action("new");
        v.profile_action("save");
        let Some(ProfileIntent::Io(ProfileOperation::Save { document, .. })) =
            v.take_profile_intent()
        else {
            panic!("expected explicit save");
        };
        let generation = v.profile_session();
        v.profile_notice("Working", true);
        v.profile_back();
        assert!(
            v.confirmation.is_none(),
            "an admitted save cannot be discarded"
        );
        assert_eq!(v.profile_session(), generation);
        assert!(v.profiles.as_ref().unwrap().busy);
        v.receive_profiles(ProfileSnapshot {
            document,
            bytes: None,
        });
        v.profile_back();
        assert!(v.profiles.is_none());
    }
    #[test]
    fn profile_launch_is_explicit_and_cancel_does_not_launch() {
        let mut v = view();
        v.profile_action("select.p0");
        assert!(v.take_profile_intent().is_none());
        v.profile_action("launch-tab");
        assert!(matches!(
            v.take_profile_intent(),
            Some(ProfileIntent::Launch(_, false))
        ));
        v.profile_back();
        assert!(v.take_profile_intent().is_none());
    }
    #[test]
    fn backing_out_of_launch_preparation_invalidates_the_request() {
        let mut v = view();
        v.profile_action("select.p0");
        v.profile_action("launch-tab");
        assert!(matches!(
            v.take_profile_intent(),
            Some(ProfileIntent::Launch(_, false))
        ));
        let submitted_generation = v.profile_session().unwrap();
        v.profile_notice("Working", true);
        v.profile_back();
        assert_ne!(v.profile_session(), Some(submitted_generation));
        assert!(!v.profiles.as_ref().unwrap().busy);
        assert!(v.profiles.as_ref().unwrap().selected.is_none());
        assert!(v.take_profile_intent().is_none());
    }
    #[test]
    fn profile_catalog_handles_all_bounded_fields_and_long_paths() {
        let mut v = view();
        v.profile_action("new");
        let p = v.profiles.as_mut().unwrap();
        let d = p.draft.as_mut().unwrap();
        d.program = Some("x".repeat(MAX_TEXT_BYTES));
        d.args = vec!["".into(); 64];
        d.environment = (0..64)
            .map(|i| EnvironmentReference {
                name: format!("KEY_{i}"),
                from: format!("SOURCE_{i}"),
            })
            .collect();
        assert!(p.catalog().is_ok());
    }
    #[test]
    fn import_is_a_fresh_draft_and_preserves_saved_collection() {
        let mut v = view();
        v.import_profiles(ProfileDocument {
            version: 1,
            profiles: vec![NamedProfile::new("default".into(), "Imported".into())],
        });
        let p = v.profiles.as_ref().unwrap();
        assert_ne!(p.draft.as_ref().unwrap().id, "default");
        assert!(p.snapshot.document.profiles.is_empty());
        v.profile_action("save");
        assert!(matches!(
            v.take_profile_intent(),
            Some(ProfileIntent::Io(ProfileOperation::Save { .. }))
        ));
    }
}
