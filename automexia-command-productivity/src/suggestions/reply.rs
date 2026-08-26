//! Authenticated application-to-editor replies for the CP5 local bridge.
//!
//! A status reply carries the same route, generation, cancellation, and
//! capability binding as a replacement. This prevents an unauthenticated empty
//! response from being mistaken for a valid result while keeping all private
//! editor text out of diagnostics.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::{
    bounded_plain, decode_json_frame, encode_json_frame, AcceptanceContext,
    EditorRequest, FrameError, NativeEditorReplacement, RouteIdentity,
    SuggestionCapability, SuggestionLimits, ValidationError, PROTOCOL_SCHEMA,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NativeEditorStatusCode {
    NoCandidates,
    Dismissed,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEditorStatus {
    pub schema: u16,
    pub request_id: u64,
    pub route: RouteIdentity,
    pub capability: SuggestionCapability,
    pub prompt_generation: u64,
    pub buffer_generation: u64,
    pub cancellation_id: u64,
    pub code: NativeEditorStatusCode,
}

impl NativeEditorStatus {
    pub fn from_request(request: &EditorRequest, code: NativeEditorStatusCode) -> Self {
        Self {
            schema: PROTOCOL_SCHEMA,
            request_id: request.request_id,
            route: request.route(),
            capability: request.capability,
            prompt_generation: request.prompt_generation,
            buffer_generation: request.buffer_generation,
            cancellation_id: request.cancellation_id,
            code,
        }
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
        Ok(())
    }

    pub fn revalidate(
        &self,
        current: &AcceptanceContext,
        capability: &SuggestionCapability,
    ) -> Result<NativeEditorStatusCode, ValidationError> {
        self.validate()?;
        if !self.capability.constant_time_eq(capability) {
            return Err(ValidationError::Capability);
        }
        if self.request_id != current.request_id
            || self.route != current.route
            || self.prompt_generation != current.prompt_generation
            || self.buffer_generation != current.buffer_generation
            || self.cancellation_id != current.cancellation_id
        {
            return Err(ValidationError::StaleAcceptance);
        }
        Ok(self.code)
    }
}

impl fmt::Debug for NativeEditorStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeEditorStatus")
            .field("schema", &self.schema)
            .field("request_id", &self.request_id)
            .field("route", &self.route)
            .field("capability", &"[REDACTED]")
            .field("prompt_generation", &self.prompt_generation)
            .field("buffer_generation", &self.buffer_generation)
            .field("cancellation_id", &self.cancellation_id)
            .field("code", &self.code)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "payload")]
pub enum NativeEditorReply {
    Replacement(NativeEditorReplacement),
    Status(NativeEditorStatus),
}

impl NativeEditorReply {
    pub fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::Replacement(replacement) => replacement.validate(),
            Self::Status(status) => status.validate(),
        }
    }
}

pub fn encode_reply_frame(reply: &NativeEditorReply) -> Result<Vec<u8>, FrameError> {
    reply.validate().map_err(FrameError::InvalidReplacement)?;
    encode_json_frame(reply)
}

pub fn decode_reply_frame(frame: &[u8]) -> Result<NativeEditorReply, FrameError> {
    let text = decode_json_frame(frame)?;
    let reply: NativeEditorReply =
        serde_json::from_str(text).map_err(|_| FrameError::InvalidPayload)?;
    reply.validate().map_err(FrameError::InvalidReplacement)?;
    Ok(reply)
}
