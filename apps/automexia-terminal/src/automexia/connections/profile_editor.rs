//! Local SSH profile editing over the existing validated library. No launch authority.
use super::direct_openssh::prepare_literal_direct_openssh_typed;
use super::{
    ConnectionLibraryDocument, HubLibrarySnapshot, HubMetadataChangeState, LibraryEdit,
};
use automexia_connectivity::connections::{
    ConnectionProfileV1, IdentityKind, OpaqueReference, SourceKind, TransportDescriptor,
    MAX_PROFILES,
};
use automexia_ui_model::connection_hub::{AccessibilityNode, AccessibilityRole, HubKey};
use std::{collections::BTreeSet, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileFocus {
    List,
    Add,
    Edit,
    Remove,
    Close,
    Name,
    Host,
    User,
    Port,
    Source,
    Save,
    Cancel,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileAction {
    Key(HubKey),
    Select(usize),
    Focus(ProfileFocus),
    Add,
    Edit,
    Remove,
    ConfirmRemove,
    Cancel,
    Close,
    Save,
    CycleSource(bool),
    Append(String),
    Backspace,
    SelectAll,
}
pub enum ProfileEffect {
    None,
    Close,
    Save { revision: u64, edit: LibraryEdit },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileDraft {
    pub name: String,
    pub host: String,
    pub user: String,
    pub port: String,
    pub source_id: Option<String>,
    original: Option<ConnectionProfileV1>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileEditor {
    pub library: Arc<ConnectionLibraryDocument>,
    pub selected: usize,
    pub focus: ProfileFocus,
    pub draft: Option<ProfileDraft>,
    pub confirming_remove: bool,
    pub pending: Option<u64>,
    pub notice: Option<&'static str>,
    base_revision: u64,
    remove_id: Option<(String, u64)>,
    replace_text: bool,
}
impl ProfileEditor {
    pub fn new(snapshot: &HubLibrarySnapshot) -> Self {
        Self {
            library: Arc::clone(&snapshot.document),
            selected: 0,
            focus: ProfileFocus::List,
            draft: None,
            confirming_remove: false,
            pending: None,
            notice: None,
            base_revision: snapshot.revision,
            remove_id: None,
            replace_text: false,
        }
    }
    pub fn sync(
        &mut self,
        snapshot: &HubLibrarySnapshot,
        state: &HubMetadataChangeState,
    ) {
        if self.library.revision != snapshot.revision {
            let selected = self
                .library
                .profiles
                .profiles
                .get(self.selected)
                .map(|p| p.id.clone());
            self.library = Arc::clone(&snapshot.document);
            self.selected = selected
                .and_then(|id| {
                    self.library
                        .profiles
                        .profiles
                        .iter()
                        .position(|p| p.id == id)
                })
                .unwrap_or(self.selected)
                .min(self.library.profiles.profiles.len().saturating_sub(1));
        }
        let Some(pending) = self.pending else {
            return;
        };
        match state {
            HubMetadataChangeState::Applied { request, .. } if *request == pending => {
                self.pending = None;
                self.reset_draft();
                self.notice =
                    Some("Connection saved. Saving does not connect or run commands.");
            }
            HubMetadataChangeState::Conflict { request, .. } if *request == pending => {
                self.pending = None;
                self.notice =
                    Some("Connections changed elsewhere. Cancel and reopen this edit.");
            }
            HubMetadataChangeState::Error { request, .. } if *request == pending => {
                self.pending = None;
                self.notice = Some("Could not save. Check the fields; connections used by workspaces or other profiles cannot be removed.");
            }
            HubMetadataChangeState::Applied { request, .. }
            | HubMetadataChangeState::Applying { request }
            | HubMetadataChangeState::Conflict { request, .. }
            | HubMetadataChangeState::Error { request, .. }
                if *request > pending =>
            {
                self.pending = None;
                self.notice = Some("Another change completed. Cancel and reopen to verify the saved settings.");
            }
            _ => {}
        }
    }
    pub fn text_focused(&self) -> bool {
        self.pending.is_none()
            && self.draft.is_some()
            && matches!(
                self.focus,
                ProfileFocus::Name
                    | ProfileFocus::Host
                    | ProfileFocus::User
                    | ProfileFocus::Port
            )
    }
    pub fn source_label(&self) -> &str {
        match self.draft.as_ref().and_then(|d| d.source_id.as_ref()) {
            None => "OpenSSH default identity",
            Some(id) => self
                .library
                .credential_sources
                .sources
                .iter()
                .find(|s| &s.id == id)
                .map_or("Source unavailable; choose again", |s| {
                    s.display_name.as_str()
                }),
        }
    }
    fn reset_draft(&mut self) {
        self.draft = None;
        self.confirming_remove = false;
        self.remove_id = None;
        self.replace_text = false;
        self.focus = ProfileFocus::List;
    }
    fn editable(profile: &ConnectionProfileV1) -> bool {
        matches!(&profile.transport, TransportDescriptor::OpenSshExplicit { proxy_jump, .. } if proxy_jump.is_empty())
            && profile.jump_profile_references.is_empty()
            && profile.recipe_references.is_empty()
            && profile.tunnels.is_empty()
            && profile.capsule.public_environment.is_empty()
            && profile.capsule.context_references.is_empty()
            && profile.capsule.provider_contexts.is_empty()
            && (profile.identity.owner == "open-ssh"
                || profile.identity.owner
                    == super::library::CREDENTIAL_SOURCE_IDENTITY_OWNER)
    }
    fn save(&mut self) -> ProfileEffect {
        let Some(draft) = &self.draft else {
            return ProfileEffect::None;
        };
        if draft.name.trim().is_empty() {
            self.notice = Some("Enter a connection name.");
            return ProfileEffect::None;
        }
        let prepared = match prepare_literal_direct_openssh_typed(
            &draft.host,
            &draft.user,
            &draft.port,
        ) {
            Ok(prepared) => prepared,
            Err(error) => {
                self.notice = Some(error.diagnostic());
                return ProfileEffect::None;
            }
        };
        let mut profile = draft
            .original
            .clone()
            .unwrap_or_else(|| prepared.profile().clone());
        let expected = draft.original.as_ref().map(|p| p.revision);
        if let Some(revision) = expected {
            let Some(next) = revision.checked_add(1) else {
                self.notice = Some("Connection revision limit reached.");
                return ProfileEffect::None;
            };
            profile.revision = next;
        } else {
            if self.library.profiles.profiles.len() >= MAX_PROFILES {
                self.notice = Some("Saved connection limit reached.");
                return ProfileEffect::None;
            }
            profile.id = format!("saved-{}", self.base_revision);
            // Revisions are monotonically increasing, but imported IDs may coincide.
            let ids = self
                .library
                .profiles
                .profiles
                .iter()
                .map(|p| p.id.as_str())
                .collect::<BTreeSet<_>>();
            let Some(id) = (0..=MAX_PROFILES)
                .map(|n| format!("{}-{n}", profile.id))
                .find(|id| !ids.contains(id.as_str()))
            else {
                return ProfileEffect::None;
            };
            profile.id = id;
            profile.description.clear();
        }
        profile.display_name = draft.name.clone();
        profile.transport = prepared.profile().transport.clone();
        profile.public_target = prepared.profile().public_target.clone();
        profile.identity = prepared.profile().identity.clone();
        if let Some(id) = &draft.source_id {
            let Some(source) = self
                .library
                .credential_sources
                .sources
                .iter()
                .find(|s| &s.id == id)
            else {
                self.notice =
                    Some("The selected source is unavailable. Choose another source.");
                return ProfileEffect::None;
            };
            profile.identity.kind = IdentityKind::Agent;
            profile.identity.reference = OpaqueReference::new(source.id.clone());
            profile.identity.owner =
                super::library::CREDENTIAL_SOURCE_IDENTITY_OWNER.into();
            profile.identity.public_label = source.display_name.clone();
        }
        profile.source.kind = SourceKind::User;
        profile.source.reference = OpaqueReference::new("user-library");
        profile.source.revision = profile.revision.to_string();
        profile.approval_fingerprint = None;
        ProfileEffect::Save {
            revision: self.base_revision,
            edit: LibraryEdit::put_profile(expected, profile),
        }
    }
    pub fn apply(&mut self, action: ProfileAction) -> ProfileEffect {
        if self.pending.is_some() {
            return match action {
                ProfileAction::Close | ProfileAction::Key(HubKey::Escape) => {
                    ProfileEffect::Close
                }
                ProfileAction::Key(HubKey::Enter)
                    if self.focus == ProfileFocus::Close =>
                {
                    ProfileEffect::Close
                }
                ProfileAction::Focus(ProfileFocus::Close)
                | ProfileAction::Key(HubKey::Tab | HubKey::ShiftTab) => {
                    self.focus = ProfileFocus::Close;
                    ProfileEffect::None
                }
                _ => ProfileEffect::None,
            };
        }
        if self.confirming_remove
            && !matches!(
                action,
                ProfileAction::Key(_)
                    | ProfileAction::ConfirmRemove
                    | ProfileAction::Cancel
                    | ProfileAction::Close
                    | ProfileAction::Focus(ProfileFocus::Cancel | ProfileFocus::Remove)
            )
        {
            return ProfileEffect::None;
        }
        match action {
            ProfileAction::Key(key) => return self.key(key),
            ProfileAction::Close => return ProfileEffect::Close,
            ProfileAction::Cancel => {
                self.reset_draft();
                self.notice = None;
            }
            ProfileAction::Select(index)
                if self.draft.is_none()
                    && index < self.library.profiles.profiles.len() =>
            {
                self.selected = index;
                self.focus = ProfileFocus::List;
            }
            ProfileAction::Add if self.draft.is_none() => {
                self.base_revision = self.library.revision;
                self.replace_text = false;
                self.draft = Some(ProfileDraft {
                    name: String::new(),
                    host: String::new(),
                    user: String::new(),
                    port: String::new(),
                    source_id: None,
                    original: None,
                });
                self.focus = ProfileFocus::Name;
                self.notice = None;
            }
            ProfileAction::Edit if self.draft.is_none() => {
                if let Some(profile) = self.library.profiles.profiles.get(self.selected) {
                    if !Self::editable(profile) {
                        self.notice = Some("This profile uses advanced settings. Edit its existing library definition to preserve them.");
                        return ProfileEffect::None;
                    }
                    let TransportDescriptor::OpenSshExplicit {
                        host, user, port, ..
                    } = &profile.transport
                    else {
                        return ProfileEffect::None;
                    };
                    self.draft = Some(ProfileDraft {
                        name: profile.display_name.clone(),
                        host: host.clone(),
                        user: user.clone().unwrap_or_default(),
                        port: port.map(|p| p.to_string()).unwrap_or_default(),
                        source_id: (profile.identity.owner
                            == super::library::CREDENTIAL_SOURCE_IDENTITY_OWNER)
                            .then(|| profile.identity.reference.as_str().to_owned()),
                        original: Some(profile.clone()),
                    });
                    self.base_revision = self.library.revision;
                    self.focus = ProfileFocus::Name;
                    self.notice = None;
                    self.replace_text = false;
                }
            }
            ProfileAction::Remove if self.draft.is_none() => {
                if let Some(profile) = self.library.profiles.profiles.get(self.selected) {
                    self.remove_id = Some((profile.id.clone(), profile.revision));
                    self.base_revision = self.library.revision;
                    self.confirming_remove = true;
                    self.focus = ProfileFocus::Cancel;
                }
            }
            ProfileAction::ConfirmRemove if self.confirming_remove => {
                if let Some((id, revision)) = &self.remove_id {
                    return ProfileEffect::Save {
                        revision: self.base_revision,
                        edit: LibraryEdit::RemoveProfile {
                            expected_entity_revision: *revision,
                            profile_id: id.clone(),
                        },
                    };
                }
            }
            ProfileAction::Save => return self.save(),
            ProfileAction::Focus(focus) => {
                self.focus = focus;
                self.replace_text = false;
            }
            ProfileAction::CycleSource(forward) => {
                self.focus = ProfileFocus::Source;
                self.replace_text = false;
                if let Some(draft) = &mut self.draft {
                    let sources = &self.library.credential_sources.sources;
                    let index = draft
                        .source_id
                        .as_ref()
                        .and_then(|id| sources.iter().position(|s| &s.id == id))
                        .map_or(0, |i| i + 1);
                    let count = sources.len() + 1;
                    let next = (index + if forward { 1 } else { count - 1 }) % count;
                    draft.source_id = next
                        .checked_sub(1)
                        .and_then(|i| sources.get(i))
                        .map(|s| s.id.clone());
                }
            }
            ProfileAction::SelectAll if self.text_focused() => self.replace_text = true,
            ProfileAction::Append(value) if self.text_focused() => {
                if value
                    .chars()
                    .any(super::controller::unsafe_metadata_character)
                {
                    return ProfileEffect::None;
                }
                let replace = self.replace_text;
                if let Some((target, limit)) = self.text_mut() {
                    if (if replace { 0 } else { target.len() })
                        .saturating_add(value.len())
                        <= limit
                    {
                        if replace {
                            target.clear();
                        }
                        target.push_str(&value);
                        self.replace_text = false;
                    }
                }
            }
            ProfileAction::Backspace if self.text_focused() => {
                let replace = self.replace_text;
                if let Some((target, _)) = self.text_mut() {
                    if replace {
                        target.clear();
                    } else if let Some((last, _)) =
                        target.grapheme_indices(true).next_back()
                    {
                        target.truncate(last);
                    }
                }
                self.replace_text = false;
            }
            _ => {}
        }
        ProfileEffect::None
    }
    fn text_mut(&mut self) -> Option<(&mut String, usize)> {
        let draft = self.draft.as_mut()?;
        Some(match self.focus {
            ProfileFocus::Name => (&mut draft.name, 256),
            ProfileFocus::Host => (&mut draft.host, 253),
            ProfileFocus::User => (&mut draft.user, 128),
            ProfileFocus::Port => (&mut draft.port, 5),
            _ => return None,
        })
    }
    fn key(&mut self, key: HubKey) -> ProfileEffect {
        match key {
            HubKey::Escape if self.draft.is_some() || self.confirming_remove => {
                self.apply(ProfileAction::Cancel)
            }
            HubKey::Escape => ProfileEffect::Close,
            HubKey::Tab | HubKey::ShiftTab => {
                let order: &[ProfileFocus] = if self.confirming_remove {
                    &[ProfileFocus::Cancel, ProfileFocus::Remove]
                } else if self.draft.is_some() {
                    &[
                        ProfileFocus::Name,
                        ProfileFocus::Host,
                        ProfileFocus::User,
                        ProfileFocus::Port,
                        ProfileFocus::Source,
                        ProfileFocus::Save,
                        ProfileFocus::Cancel,
                    ]
                } else {
                    &[
                        ProfileFocus::List,
                        ProfileFocus::Add,
                        ProfileFocus::Edit,
                        ProfileFocus::Remove,
                        ProfileFocus::Close,
                    ]
                };
                let index = order.iter().position(|f| *f == self.focus).unwrap_or(0);
                self.focus = order[(index
                    + if key == HubKey::Tab {
                        1
                    } else {
                        order.len() - 1
                    })
                    % order.len()];
                self.replace_text = false;
                ProfileEffect::None
            }
            HubKey::Up | HubKey::Down if self.focus == ProfileFocus::List => {
                self.selected = if key == HubKey::Up {
                    self.selected.saturating_sub(1)
                } else {
                    self.selected
                        .saturating_add(1)
                        .min(self.library.profiles.profiles.len().saturating_sub(1))
                };
                ProfileEffect::None
            }
            HubKey::Enter => self.apply(match self.focus {
                ProfileFocus::List | ProfileFocus::Edit => ProfileAction::Edit,
                ProfileFocus::Add => ProfileAction::Add,
                ProfileFocus::Remove if self.confirming_remove => {
                    ProfileAction::ConfirmRemove
                }
                ProfileFocus::Remove => ProfileAction::Remove,
                ProfileFocus::Close => ProfileAction::Close,
                ProfileFocus::Source => ProfileAction::CycleSource(true),
                ProfileFocus::Save => ProfileAction::Save,
                ProfileFocus::Cancel => ProfileAction::Cancel,
                _ => return ProfileEffect::None,
            }),
            _ => ProfileEffect::None,
        }
    }
    pub fn accessibility_tree(&self) -> Vec<AccessibilityNode> {
        let mut controls: Vec<(String, String, ProfileFocus, AccessibilityRole)> =
            Vec::new();
        if self.confirming_remove {
            controls.push((
                "cancel".into(),
                "Cancel removal".into(),
                ProfileFocus::Cancel,
                AccessibilityRole::Button,
            ));
            controls.push((
                "remove".into(),
                "Confirm removal".into(),
                ProfileFocus::Remove,
                AccessibilityRole::Button,
            ));
        } else if let Some(draft) = &self.draft {
            for (id, value, focus) in [
                ("name", &draft.name, ProfileFocus::Name),
                ("host", &draft.host, ProfileFocus::Host),
                ("user", &draft.user, ProfileFocus::User),
                ("port", &draft.port, ProfileFocus::Port),
            ] {
                controls.push((
                    id.into(),
                    format!("{id}: {value}"),
                    focus,
                    AccessibilityRole::TextBox,
                ));
            }
            controls.push((
                "source".into(),
                format!("Credential source: {}", self.source_label()),
                ProfileFocus::Source,
                AccessibilityRole::Button,
            ));
            controls.push((
                "save".into(),
                "Save connection".into(),
                ProfileFocus::Save,
                AccessibilityRole::Button,
            ));
            controls.push((
                "cancel".into(),
                "Cancel edit".into(),
                ProfileFocus::Cancel,
                AccessibilityRole::Button,
            ));
        } else {
            for (index, profile) in self.library.profiles.profiles.iter().enumerate() {
                controls.push((
                    format!("row-{index}"),
                    profile.display_name.clone(),
                    ProfileFocus::List,
                    AccessibilityRole::Row,
                ));
            }
            for (id, focus) in [
                ("add", ProfileFocus::Add),
                ("edit", ProfileFocus::Edit),
                ("remove", ProfileFocus::Remove),
            ] {
                controls.push((id.into(), id.into(), focus, AccessibilityRole::Button));
            }
        }
        controls.push((
            "back".into(),
            "Back. Escape".into(),
            ProfileFocus::Close,
            AccessibilityRole::Button,
        ));
        let mut nodes = vec![AccessibilityNode {
            id: "saved-connections".into(),
            role: AccessibilityRole::Dialog,
            name: "Saved connections".into(),
            description: String::new(),
            modal: true,
            focusable: false,
            selected: false,
            disabled: false,
            live: false,
            actions: Vec::new(),
        }];
        nodes.extend(controls.into_iter().map(|(id, name, focus, role)| {
            AccessibilityNode {
                selected: focus == self.focus
                    && (role != AccessibilityRole::Row
                        || id == format!("row-{}", self.selected)),
                id: format!("saved-connection-{id}"),
                role,
                name,
                description: String::new(),
                modal: false,
                focusable: self.pending.is_none() || focus == ProfileFocus::Close,
                disabled: self.pending.is_some() && focus != ProfileFocus::Close,
                live: false,
                actions: vec!["activate".into()],
            }
        }));
        if let Some(notice) = self.notice {
            nodes.push(AccessibilityNode {
                id: "saved-connection-notice".into(),
                role: AccessibilityRole::Status,
                name: notice.into(),
                description: String::new(),
                modal: false,
                focusable: false,
                selected: false,
                disabled: false,
                live: true,
                actions: Vec::new(),
            });
        }
        nodes
    }
}
