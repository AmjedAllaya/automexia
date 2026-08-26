//! Translation between capability-free session-helper records and the
//! authenticated application suggestion model.
//!
//! This owner performs no IO and never returns a PTY write. Route authority is
//! supplied once by the application, not by the shell-facing record.

use std::fmt;

use automexia_command_productivity::suggestions::helper::{
    HelperRecordError, HelperReplace, HelperRequest,
};
use automexia_command_productivity::suggestions::{
    AcceptanceBindings, AcceptanceContext, Candidate, CandidateFreshness, CandidateKind,
    CandidateRisk, CandidateSource, CompletionMode, EditorRequest, EditorSubmission,
    NativeEditorReplacement, RankInput, RequestReason, RouteIdentity, SourceBatch,
    SuggestionCapability, PROTOCOL_SCHEMA,
};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub struct HelperSessionBinding {
    pub route: RouteIdentity,
    pub capability: SuggestionCapability,
    pub prompt_generation: u64,
    pub source_revision: u64,
}

impl fmt::Debug for HelperSessionBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HelperSessionBinding")
            .field("route", &self.route)
            .field("capability", &"[REDACTED]")
            .field("prompt_generation", &self.prompt_generation)
            .field("source_revision", &self.source_revision)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelperSessionError {
    InvalidBinding,
    Record(HelperRecordError),
    StaleGeneration,
    IdentityExhausted,
    InvalidSubmission,
    InvalidReplacement,
    NoActiveRequest,
}

impl fmt::Display for HelperSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidBinding => "suggestion helper binding is invalid",
            Self::Record(_) => "suggestion helper record is invalid",
            Self::StaleGeneration => "suggestion helper generation is stale",
            Self::IdentityExhausted => "suggestion helper identity space is exhausted",
            Self::InvalidSubmission => "suggestion helper submission is invalid",
            Self::InvalidReplacement => "suggestion helper replacement is invalid",
            Self::NoActiveRequest => "suggestion helper has no active request",
        })
    }
}

impl std::error::Error for HelperSessionError {}

impl From<HelperRecordError> for HelperSessionError {
    fn from(error: HelperRecordError) -> Self {
        Self::Record(error)
    }
}

/// One route-bound translator with a single current editor generation.
pub struct HelperSessionBridge {
    binding: HelperSessionBinding,
    next_request_id: u64,
    last_adapter_generation: u64,
    active: Option<EditorRequest>,
}

impl HelperSessionBridge {
    pub fn new(binding: HelperSessionBinding) -> Result<Self, HelperSessionError> {
        if binding.route.application_generation == 0
            || binding.route.window_id == 0
            || binding.route.tab_id == 0
            || binding.route.pane_id == 0
            || binding.route.session_id == 0
            || binding.route.endpoint_instance == 0
            || binding.route.editor_version.is_empty()
            || !binding.capability.is_valid()
            || binding.prompt_generation == 0
            || binding.source_revision == 0
        {
            return Err(HelperSessionError::InvalidBinding);
        }
        Ok(Self {
            binding,
            next_request_id: 1,
            last_adapter_generation: 0,
            active: None,
        })
    }

