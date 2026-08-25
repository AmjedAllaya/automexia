//! Pure, capability-free models for the optional CP5 native-editor suggestion path.
//!
//! This module performs no filesystem, process, network, credential, PTY,
//! clipboard, terminal-output, persistence, or renderer work. Native editor and
//! application adapters must validate these models at both ingress and
//! acceptance.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

pub const PROTOCOL_SCHEMA: u16 = 1;

pub struct SuggestionLimits;

impl SuggestionLimits {
    pub const BUFFER_BYTES: usize = 16 * 1024;
    pub const FRAME_BYTES: usize = 1024 * 1024;
    pub const CANDIDATE_COUNT: usize = 512;
    pub const CANDIDATE_BYTES: usize = 1024;
    pub const BATCH_BYTES: usize = 512 * 1024;
    pub const VISIBLE_ROWS: usize = 12;
    pub const ACTIVE_ROUTES: usize = 64;
    pub const CACHE_BYTES: usize = 8 * 1024 * 1024;
    pub const SOURCE_DEADLINE_MS: u64 = 250;
    pub const TOKEN_CONTEXT_BYTES: usize = 256;
    pub const EDITOR_VERSION_BYTES: usize = 64;
    pub const CWD_BYTES: usize = 4096;
    pub const DESCRIPTION_BYTES: usize = 1024;
    pub const FREQUENCY_IDS: usize = 4096;
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SuggestionCapability([u8; 32]);

impl SuggestionCapability {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    pub fn is_valid(&self) -> bool {
        !self.is_zero()
    }

    #[must_use]
    pub fn constant_time_eq(&self, other: &Self) -> bool {
        let mut difference = 0_u8;
        for (left, right) in self.0.iter().zip(other.0.iter()) {
            difference |= left ^ right;
        }
        difference == 0
    }

