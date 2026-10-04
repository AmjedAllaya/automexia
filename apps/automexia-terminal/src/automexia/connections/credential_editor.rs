//! Route-local credential-source editing; no vault or process authority.

use automexia_connectivity::connections::{
    validate_credential_source, CredentialProvider, CredentialSourceV1, SshAgentEndpoint,
};
use automexia_ui_model::connection_hub::HubKey;

use super::{HubLibrarySnapshot, LibraryEdit};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialFocus {
    List,
    Add,
    Edit,
    Close,
    Name,
    Provider,
    Endpoint,
    Save,
    Cancel,
    Remove,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CredentialAction {
    Select(usize),
    Add,
    Edit,
    Remove,
    ConfirmRemove,
    Cancel,
    Close,
    Focus(CredentialFocus),
    NextProvider,
    PreviousProvider,
    Save,
    Append(String),
    Backspace,
    SelectAll,
    Key(HubKey),
}

pub enum CredentialEffect {
    None,
    Close,
    Save { revision: u64, edit: LibraryEdit },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialEditor {
    pub sources: Vec<CredentialSourceV1>,
    pub selected: usize,
    pub focus: CredentialFocus,
    pub draft: Option<CredentialSourceV1>,
    pub confirming_remove: bool,
    pub notice: Option<&'static str>,
    pub pending: Option<u64>,
    revision: u64,
    editing_revision: Option<u64>,
    draft_base_revision: u64,
    replace_text: bool,
}

impl CredentialEditor {
    pub fn new(library: &HubLibrarySnapshot) -> Self {
        Self {
            sources: library.document.credential_sources.sources.clone(),
            selected: 0,
            focus: CredentialFocus::List,
            draft: None,
            confirming_remove: false,
            notice: None,
            pending: None,
            revision: library.revision,
            editing_revision: None,
            draft_base_revision: library.revision,
            replace_text: false,
        }
    }

    pub fn sync(
        &mut self,
        library: &HubLibrarySnapshot,
        state: &super::HubMetadataChangeState,
    ) {
        if self.revision != library.revision {
            let selected = self
                .sources
                .get(self.selected)
                .map(|source| source.id.clone());
            self.sources
                .clone_from(&library.document.credential_sources.sources);
            self.selected = selected
                .and_then(|id| self.sources.iter().position(|source| source.id == id))
                .unwrap_or(self.selected)
                .min(self.sources.len().saturating_sub(1));
            self.revision = library.revision;
        }
        if let Some(pending) = self.pending {
            match state {
                super::HubMetadataChangeState::Applied { request, .. }
                    if *request == pending =>
                {
                    self.pending = None;
                    self.draft = None;
                    self.confirming_remove = false;
                    self.focus = CredentialFocus::List;
                    self.notice = Some("Credential source saved. Unlock and manage keys in your vault app.");
                }
                super::HubMetadataChangeState::Conflict { request, .. }
                    if *request == pending =>
                {
                    self.pending = None;
                    self.notice = Some("Sources changed elsewhere. Cancel and reopen this edit before saving.");
                }
                super::HubMetadataChangeState::Error { request, .. }
                    if *request == pending =>
                {
                    self.pending = None;
                    self.notice = Some("Could not save. A source used by a connection cannot be removed.");
                }
                super::HubMetadataChangeState::Applied { request, .. }
                | super::HubMetadataChangeState::Applying { request }
                | super::HubMetadataChangeState::Conflict { request, .. }
                | super::HubMetadataChangeState::Error { request, .. }
                    if *request > pending =>
                {
                    self.pending = None;
                    self.notice = Some("Another change completed. Cancel and reopen to verify the saved settings.");
                }
                _ => {}
            }
        }
    }

    pub fn text_focused(&self) -> bool {
        self.draft.is_some()
            && (self.focus == CredentialFocus::Name
                || (!cfg!(windows) && self.focus == CredentialFocus::Endpoint))
    }

    pub fn accessibility_tree(
        &self,
    ) -> Vec<automexia_ui_model::connection_hub::AccessibilityNode> {
        use automexia_ui_model::connection_hub::{
            AccessibilityNode, AccessibilityRole as Role,
        };
        let node = |id: &str, role, name: String, focus: Option<CredentialFocus>| {
            AccessibilityNode {
                id: id.into(),
                role,
                name,
                description: String::new(),
                modal: role == Role::Dialog,
                focusable: focus.is_some()
                    && (self.pending.is_none() || focus == Some(CredentialFocus::Close)),
                selected: focus == Some(self.focus),
                disabled: self.pending.is_some() && focus != Some(CredentialFocus::Close),
                live: role == Role::Status,
                actions: if focus.is_some() {
                    vec!["activate".into()]
                } else {
                    Vec::new()
                },
            }
        };
        let mut nodes = vec![node(
            "credential-sources",
            Role::Dialog,
            "Credential sources".into(),
            None,
        )];
        if self.confirming_remove {
            nodes.push(node("credential-remove-confirmation", Role::Alert, "Remove the source from Automexia? The vault and credentials stay unchanged.".into(), None));
            nodes.push(node(
                "credential-remove-cancel",
                Role::Button,
                "Cancel removal".into(),
                Some(CredentialFocus::Cancel),
            ));
            nodes.push(node(
                "credential-remove",
                Role::Button,
                "Confirm removal".into(),
                Some(CredentialFocus::Remove),
            ));
        } else if let Some(draft) = &self.draft {
            nodes.push(node(
                "credential-name",
                Role::TextBox,
                format!("Name: {}", draft.display_name),
                Some(CredentialFocus::Name),
            ));
            nodes.push(node(
                "credential-provider",
                Role::Button,
                format!(
                    "Vault: {}. Left and right arrows change provider.",
                    draft.provider.label()
                ),
                Some(CredentialFocus::Provider),
            ));
            let endpoint = match &draft.endpoint {
                SshAgentEndpoint::System => "System SSH agent",
                SshAgentEndpoint::UnixSocket { path } => path,
            };
            if !cfg!(windows) {
                nodes.push(node(
                    "credential-endpoint",
                    Role::TextBox,
                    format!("Agent socket: {endpoint}"),
                    Some(CredentialFocus::Endpoint),
                ));
            }
            nodes.push(node(
                "credential-save",
                Role::Button,
                "Save credential source".into(),
                Some(CredentialFocus::Save),
            ));
            nodes.push(node(
                "credential-cancel",
                Role::Button,
                "Cancel edit".into(),
                Some(CredentialFocus::Cancel),
            ));
        } else {
            for (index, source) in self.sources.iter().enumerate() {
                let mut row = node(
                    &format!("credential-row-{index}"),
                    Role::Row,
                    format!("{}: {}", source.display_name, source.provider.label()),
                    Some(CredentialFocus::List),
                );
                row.selected = self.selected == index;
                nodes.push(row);
            }
            nodes.push(node(
                "credential-add",
                Role::Button,
                "Add source. N".into(),
                Some(CredentialFocus::Add),
            ));
            nodes.push(node(
                "credential-edit",
                Role::Button,
                "Edit selected source. Enter".into(),
                Some(CredentialFocus::Edit),
            ));
            nodes.push(node(
                "credential-delete",
                Role::Button,
                "Remove selected source. Delete".into(),
                Some(CredentialFocus::Remove),
            ));
        }
        nodes.push(node(
            "credential-back",
            Role::Button,
            "Back. Escape".into(),
            Some(CredentialFocus::Close),
        ));
        if let Some(notice) = self.notice {
            nodes.push(node("credential-status", Role::Status, notice.into(), None));
        }
        nodes
    }

    pub fn apply(&mut self, action: CredentialAction) -> CredentialEffect {
        if self.pending.is_some() {
            return match action {
                CredentialAction::Close | CredentialAction::Key(HubKey::Escape) => {
                    CredentialEffect::Close
                }
                CredentialAction::Key(HubKey::Enter)
                    if self.focus == CredentialFocus::Close =>
                {
                    CredentialEffect::Close
                }
                CredentialAction::Focus(CredentialFocus::Close)
                | CredentialAction::Key(HubKey::Tab | HubKey::ShiftTab) => {
                    self.focus = CredentialFocus::Close;
                    CredentialEffect::None
                }
                _ => CredentialEffect::None,
            };
        }
        if self.confirming_remove
            && !matches!(
                action,
                CredentialAction::Key(_)
                    | CredentialAction::Cancel
                    | CredentialAction::Close
                    | CredentialAction::ConfirmRemove
                    | CredentialAction::Focus(
                        CredentialFocus::Cancel | CredentialFocus::Remove
                    )
            )
        {
            return CredentialEffect::None;
        }
        match action {
            CredentialAction::Key(key) => return self.key(key),
            CredentialAction::Close => return CredentialEffect::Close,
            CredentialAction::Select(index)
                if self.draft.is_none() && index < self.sources.len() =>
            {
                self.selected = index;
                self.focus = CredentialFocus::List;
            }
            CredentialAction::Add => {
                self.replace_text = false;
                let Some(id) = (0..=64)
                    .map(|offset| format!("vault-{}-{offset}", self.revision))
                    .find(|id| !self.sources.iter().any(|source| &source.id == id))
                else {
                    return CredentialEffect::None;
                };
                if self.sources.len() >= 64 {
                    self.notice = Some("Up to 64 credential sources can be saved.");
                    return CredentialEffect::None;
                }
                self.draft = Some(CredentialSourceV1 {
                    schema_version: 1,
                    id,
                    revision: 1,
                    display_name: "My vault".into(),
                    provider: CredentialProvider::SystemSshAgent,
                    endpoint: SshAgentEndpoint::System,
                });
                self.editing_revision = None;
                self.draft_base_revision = self.revision;
                self.focus = CredentialFocus::Name;
                self.notice = None;
            }
            CredentialAction::Edit => {
                self.replace_text = false;
                if let Some(source) = self.sources.get(self.selected) {
                    let Some(next) = source.revision.checked_add(1) else {
                        return CredentialEffect::None;
                    };
                    self.editing_revision = Some(source.revision);
                    let mut draft = source.clone();
                    draft.revision = next;
                    self.draft = Some(draft);
                    self.draft_base_revision = self.revision;
                    self.focus = CredentialFocus::Name;
                    self.notice = None;
                }
            }
            CredentialAction::Remove if self.sources.get(self.selected).is_some() => {
                self.confirming_remove = true;
                self.focus = CredentialFocus::Cancel;
                self.draft_base_revision = self.revision;
            }
            CredentialAction::ConfirmRemove if self.confirming_remove => {
                if let Some(source) = self.sources.get(self.selected) {
                    return CredentialEffect::Save {
                        revision: self.draft_base_revision,
                        edit: LibraryEdit::RemoveCredentialSource {
                            expected_entity_revision: source.revision,
                            source_id: source.id.clone(),
                        },
                    };
                }
            }
            CredentialAction::Cancel => {
                self.replace_text = false;
                self.draft = None;
                self.confirming_remove = false;
                self.focus = CredentialFocus::List;
                self.notice = None;
            }
            CredentialAction::Focus(focus) => {
                self.focus = focus;
                self.replace_text = false;
            }
            CredentialAction::SelectAll if self.text_focused() => {
                self.replace_text = true
            }
            CredentialAction::NextProvider | CredentialAction::PreviousProvider => {
                self.focus = CredentialFocus::Provider;
                self.replace_text = false;
                if let Some(draft) = &mut self.draft {
                    let position = CredentialProvider::ALL
                        .iter()
                        .position(|provider| *provider == draft.provider)
                        .unwrap_or(0);
                    let step = if action == CredentialAction::NextProvider {
                        1
                    } else {
                        CredentialProvider::ALL.len() - 1
                    };
                    draft.provider = CredentialProvider::ALL
                        [(position + step) % CredentialProvider::ALL.len()];
                }
            }
            CredentialAction::Append(value) if self.text_focused() => {
                if value
                    .chars()
                    .any(super::controller::unsafe_metadata_character)
                {
                    return CredentialEffect::None;
                }
                if let Some(draft) = &mut self.draft {
                    let target = match self.focus {
                        CredentialFocus::Name => &mut draft.display_name,
                        CredentialFocus::Endpoint if !cfg!(windows) => {
                            if matches!(draft.endpoint, SshAgentEndpoint::System) {
                                draft.endpoint = SshAgentEndpoint::UnixSocket {
                                    path: String::new(),
                                };
                            }
                            let SshAgentEndpoint::UnixSocket { path } =
                                &mut draft.endpoint
                            else {
                                return CredentialEffect::None;
                            };
                            path
                        }
                        _ => return CredentialEffect::None,
                    };
                    let limit = if self.focus == CredentialFocus::Name {
                        256
                    } else {
                        1024
                    };
                    if (if self.replace_text { 0 } else { target.len() })
                        .saturating_add(value.len())
                        <= limit
                    {
                        if self.replace_text {
                            target.clear();
                        }
                        target.push_str(&value);
                        self.replace_text = false;
                    }
                }
            }
            CredentialAction::Backspace if self.text_focused() => {
                if let Some(draft) = &mut self.draft {
                    let target = match self.focus {
                        CredentialFocus::Name => &mut draft.display_name,
                        CredentialFocus::Endpoint => {
                            let SshAgentEndpoint::UnixSocket { path } =
                                &mut draft.endpoint
                            else {
                                return CredentialEffect::None;
                            };
                            path
                        }
                        _ => return CredentialEffect::None,
                    };
                    if self.replace_text {
                        target.clear();
                        self.replace_text = false;
                    } else if let Some((last, _)) =
                        target.grapheme_indices(true).next_back()
                    {
                        target.truncate(last);
                    }
                    if matches!(&draft.endpoint, SshAgentEndpoint::UnixSocket { path } if path.is_empty())
                    {
                        draft.endpoint = SshAgentEndpoint::System;
                    }
                }
            }
            CredentialAction::Save => {
                if let Some(draft) = &self.draft {
                    if validate_credential_source(draft).is_err() {
                        self.notice = Some("Enter a name and a valid absolute agent socket path, or leave the socket empty for the system agent.");
                    } else {
                        return CredentialEffect::Save {
                            revision: self.draft_base_revision,
                            edit: LibraryEdit::PutCredentialSource {
                                expected_entity_revision: self.editing_revision,
                                source: draft.clone(),
                            },
                        };
                    }
                }
            }
            _ => {}
        }
        CredentialEffect::None
    }

    fn key(&mut self, key: HubKey) -> CredentialEffect {
        match key {
            HubKey::Escape if self.draft.is_some() || self.confirming_remove => {
                self.apply(CredentialAction::Cancel)
            }
            HubKey::Escape => CredentialEffect::Close,
            HubKey::Tab | HubKey::ShiftTab => {
                let order: &[CredentialFocus] = if self.confirming_remove {
                    &[CredentialFocus::Cancel, CredentialFocus::Remove]
                } else if self.draft.is_some() {
                    if cfg!(windows) {
                        &[
                            CredentialFocus::Name,
                            CredentialFocus::Provider,
                            CredentialFocus::Save,
                            CredentialFocus::Cancel,
                        ]
                    } else {
                        &[
                            CredentialFocus::Name,
                            CredentialFocus::Provider,
                            CredentialFocus::Endpoint,
                            CredentialFocus::Save,
                            CredentialFocus::Cancel,
                        ]
                    }
                } else {
                    &[
                        CredentialFocus::List,
                        CredentialFocus::Add,
                        CredentialFocus::Edit,
                        CredentialFocus::Remove,
                        CredentialFocus::Close,
                    ]
                };
                let index = order
                    .iter()
                    .position(|focus| *focus == self.focus)
                    .unwrap_or(0);
                let step = if key == HubKey::Tab {
                    1
                } else {
                    order.len() - 1
                };
                self.focus = order[(index + step) % order.len()];
                self.replace_text = false;
                CredentialEffect::None
            }
            HubKey::Up | HubKey::Down if self.focus == CredentialFocus::List => {
                self.selected = if key == HubKey::Up {
                    self.selected.saturating_sub(1)
                } else {
                    self.selected
                        .saturating_add(1)
                        .min(self.sources.len().saturating_sub(1))
                };
                CredentialEffect::None
            }
            HubKey::Enter => self.apply(match self.focus {
                CredentialFocus::List | CredentialFocus::Edit => CredentialAction::Edit,
                CredentialFocus::Add => CredentialAction::Add,
                CredentialFocus::Close => CredentialAction::Close,
                CredentialFocus::Provider => CredentialAction::NextProvider,
                CredentialFocus::Save => CredentialAction::Save,
                CredentialFocus::Cancel => CredentialAction::Cancel,
                CredentialFocus::Remove => {
                    if self.confirming_remove {
                        CredentialAction::ConfirmRemove
                    } else {
                        CredentialAction::Remove
                    }
                }
                _ => return CredentialEffect::None,
            }),
            _ => CredentialEffect::None,
        }
    }
}