    pub fn translate_request(
        &mut self,
        helper: HelperRequest,
    ) -> Result<EditorSubmission, HelperSessionError> {
        helper.validate()?;
        if helper.adapter_generation <= self.last_adapter_generation {
            return Err(HelperSessionError::StaleGeneration);
        }
        let request_id = self.next_request_id;
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .ok_or(HelperSessionError::IdentityExhausted)?;
        let route = &self.binding.route;
        let request = EditorRequest {
            schema: PROTOCOL_SCHEMA,
            request_id,
            application_generation: route.application_generation,
            window_id: route.window_id,
            tab_id: route.tab_id,
            pane_id: route.pane_id,
            session_id: route.session_id,
            shell: route.shell,
            editor_version: route.editor_version.clone(),
            endpoint_instance: route.endpoint_instance,
            capability: self.binding.capability,
            prompt_generation: self.binding.prompt_generation,
            buffer_generation: helper.adapter_generation,
            cursor_grapheme: helper.buffer[..helper.cursor_byte].graphemes(true).count(),
            buffer: helper.buffer,
            cursor_byte: helper.cursor_byte,
            selection: helper.selection,
            replacement_span: helper.replacement_span,
            quote_context: helper.quote_context,
            token_context: "native-editor".into(),
            cwd: None,
            completion_mode: CompletionMode::Explicit,
            source_revision: self.binding.source_revision,
            reason: RequestReason::UserRequested,
            cancellation_id: request_id,
        };

        let candidates = helper
            .native_candidates
            .into_iter()
            .enumerate()
            .map(|(index, insertion)| {
                let candidate_id = u64::try_from(index)
                    .ok()
                    .and_then(|index| index.checked_add(1))
                    .ok_or(HelperSessionError::IdentityExhausted)?;
                let native_rank = u16::try_from(index)
                    .map_err(|_| HelperSessionError::IdentityExhausted)?;
                Ok(RankInput::new(
                    Candidate {
                        request_id,
                        candidate_id,
                        display: insertion.clone(),
                        insertion,
                        description: String::new(),
                        kind: CandidateKind::Context,
                        source: CandidateSource::NativeShell,
                        freshness: CandidateFreshness::Current,
                        replacement_span: request.replacement_span,
                        quoting: request.quote_context,
                        public_context: false,
                        risk: CandidateRisk::Unknown,
                    },
                    Some(native_rank),
                    0,
                ))
            })
            .collect::<Result<Vec<_>, HelperSessionError>>()?;
        let batch = SourceBatch::new(
            CandidateSource::NativeShell,
            self.binding.source_revision,
            CandidateFreshness::Current,
            candidates,
        )
        .map_err(|_| HelperSessionError::InvalidSubmission)?;
        let submission = EditorSubmission {
            schema: PROTOCOL_SCHEMA,
            request: request.clone(),
            batches: vec![batch],
            acceptance: AcceptanceBindings::default(),
        };
        submission
            .validate()
            .map_err(|_| HelperSessionError::InvalidSubmission)?;
        self.last_adapter_generation = helper.adapter_generation;
        self.active = Some(request);
        Ok(submission)
    }

    pub fn translate_replacement(
        &mut self,
        adapter_generation: u64,
        replacement: &NativeEditorReplacement,
    ) -> Result<HelperReplace, HelperSessionError> {
        let request = self
            .active
            .as_ref()
            .ok_or(HelperSessionError::NoActiveRequest)?;
        if adapter_generation != self.last_adapter_generation
            || adapter_generation != request.buffer_generation
        {
            return Err(HelperSessionError::StaleGeneration);
        }
        let current = AcceptanceContext::from_request(request);
        let insertion = replacement
            .revalidate(&current, &self.binding.capability)
            .map_err(|_| HelperSessionError::InvalidReplacement)?;
        let insertion = String::from_utf8(insertion.bytes)
            .map_err(|_| HelperSessionError::InvalidReplacement)?;
        let translated = HelperReplace {
            adapter_generation,
            request_id: request.request_id,
            replacement_span: replacement.replacement_span,
            insertion,
        };
        translated.validate()?;
        self.active = None;
        Ok(translated)
    }

    pub fn dismiss(&mut self, adapter_generation: u64) -> bool {
        if self
            .active
            .as_ref()
            .is_some_and(|request| request.buffer_generation == adapter_generation)
        {
            self.active = None;
            true
        } else {
            false
        }
    }
}

impl fmt::Debug for HelperSessionBridge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HelperSessionBridge")
            .field("binding", &self.binding)
            .field("next_request_id", &self.next_request_id)
            .field("last_adapter_generation", &self.last_adapter_generation)
            .field("active", &self.active.is_some())
            .finish()
    }
}