    fn is_zero(&self) -> bool {
        let mut combined = 0_u8;
        for value in self.0 {
            combined |= value;
        }
        combined == 0
    }
}

impl fmt::Debug for SuggestionCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SuggestionCapability([REDACTED; 32])")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShellKind {
    PowerShell,
    Bash,
    Zsh,
    Fish,
    Wsl,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteIdentity {
    pub application_generation: u64,
    pub window_id: u64,
    pub tab_id: u64,
    pub pane_id: u64,
    pub session_id: u64,
    pub shell: ShellKind,
    pub editor_version: String,
    pub endpoint_instance: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplacementSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QuoteContext {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
    Escaped,
    ShellSpecific,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompletionMode {
    Automatic,
    Explicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RequestReason {
    BufferChanged,
    CursorMoved,
    UserRequested,
    SourceRefreshed,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorRequest {
    pub schema: u16,
    pub request_id: u64,
    pub application_generation: u64,
    pub window_id: u64,
    pub tab_id: u64,
    pub pane_id: u64,
    pub session_id: u64,
    pub shell: ShellKind,
    pub editor_version: String,
    pub endpoint_instance: u64,
    pub capability: SuggestionCapability,
    pub prompt_generation: u64,
    pub buffer_generation: u64,
    pub buffer: String,
    pub cursor_byte: usize,
    pub cursor_grapheme: usize,
    pub selection: Option<ReplacementSpan>,
    pub replacement_span: ReplacementSpan,
    pub quote_context: QuoteContext,
    pub token_context: String,
    pub cwd: Option<String>,
    pub completion_mode: CompletionMode,
    pub source_revision: u64,
    pub reason: RequestReason,
    pub cancellation_id: u64,
}

impl EditorRequest {
    pub fn route(&self) -> RouteIdentity {
        RouteIdentity {
            application_generation: self.application_generation,
            window_id: self.window_id,
            tab_id: self.tab_id,
            pane_id: self.pane_id,
            session_id: self.session_id,
            shell: self.shell,
            editor_version: self.editor_version.clone(),
            endpoint_instance: self.endpoint_instance,
        }
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.schema != PROTOCOL_SCHEMA {
            return Err(ValidationError::Schema);
        }
        if [
            self.request_id,
            self.application_generation,
            self.window_id,
            self.tab_id,
            self.pane_id,
            self.session_id,
            self.endpoint_instance,
            self.prompt_generation,
            self.buffer_generation,
            self.source_revision,
            self.cancellation_id,
        ]
        .contains(&0)
        {
            return Err(ValidationError::Identity);
        }
        if self.capability.is_zero() {
            return Err(ValidationError::Capability);
        }
        if !bounded_plain(
            &self.editor_version,
            SuggestionLimits::EDITOR_VERSION_BYTES,
            false,
        ) {
            return Err(ValidationError::EditorVersion);
        }
        if self.buffer.len() > SuggestionLimits::BUFFER_BYTES
            || self.buffer.contains('\0')
        {
            return Err(ValidationError::Buffer);
        }
        if self.cursor_byte > self.buffer.len()
            || !self.buffer.is_char_boundary(self.cursor_byte)
            || !is_grapheme_boundary(&self.buffer, self.cursor_byte)
        {
            return Err(ValidationError::Cursor);
        }
        let actual_graphemes = self.buffer[..self.cursor_byte].graphemes(true).count();
        if actual_graphemes != self.cursor_grapheme {
            return Err(ValidationError::Cursor);
        }
        validate_span(&self.buffer, self.replacement_span)?;
        if self.replacement_span.start > self.cursor_byte
            || self.replacement_span.end < self.cursor_byte
        {
            return Err(ValidationError::ReplacementSpan);
        }
        if let Some(selection) = self.selection {
            validate_span(&self.buffer, selection)?;
        }
        if !bounded_plain(
            &self.token_context,
            SuggestionLimits::TOKEN_CONTEXT_BYTES,
            true,
        ) {
            return Err(ValidationError::TokenContext);
        }
        if self.cwd.as_deref().is_some_and(|cwd| {
            cwd.len() > SuggestionLimits::CWD_BYTES || cwd.contains('\0')
        }) {
            return Err(ValidationError::Cwd);
        }
        Ok(())
    }

    pub fn authenticate(
        &self,
        route: &RouteIdentity,
        capability: &SuggestionCapability,
        last_request_id: u64,
    ) -> Result<(), ValidationError> {
        self.validate()?;
        if &self.route() != route {
            return Err(ValidationError::Route);
        }
        if !self.capability.constant_time_eq(capability) {
            return Err(ValidationError::Capability);
        }
        if self.request_id <= last_request_id {
            return Err(ValidationError::Replay);
        }
        Ok(())
    }
    fn query(&self) -> &str {
        &self.buffer[self.replacement_span.start..self.cursor_byte]
    }
}

impl fmt::Debug for EditorRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EditorRequest")
            .field("schema", &self.schema)
            .field("request_id", &self.request_id)
            .field("route", &self.route())
            .field("prompt_generation", &self.prompt_generation)
            .field("buffer_generation", &self.buffer_generation)
            .field("buffer_bytes", &self.buffer.len())
            .field("cursor_byte", &self.cursor_byte)
            .field("cursor_grapheme", &self.cursor_grapheme)
            .field("replacement_span", &self.replacement_span)
            .field("cwd_present", &self.cwd.is_some())
            .field("source_revision", &self.source_revision)
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationError {
    Schema,
    Identity,
    Route,
    Capability,
    Replay,
    EditorVersion,
    Buffer,
    Cursor,
    Selection,
    ReplacementSpan,
    TokenContext,
    Cwd,
    CandidateIdentity,
    CandidateText,
    CandidateSource,
    CandidateCount,
    CandidateBatch,
    StaleAcceptance,
    RouteLimit,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Schema => "unsupported suggestion schema",
            Self::Identity => "invalid suggestion identity",
            Self::Route => "suggestion route mismatch",
            Self::Capability => "suggestion capability mismatch",
            Self::Replay => "replayed suggestion request",
            Self::EditorVersion => "invalid editor version",
            Self::Buffer => "invalid or oversized editor buffer",
            Self::Cursor => "invalid editor cursor",
            Self::Selection => "invalid editor selection",
            Self::ReplacementSpan => "invalid replacement span",
            Self::TokenContext => "invalid token context",
            Self::Cwd => "invalid working directory",
            Self::CandidateIdentity => "candidate identity mismatch",
            Self::CandidateText => "invalid or oversized candidate text",
            Self::CandidateSource => "candidate source mismatch",
            Self::CandidateCount => "candidate count exceeds limit",
            Self::CandidateBatch => "candidate batch exceeds limit",
            Self::StaleAcceptance => "candidate acceptance is stale",
            Self::RouteLimit => "active suggestion route limit reached",
        })
    }
}

impl std::error::Error for ValidationError {}

fn validate_span(buffer: &str, span: ReplacementSpan) -> Result<(), ValidationError> {
    if span.start > span.end
        || span.end > buffer.len()
        || !buffer.is_char_boundary(span.start)
        || !buffer.is_char_boundary(span.end)
        || !is_grapheme_boundary(buffer, span.start)
        || !is_grapheme_boundary(buffer, span.end)
    {
        return Err(ValidationError::ReplacementSpan);
    }
    Ok(())
}

fn is_grapheme_boundary(value: &str, offset: usize) -> bool {
    offset == 0
        || offset == value.len()
        || value
            .grapheme_indices(true)
            .any(|(candidate, _)| candidate == offset)
}

fn has_bidi_control(value: &str) -> bool {
    value.chars().any(|character| {
        matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
    })
}

fn bounded_plain(value: &str, maximum: usize, allow_empty: bool) -> bool {
    value.len() <= maximum
        && (allow_empty || !value.trim().is_empty())
        && !value.chars().any(char::is_control)
        && !has_bidi_control(value)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    FrameTooShort,
    FrameTooLarge,
    LengthMismatch,
    InvalidUtf8,
    InvalidPayload,
    InvalidRequest(ValidationError),
    InvalidSubmission(ValidationError),
    InvalidReplacement(ValidationError),
}

impl fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::FrameTooShort => "suggestion frame is truncated",
            Self::FrameTooLarge => "suggestion frame exceeds the byte limit",
            Self::LengthMismatch => "suggestion frame length does not match its prefix",
            Self::InvalidUtf8 => "suggestion frame is not strict UTF-8",
            Self::InvalidPayload => "suggestion frame JSON is invalid",
            Self::InvalidRequest(_) => "suggestion request is invalid",
            Self::InvalidSubmission(_) => "suggestion submission is invalid",
            Self::InvalidReplacement(_) => "suggestion replacement is invalid",
        })
    }
}

impl std::error::Error for FrameError {}

pub fn encode_request_frame(request: &EditorRequest) -> Result<Vec<u8>, FrameError> {
    request.validate().map_err(FrameError::InvalidRequest)?;
    let payload = serde_json::to_vec(request).map_err(|_| FrameError::InvalidPayload)?;
    if payload.len() > SuggestionLimits::FRAME_BYTES {
        return Err(FrameError::FrameTooLarge);
    }
    let length = u32::try_from(payload.len()).map_err(|_| FrameError::FrameTooLarge)?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_request_frame(frame: &[u8]) -> Result<EditorRequest, FrameError> {
    let prefix: [u8; 4] = frame
        .get(..4)
        .ok_or(FrameError::FrameTooShort)?
        .try_into()
        .map_err(|_| FrameError::FrameTooShort)?;
    let declared = u32::from_le_bytes(prefix) as usize;
    if declared > SuggestionLimits::FRAME_BYTES {
        return Err(FrameError::FrameTooLarge);
    }
    let payload = frame.get(4..).ok_or(FrameError::FrameTooShort)?;
    if payload.len() != declared {
        return Err(FrameError::LengthMismatch);
    }
    let text = std::str::from_utf8(payload).map_err(|_| FrameError::InvalidUtf8)?;
    let request: EditorRequest =
        serde_json::from_str(text).map_err(|_| FrameError::InvalidPayload)?;
    request.validate().map_err(FrameError::InvalidRequest)?;
    Ok(request)
}

pub fn encode_submission_frame(
    submission: &EditorSubmission,
) -> Result<Vec<u8>, FrameError> {
    submission
        .validate()
        .map_err(FrameError::InvalidSubmission)?;
    encode_json_frame(submission)
}

pub fn decode_submission_frame(frame: &[u8]) -> Result<EditorSubmission, FrameError> {
    let text = decode_json_frame(frame)?;
    let submission: EditorSubmission =
        serde_json::from_str(text).map_err(|_| FrameError::InvalidPayload)?;
    submission
        .validate()
        .map_err(FrameError::InvalidSubmission)?;
    Ok(submission)
}

pub fn encode_replacement_frame(
    replacement: &NativeEditorReplacement,
) -> Result<Vec<u8>, FrameError> {
    replacement
        .validate()
        .map_err(FrameError::InvalidReplacement)?;
    encode_json_frame(replacement)
}

pub fn decode_replacement_frame(
    frame: &[u8],
) -> Result<NativeEditorReplacement, FrameError> {
    let text = decode_json_frame(frame)?;
    let replacement: NativeEditorReplacement =
        serde_json::from_str(text).map_err(|_| FrameError::InvalidPayload)?;
    replacement
        .validate()
        .map_err(FrameError::InvalidReplacement)?;
    Ok(replacement)
}

fn encode_json_frame(value: &impl Serialize) -> Result<Vec<u8>, FrameError> {
    let payload = serde_json::to_vec(value).map_err(|_| FrameError::InvalidPayload)?;
    if payload.len() > SuggestionLimits::FRAME_BYTES {
        return Err(FrameError::FrameTooLarge);
    }
    let length = u32::try_from(payload.len()).map_err(|_| FrameError::FrameTooLarge)?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

fn decode_json_frame(frame: &[u8]) -> Result<&str, FrameError> {
    let prefix: [u8; 4] = frame
        .get(..4)
        .ok_or(FrameError::FrameTooShort)?
        .try_into()
        .map_err(|_| FrameError::FrameTooShort)?;
    let declared = u32::from_le_bytes(prefix) as usize;
    if declared > SuggestionLimits::FRAME_BYTES {
        return Err(FrameError::FrameTooLarge);
    }
    let payload = frame.get(4..).ok_or(FrameError::FrameTooShort)?;
    if payload.len() != declared {
        return Err(FrameError::LengthMismatch);
    }
    std::str::from_utf8(payload).map_err(|_| FrameError::InvalidUtf8)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CandidateSource {
    NativeShell,
    ShellHistory,
    ShellCwd,
    AcceptedFrequency,
    CachedPublic,
    TypedAction,
}

impl CandidateSource {
    pub const ALL: [Self; 6] = [
        Self::NativeShell,
        Self::ShellHistory,
        Self::ShellCwd,
        Self::AcceptedFrequency,
        Self::CachedPublic,
        Self::TypedAction,
    ];

    pub const fn priority(self) -> u8 {
        match self {
            Self::NativeShell => 0,
            Self::ShellHistory => 1,
            Self::ShellCwd => 2,
            Self::AcceptedFrequency => 3,
            Self::CachedPublic => 4,
            Self::TypedAction => 5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CandidateKind {
    Command,
    Option,
    Argument,
    Path,
    History,
    Context,
    Action,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CandidateFreshness {
    Current,
    Cached,
    Stale,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CandidateRisk {
    ReadOnly,
    Mutating,
    Destructive,
    Unknown,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub request_id: u64,
    pub candidate_id: u64,
    pub insertion: String,
    pub display: String,
    pub description: String,
    pub kind: CandidateKind,
    pub source: CandidateSource,
    pub freshness: CandidateFreshness,
    pub replacement_span: ReplacementSpan,
    pub quoting: QuoteContext,
    pub public_context: bool,
    pub risk: CandidateRisk,
}

impl Candidate {
    pub fn validate(&self, request: &EditorRequest) -> Result<(), ValidationError> {
        if self.request_id != request.request_id || self.candidate_id == 0 {
            return Err(ValidationError::CandidateIdentity);
        }
        if self.replacement_span != request.replacement_span {
            return Err(ValidationError::ReplacementSpan);
        }
        if self.insertion.len() > SuggestionLimits::CANDIDATE_BYTES
            || self.insertion.is_empty()
            || self.insertion.contains(['\0', '\r', '\n'])
            || has_bidi_control(&self.insertion)
            || !bounded_plain(&self.display, SuggestionLimits::CANDIDATE_BYTES, false)
            || !bounded_plain(
                &self.description,
                SuggestionLimits::DESCRIPTION_BYTES,
                true,
            )
        {
            return Err(ValidationError::CandidateText);
        }
        Ok(())
    }

    pub fn revalidate_for_acceptance(
        &self,
        request: &EditorRequest,
        current: &AcceptanceContext,
    ) -> Result<NativeInsertion, ValidationError> {
        self.validate(request)?;
        if current.request_id != request.request_id
            || current.route != request.route()
            || current.prompt_generation != request.prompt_generation
            || current.buffer_generation != request.buffer_generation
            || current.cancellation_id != request.cancellation_id
            || current.replacement_span != request.replacement_span
        {
            return Err(ValidationError::StaleAcceptance);
        }
        Ok(NativeInsertion {
            span: self.replacement_span,
            bytes: self.insertion.as_bytes().to_vec(),
        })
    }
}

impl fmt::Debug for Candidate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Candidate")
            .field("request_id", &self.request_id)
            .field("candidate_id", &self.candidate_id)
            .field("insertion_bytes", &self.insertion.len())
            .field("display_bytes", &self.display.len())
            .field("description_bytes", &self.description.len())
            .field("kind", &self.kind)
            .field("source", &self.source)
            .field("freshness", &self.freshness)
            .field("risk", &self.risk)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeInsertion {
    pub span: ReplacementSpan,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceContext {
    pub request_id: u64,
    pub route: RouteIdentity,
    pub prompt_generation: u64,
    pub buffer_generation: u64,
    pub cancellation_id: u64,
    pub replacement_span: ReplacementSpan,
}

impl AcceptanceContext {
    pub fn from_request(request: &EditorRequest) -> Self {
        Self {
            request_id: request.request_id,
            route: request.route(),
            prompt_generation: request.prompt_generation,
            buffer_generation: request.buffer_generation,
            cancellation_id: request.cancellation_id,
            replacement_span: request.replacement_span,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEditorReplacement {
    pub schema: u16,
    pub request_id: u64,
    pub route: RouteIdentity,
    pub capability: SuggestionCapability,
    pub prompt_generation: u64,
    pub buffer_generation: u64,
    pub cancellation_id: u64,
    pub candidate_id: u64,
    pub replacement_span: ReplacementSpan,
    pub insertion: String,
    pub execute: bool,
}

impl NativeEditorReplacement {
    pub fn from_candidate(
        request: &EditorRequest,
        candidate: &Candidate,
        current: &AcceptanceContext,
    ) -> Result<Self, ValidationError> {
        let insertion = candidate.revalidate_for_acceptance(request, current)?;
        let value = Self {
            schema: PROTOCOL_SCHEMA,
            request_id: request.request_id,
            route: request.route(),
            capability: request.capability,
            prompt_generation: request.prompt_generation,
            buffer_generation: request.buffer_generation,
            cancellation_id: request.cancellation_id,
            candidate_id: candidate.candidate_id,
            replacement_span: insertion.span,
            insertion: String::from_utf8(insertion.bytes)
                .map_err(|_| ValidationError::CandidateText)?,
            execute: false,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.schema != PROTOCOL_SCHEMA {
            return Err(ValidationError::Schema);
        }
        if [
            self.request_id,
            self.route.application_generation,
            self.route.window_id,
            self.route.tab_id,
            self.route.pane_id,
            self.route.session_id,
            self.route.endpoint_instance,
            self.prompt_generation,
            self.buffer_generation,
            self.cancellation_id,
            self.candidate_id,
        ]
        .contains(&0)
        {
            return Err(ValidationError::Identity);
        }
        if !self.capability.is_valid() {
            return Err(ValidationError::Capability);
        }
        if !bounded_plain(
            &self.route.editor_version,
            SuggestionLimits::EDITOR_VERSION_BYTES,
            false,
        ) {
            return Err(ValidationError::EditorVersion);
        }
        if self.insertion.is_empty()
            || self.insertion.len() > SuggestionLimits::CANDIDATE_BYTES
            || self.insertion.contains(['\0', '\r', '\n'])
            || has_bidi_control(&self.insertion)
            || self.replacement_span.start > self.replacement_span.end
            || self.execute
        {
            return Err(ValidationError::CandidateText);
        }
        Ok(())
    }

    pub fn revalidate(
        &self,
        current: &AcceptanceContext,
        capability: &SuggestionCapability,
    ) -> Result<NativeInsertion, ValidationError> {
        self.validate()?;
        if !self.capability.constant_time_eq(capability) {
            return Err(ValidationError::Capability);
        }
        if self.request_id != current.request_id
            || self.route != current.route
            || self.prompt_generation != current.prompt_generation
            || self.buffer_generation != current.buffer_generation
            || self.cancellation_id != current.cancellation_id
            || self.replacement_span != current.replacement_span
        {
            return Err(ValidationError::StaleAcceptance);
        }
        Ok(NativeInsertion {
            span: self.replacement_span,
            bytes: self.insertion.as_bytes().to_vec(),
        })
    }
}

impl fmt::Debug for NativeEditorReplacement {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeEditorReplacement")
            .field("schema", &self.schema)
            .field("request_id", &self.request_id)
            .field("route", &self.route)
            .field("capability", &"[REDACTED]")
            .field("candidate_id", &self.candidate_id)
            .field("replacement_span", &self.replacement_span)
            .field("insertion_bytes", &self.insertion.len())
            .field("execute", &self.execute)
            .finish()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RankInput {
    pub candidate: Candidate,
    pub native_rank: Option<u16>,
    pub frequency: u32,
}

impl RankInput {
    pub const fn new(
        candidate: Candidate,
        native_rank: Option<u16>,
        frequency: u32,
    ) -> Self {
        Self {
            candidate,
            native_rank,
            frequency,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceBatch {
    pub source: CandidateSource,
    pub source_revision: u64,
    pub freshness: CandidateFreshness,
    pub candidates: Vec<RankInput>,
}

impl SourceBatch {
    pub fn new(
        source: CandidateSource,
        source_revision: u64,
        freshness: CandidateFreshness,
        candidates: Vec<RankInput>,
    ) -> Result<Self, ValidationError> {
        if source_revision == 0 {
            return Err(ValidationError::Identity);
        }
        if candidates.len() > SuggestionLimits::CANDIDATE_COUNT {
            return Err(ValidationError::CandidateCount);
        }
        if candidates
            .iter()
            .any(|candidate| candidate.candidate.source != source)
        {
            return Err(ValidationError::CandidateSource);
        }
        let bytes = candidates.iter().fold(0_usize, |total, candidate| {
            total
                .saturating_add(candidate.candidate.insertion.len())
                .saturating_add(candidate.candidate.display.len())
                .saturating_add(candidate.candidate.description.len())
        });
        if bytes > SuggestionLimits::BATCH_BYTES {
            return Err(ValidationError::CandidateBatch);
        }
        Ok(Self {
            source,
            source_revision,
            freshness,
            candidates,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceBindings {
    pub tab: bool,
    pub right_arrow: bool,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorSubmission {
    pub schema: u16,
    pub request: EditorRequest,
    pub batches: Vec<SourceBatch>,
    pub acceptance: AcceptanceBindings,
}

impl EditorSubmission {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.schema != PROTOCOL_SCHEMA || self.request.schema != self.schema {
            return Err(ValidationError::Schema);
        }
        self.request.validate()?;
        if self.batches.len() > 3 {
            return Err(ValidationError::CandidateSource);
        }
        let mut sources = BTreeSet::new();
        let mut candidate_count = 0_usize;
        let mut candidate_bytes = 0_usize;
        for batch in &self.batches {
            if !matches!(
                batch.source,
                CandidateSource::NativeShell
                    | CandidateSource::ShellHistory
                    | CandidateSource::ShellCwd
            ) || !sources.insert(batch.source)
                || !source_batch_valid(&self.request, batch)
            {
                return Err(ValidationError::CandidateSource);
            }
            for candidate in &batch.candidates {
                if candidate.frequency != 0
                    || (batch.source != CandidateSource::NativeShell
                        && candidate.native_rank.is_some())
                    || candidate.candidate.public_context
                {
                    return Err(ValidationError::CandidateSource);
                }
                candidate_count = candidate_count.saturating_add(1);
                candidate_bytes = candidate_bytes
                    .saturating_add(candidate.candidate.insertion.len())
                    .saturating_add(candidate.candidate.display.len())
                    .saturating_add(candidate.candidate.description.len());
            }
        }
        if candidate_count > SuggestionLimits::CANDIDATE_COUNT {
            return Err(ValidationError::CandidateCount);
        }
        if candidate_bytes > SuggestionLimits::BATCH_BYTES {
            return Err(ValidationError::CandidateBatch);
        }
        Ok(())
    }
}

impl fmt::Debug for EditorSubmission {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EditorSubmission")
            .field("schema", &self.schema)
            .field("request", &self.request)
            .field("batch_count", &self.batches.len())
            .field(
                "candidate_count",
                &self
                    .batches
                    .iter()
                    .map(|batch| batch.candidates.len())
                    .sum::<usize>(),
            )
            .field("acceptance", &self.acceptance)
            .finish()
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourcePolicy {
    pub shell_history: bool,
    pub frequency: bool,
}

impl SourcePolicy {
    const fn permits(self, source: CandidateSource) -> bool {
        match source {
            CandidateSource::ShellHistory => self.shell_history,
            CandidateSource::AcceptedFrequency => self.frequency,
            _ => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFailure {
    Disabled,
    Deadline,
    Unavailable,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceResult {
    Ready {
        batch: SourceBatch,
        elapsed_ms: u64,
    },
    Failed {
        source: CandidateSource,
        failure: SourceFailure,
    },
}

impl SourceResult {
    pub const fn source(&self) -> CandidateSource {
        match self {
            Self::Ready { batch, .. } => batch.source,
            Self::Failed { source, .. } => *source,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceStatus {
    pub source: CandidateSource,
    pub freshness: CandidateFreshness,
    pub candidate_count: usize,
    pub failure: Option<SourceFailure>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceProjection {
    pub batches: Vec<SourceBatch>,
    pub statuses: Vec<SourceStatus>,
}

/// Capability-free aggregation of editor-owned and already-cached sources.
///
/// Callers supply typed, bounded results. This owner has no filesystem,
/// network, process, provider, credential, clipboard, PTY, output, history-file,
/// extension, or renderer access.
#[derive(Clone, Debug, Default)]
pub struct LocalSourceBroker {
    last_known_public: Option<SourceBatch>,
}

impl LocalSourceBroker {
    pub fn collect(
        &mut self,
        request: &EditorRequest,
        results: Vec<SourceResult>,
        policy: SourcePolicy,
    ) -> Result<SourceProjection, ValidationError> {
        request.validate()?;
        let mut unique = BTreeMap::new();
        for result in results {
            if unique.insert(result.source(), result).is_some() {
                return Err(ValidationError::CandidateSource);
            }
        }

        let mut batches = Vec::new();
        let mut statuses = Vec::with_capacity(CandidateSource::ALL.len());
        for source in CandidateSource::ALL {
            if !policy.permits(source) {
                statuses.push(SourceStatus {
                    source,
                    freshness: CandidateFreshness::Unavailable,
                    candidate_count: 0,
                    failure: Some(SourceFailure::Disabled),
                });
                continue;
            }

            match unique.remove(&source) {
                Some(SourceResult::Ready {
                    batch: _,
                    elapsed_ms,
                }) if elapsed_ms > SuggestionLimits::SOURCE_DEADLINE_MS => {
                    self.fallback_or_status(
                        source,
                        SourceFailure::Deadline,
                        &mut batches,
                        &mut statuses,
                    );
                }
                Some(SourceResult::Ready { batch, .. })
                    if !source_batch_valid(request, &batch) =>
                {
                    self.fallback_or_status(
                        source,
                        SourceFailure::Rejected,
                        &mut batches,
                        &mut statuses,
                    );
                }
                Some(SourceResult::Ready { batch, .. }) => {
                    if source == CandidateSource::CachedPublic {
                        self.last_known_public = Some(batch.clone());
                    }
                    statuses.push(SourceStatus {
                        source,
                        freshness: batch.freshness,
                        candidate_count: batch.candidates.len(),
                        failure: None,
                    });
                    batches.push(batch);
                }
                Some(SourceResult::Failed { failure, .. }) => {
                    self.fallback_or_status(source, failure, &mut batches, &mut statuses);
                }
                None => self.fallback_or_status(
                    source,
                    SourceFailure::Unavailable,
                    &mut batches,
                    &mut statuses,
                ),
            }
        }
        Ok(SourceProjection { batches, statuses })
    }

    pub fn reset(&mut self) {
        self.last_known_public = None;
    }

    fn fallback_or_status(
        &self,
        source: CandidateSource,
        failure: SourceFailure,
        batches: &mut Vec<SourceBatch>,
        statuses: &mut Vec<SourceStatus>,
    ) {
        let fallback = (source == CandidateSource::CachedPublic)
            .then(|| self.last_known_public.clone())
            .flatten()
            .map(mark_batch_stale);
        if let Some(batch) = fallback {
            statuses.push(SourceStatus {
                source,
                freshness: CandidateFreshness::Stale,
                candidate_count: batch.candidates.len(),
                failure: Some(failure),
            });
            batches.push(batch);
        } else {
            statuses.push(SourceStatus {
                source,
                freshness: CandidateFreshness::Unavailable,
                candidate_count: 0,
                failure: Some(failure),
            });
        }
    }
}

fn source_batch_valid(request: &EditorRequest, batch: &SourceBatch) -> bool {
    batch.source_revision <= request.source_revision
        && batch.candidates.iter().all(|ranked| {
            ranked.candidate.validate(request).is_ok()
                && ranked.candidate.source == batch.source
                && (batch.source != CandidateSource::CachedPublic
                    || ranked.candidate.public_context)
        })
}

fn mark_batch_stale(mut batch: SourceBatch) -> SourceBatch {
    batch.freshness = CandidateFreshness::Stale;
    for ranked in &mut batch.candidates {
        ranked.candidate.freshness = CandidateFreshness::Stale;
    }
    batch
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FrequencyEntry {
    count: u32,
    epoch: u64,
}

/// Memory-only, bounded, decayed acceptance counters keyed only by candidate ID.
#[derive(Clone, Debug, Default)]
pub struct FrequencyMemory {
    entries: BTreeMap<u64, FrequencyEntry>,
}

impl FrequencyMemory {
    pub fn record_accept(&mut self, candidate_id: u64, epoch: u64) -> bool {
        if candidate_id == 0 || epoch == 0 {
            return false;
        }
        if let Some(entry) = self.entries.get_mut(&candidate_id) {
            entry.count = decayed_count(*entry, epoch).saturating_add(1);
            entry.epoch = epoch.max(entry.epoch);
            return true;
        }
        if self.entries.len() >= SuggestionLimits::FREQUENCY_IDS {
            let evict = self
                .entries
                .iter()
                .min_by_key(|(id, entry)| (entry.count, entry.epoch, **id))
                .map(|(id, _)| *id);
            if let Some(evict) = evict {
                self.entries.remove(&evict);
            }
        }
        self.entries
            .insert(candidate_id, FrequencyEntry { count: 1, epoch });
        true
    }

    pub fn score(&self, candidate_id: u64, epoch: u64) -> u32 {
        self.entries
            .get(&candidate_id)
            .map_or(0, |entry| decayed_count(*entry, epoch))
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn decayed_count(entry: FrequencyEntry, epoch: u64) -> u32 {
    let shifts = epoch.saturating_sub(entry.epoch) / 32;
    entry.count >> u32::try_from(shifts.min(31)).unwrap_or(31)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankedCandidate {
    pub candidate: Candidate,
    pub exact_prefix: bool,
    pub native_rank: u16,
    pub word_boundary_prefix: bool,
    pub case_insensitive_prefix: bool,
    pub frequency: u32,
    pub fuzzy_score: u16,
    pub matched_graphemes: Vec<usize>,
}

pub fn rank_batches(
    request: &EditorRequest,
    batches: &[SourceBatch],
    policy: SourcePolicy,
) -> Result<Vec<RankedCandidate>, ValidationError> {
    request.validate()?;
    let mut total_candidates = 0_usize;
    let mut total_bytes = 0_usize;
    let mut ranked = Vec::new();
    let mut seen = BTreeSet::new();
    let query = request.query();
    let query_folded = query.to_lowercase();

    for batch in batches {
        if !policy.permits(batch.source) {
            continue;
        }
        total_candidates = total_candidates.saturating_add(batch.candidates.len());
        if total_candidates > SuggestionLimits::CANDIDATE_COUNT {
            return Err(ValidationError::CandidateCount);
        }
        for input in &batch.candidates {
            input.candidate.validate(request)?;
            if input.candidate.source != batch.source {
                return Err(ValidationError::CandidateSource);
            }
            total_bytes = total_bytes
                .saturating_add(input.candidate.insertion.len())
                .saturating_add(input.candidate.display.len())
                .saturating_add(input.candidate.description.len());
            if total_bytes > SuggestionLimits::BATCH_BYTES {
                return Err(ValidationError::CandidateBatch);
            }

            let dedupe_key = input.candidate.insertion.clone();
            if !seen.insert(dedupe_key) {
                continue;
            }
            let insertion_folded = input.candidate.insertion.to_lowercase();
            let display_folded = input.candidate.display.to_lowercase();
            let (fuzzy_score, _) = fuzzy_match(&query_folded, &insertion_folded);
            let (_, matched_graphemes) = fuzzy_match(&query_folded, &display_folded);
            ranked.push(RankedCandidate {
                candidate: input.candidate.clone(),
                exact_prefix: input.candidate.insertion.starts_with(query),
                native_rank: input.native_rank.unwrap_or(u16::MAX),
                word_boundary_prefix: word_boundary_prefix(
                    &input.candidate.insertion,
                    query,
                ),
                case_insensitive_prefix: insertion_folded.starts_with(&query_folded),
                frequency: if policy.frequency { input.frequency } else { 0 },
                fuzzy_score,
                matched_graphemes,
            });
        }
    }

    ranked.sort_by(compare_ranked);
    Ok(ranked)
}

fn compare_ranked(left: &RankedCandidate, right: &RankedCandidate) -> Ordering {
    right
        .exact_prefix
        .cmp(&left.exact_prefix)
        .then_with(|| left.native_rank.cmp(&right.native_rank))
        .then_with(|| right.word_boundary_prefix.cmp(&left.word_boundary_prefix))
        .then_with(|| {
            right
                .case_insensitive_prefix
                .cmp(&left.case_insensitive_prefix)
        })
        .then_with(|| right.frequency.cmp(&left.frequency))
        .then_with(|| right.fuzzy_score.cmp(&left.fuzzy_score))
        .then_with(|| {
            left.candidate
                .source
                .priority()
                .cmp(&right.candidate.source.priority())
        })
        .then_with(|| {
            left.candidate
                .display
                .to_lowercase()
                .cmp(&right.candidate.display.to_lowercase())
        })
        .then_with(|| {
            left.candidate
                .candidate_id
                .cmp(&right.candidate.candidate_id)
        })
}

fn word_boundary_prefix(candidate: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    candidate
        .char_indices()
        .filter(|(index, _)| {
            *index == 0
                || candidate[..*index]
                    .chars()
                    .next_back()
                    .is_some_and(|character| !character.is_alphanumeric())
        })
        .any(|(index, _)| candidate[index..].starts_with(query))
}

fn fuzzy_match(query: &str, candidate: &str) -> (u16, Vec<usize>) {
    if query.is_empty() {
        return (0, Vec::new());
    }
    let mut matched = Vec::new();
    let mut candidate_graphemes = candidate.graphemes(true).enumerate();
    let mut previous = None;
    let mut adjacency = 0_u16;
    for needle in query.graphemes(true) {
        let Some((index, _)) =
            candidate_graphemes.find(|(_, grapheme)| *grapheme == needle)
        else {
            return (0, Vec::new());
        };
        if previous.is_some_and(|value| value + 1 == index) {
            adjacency = adjacency.saturating_add(1);
        }
        previous = Some(index);
        matched.push(index);
    }
    let count = u16::try_from(matched.len()).unwrap_or(u16::MAX);
    (count.saturating_mul(4).saturating_add(adjacency), matched)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotDisposition {
    Queued,
    Replaced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Slot {
    request_id: u64,
    cancellation_id: u64,
}

#[derive(Clone, Debug, Default)]
pub struct LatestRequestSlots {
    slots: BTreeMap<RouteIdentity, Slot>,
}

impl LatestRequestSlots {
    pub const fn new() -> Self {
        Self {
            slots: BTreeMap::new(),
        }
    }

    pub fn submit(
        &mut self,
        request: &EditorRequest,
    ) -> Result<SlotDisposition, ValidationError> {
        request.validate()?;
        let route = request.route();
        let replaced = self.slots.contains_key(&route);
        if !replaced && self.slots.len() >= SuggestionLimits::ACTIVE_ROUTES {
            return Err(ValidationError::RouteLimit);
        }
        self.slots.insert(
            route,
            Slot {
                request_id: request.request_id,
                cancellation_id: request.cancellation_id,
            },
        );
        Ok(if replaced {
            SlotDisposition::Replaced
        } else {
            SlotDisposition::Queued
        })
    }

    pub fn accepts(
        &self,
        route: &RouteIdentity,
        request_id: u64,
        cancellation_id: u64,
    ) -> bool {
        self.slots.get(route).is_some_and(|slot| {
            slot.request_id == request_id && slot.cancellation_id == cancellation_id
        })
    }

    pub fn close_route(&mut self, route: &RouteIdentity) -> bool {
        self.slots.remove(route).is_some()
    }

    pub fn clear(&mut self) {
        self.slots.clear();
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_debug_never_contains_secret_bytes() {
        let value = SuggestionCapability::from_bytes([0xab; 32]);
        assert_eq!(format!("{value:?}"), "SuggestionCapability([REDACTED; 32])");
        assert!(value.constant_time_eq(&value));
        assert!(!value.constant_time_eq(&SuggestionCapability::from_bytes([
            0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab,
            0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab,
            0xab, 0xab, 0xab, 0xab, 0xab, 0xaa,
        ])));
    }

    #[test]
    fn fuzzy_score_requires_an_ordered_subsequence() {
        assert!(fuzzy_match("kget", "kubectl-get").0 > 0);
        assert_eq!(fuzzy_match("zget", "kubectl-get").0, 0);
    }
}
