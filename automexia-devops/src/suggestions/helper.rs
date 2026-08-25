//! Bounded, capability-free records exchanged with one session helper.
//!
//! The shell-facing protocol carries no route identity, endpoint name,
//! application capability, or execution bit. The signed helper supplies
//! authenticated route state before translating records to schema-1 JSON.

use std::fmt;

use super::{
    has_bidi_control, is_grapheme_boundary, validate_span, QuoteContext, ReplacementSpan,
    SuggestionLimits,
};

pub const HELPER_MAGIC: [u8; 4] = *b"AXSH";
pub const HELPER_VERSION: u16 = 1;
pub const HELPER_HEADER_BYTES: usize = 11;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum RecordKind {
    Request = 1,
    Accept = 2,
    Dismiss = 3,
    Replace = 4,
    Status = 5,
}

impl TryFrom<u8> for RecordKind {
    type Error = HelperRecordError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Request),
            2 => Ok(Self::Accept),
            3 => Ok(Self::Dismiss),
            4 => Ok(Self::Replace),
            5 => Ok(Self::Status),
            _ => Err(HelperRecordError::UnknownKind),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum HelperDismissReason {
    Escape = 1,
    Enter = 2,
    Typing = 3,
    FocusChanged = 4,
    RouteClosed = 5,
    TimedOut = 6,
}

impl TryFrom<u8> for HelperDismissReason {
    type Error = HelperRecordError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Escape),
            2 => Ok(Self::Enter),
            3 => Ok(Self::Typing),
            4 => Ok(Self::FocusChanged),
            5 => Ok(Self::RouteClosed),
            6 => Ok(Self::TimedOut),
            _ => Err(HelperRecordError::InvalidPayload),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum HelperStatusCode {
    Ready = 1,
    NoCandidates = 2,
    Stale = 3,
    Unavailable = 4,
    Killed = 5,
    Invalid = 6,
}

impl TryFrom<u8> for HelperStatusCode {
    type Error = HelperRecordError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Ready),
            2 => Ok(Self::NoCandidates),
            3 => Ok(Self::Stale),
            4 => Ok(Self::Unavailable),
            5 => Ok(Self::Killed),
            6 => Ok(Self::Invalid),
            _ => Err(HelperRecordError::InvalidPayload),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct HelperRequest {
    pub buffer: String,
    pub cursor_byte: usize,
    pub adapter_generation: u64,
    pub replacement_span: ReplacementSpan,
    pub selection: Option<ReplacementSpan>,
    pub quote_context: QuoteContext,
    pub native_candidates: Vec<String>,
}

impl HelperRequest {
    pub fn validate(&self) -> Result<(), HelperRecordError> {
        if self.adapter_generation == 0
            || self.buffer.len() > SuggestionLimits::BUFFER_BYTES
            || self.buffer.contains('\0')
            || self.cursor_byte > self.buffer.len()
            || !self.buffer.is_char_boundary(self.cursor_byte)
            || !is_grapheme_boundary(&self.buffer, self.cursor_byte)
        {
            return Err(HelperRecordError::InvalidPayload);
        }
        validate_span(&self.buffer, self.replacement_span)
            .map_err(|_| HelperRecordError::InvalidPayload)?;
        if self.replacement_span.start > self.cursor_byte
            || self.replacement_span.end < self.cursor_byte
        {
            return Err(HelperRecordError::InvalidPayload);
        }
        if let Some(selection) = self.selection {
            validate_span(&self.buffer, selection)
                .map_err(|_| HelperRecordError::InvalidPayload)?;
        }
        if self.native_candidates.len() > SuggestionLimits::CANDIDATE_COUNT {
            return Err(HelperRecordError::InvalidPayload);
        }
        let mut total = self.buffer.len();
        for candidate in &self.native_candidates {
            if candidate.is_empty()
                || candidate.len() > SuggestionLimits::CANDIDATE_BYTES
                || candidate.chars().any(char::is_control)
                || has_bidi_control(candidate)
            {
                return Err(HelperRecordError::InvalidPayload);
            }
            total = total
                .checked_add(candidate.len())
                .ok_or(HelperRecordError::FrameTooLarge)?;
        }
        if total > SuggestionLimits::BATCH_BYTES {
            return Err(HelperRecordError::FrameTooLarge);
        }
        Ok(())
    }
}

impl fmt::Debug for HelperRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HelperRequest")
            .field("buffer_bytes", &self.buffer.len())
            .field("cursor_byte", &self.cursor_byte)
            .field("adapter_generation", &self.adapter_generation)
            .field("replacement_span", &self.replacement_span)
            .field("selection", &self.selection)
            .field("quote_context", &self.quote_context)
            .field("native_candidate_count", &self.native_candidates.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HelperAccept {
    pub adapter_generation: u64,
    pub request_id: u64,
    pub candidate_id: u64,
}

impl HelperAccept {
    fn validate(self) -> Result<(), HelperRecordError> {
        if self.adapter_generation == 0 || self.request_id == 0 || self.candidate_id == 0
        {
            Err(HelperRecordError::InvalidPayload)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HelperDismiss {
    pub adapter_generation: u64,
    pub reason: HelperDismissReason,
}

impl HelperDismiss {
    fn validate(self) -> Result<(), HelperRecordError> {
        if self.adapter_generation == 0 {
            Err(HelperRecordError::InvalidPayload)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct HelperReplace {
    pub adapter_generation: u64,
    pub request_id: u64,
    pub replacement_span: ReplacementSpan,
    pub insertion: String,
}

impl HelperReplace {
    pub fn validate(&self) -> Result<(), HelperRecordError> {
        if self.adapter_generation == 0
            || self.request_id == 0
            || self.replacement_span.start > self.replacement_span.end
            || self.insertion.is_empty()
            || self.insertion.len() > SuggestionLimits::CANDIDATE_BYTES
            || self.insertion.chars().any(char::is_control)
            || has_bidi_control(&self.insertion)
        {
            Err(HelperRecordError::InvalidPayload)
        } else {
            Ok(())
        }
    }
}

impl fmt::Debug for HelperReplace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HelperReplace")
            .field("adapter_generation", &self.adapter_generation)
            .field("request_id", &self.request_id)
            .field("replacement_span", &self.replacement_span)
            .field("insertion_bytes", &self.insertion.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HelperStatus {
    pub code: HelperStatusCode,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum HelperRecord {
    Request(HelperRequest),
    Accept(HelperAccept),
    Dismiss(HelperDismiss),
    Replace(HelperReplace),
    Status(HelperStatus),
}

impl HelperRecord {
    pub fn validate(&self) -> Result<(), HelperRecordError> {
        match self {
            Self::Request(request) => request.validate(),
            Self::Accept(accept) => accept.validate(),
            Self::Dismiss(dismiss) => dismiss.validate(),
            Self::Replace(replacement) => replacement.validate(),
            Self::Status(_) => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelperRecordError {
    FrameTooShort,
    FrameTooLarge,
    InvalidMagic,
    UnsupportedVersion,
    UnknownKind,
    LengthMismatch,
    InvalidUtf8,
    InvalidPayload,
}

impl fmt::Display for HelperRecordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::FrameTooShort => "suggestion helper record is truncated",
            Self::FrameTooLarge => "suggestion helper record exceeds its byte limit",
            Self::InvalidMagic => "suggestion helper record magic is invalid",
            Self::UnsupportedVersion => "suggestion helper record version is unsupported",
            Self::UnknownKind => "suggestion helper record kind is unknown",
            Self::LengthMismatch => "suggestion helper record length does not match",
            Self::InvalidUtf8 => "suggestion helper record text is not strict UTF-8",
            Self::InvalidPayload => "suggestion helper record payload is invalid",
        })
    }
}

impl std::error::Error for HelperRecordError {}

pub fn encode_record(record: &HelperRecord) -> Result<Vec<u8>, HelperRecordError> {
    record.validate()?;
    let (kind, payload) = match record {
        HelperRecord::Request(request) => {
            let mut payload = Vec::new();
            put_string(&mut payload, &request.buffer)?;
            put_usize(&mut payload, request.cursor_byte)?;
            put_u64(&mut payload, request.adapter_generation);
            put_span(&mut payload, request.replacement_span)?;
            match request.selection {
                Some(selection) => {
                    payload.push(1);
                    put_span(&mut payload, selection)?;
                }
                None => payload.push(0),
            }
            payload.push(encode_quote(request.quote_context));
            put_usize(&mut payload, request.native_candidates.len())?;
            for candidate in &request.native_candidates {
                put_string(&mut payload, candidate)?;
            }
            (RecordKind::Request, payload)
        }
        HelperRecord::Accept(accept) => {
            let mut payload = Vec::with_capacity(24);
            put_u64(&mut payload, accept.adapter_generation);
            put_u64(&mut payload, accept.request_id);
            put_u64(&mut payload, accept.candidate_id);
            (RecordKind::Accept, payload)
        }
        HelperRecord::Dismiss(dismiss) => {
            let mut payload = Vec::with_capacity(9);
            put_u64(&mut payload, dismiss.adapter_generation);
            payload.push(dismiss.reason as u8);
            (RecordKind::Dismiss, payload)
        }
        HelperRecord::Replace(replacement) => {
            let mut payload = Vec::new();
            put_u64(&mut payload, replacement.adapter_generation);
            put_u64(&mut payload, replacement.request_id);
            put_span(&mut payload, replacement.replacement_span)?;
            put_string(&mut payload, &replacement.insertion)?;
            (RecordKind::Replace, payload)
        }
        HelperRecord::Status(status) => (RecordKind::Status, vec![status.code as u8]),
    };
    if payload.len() > SuggestionLimits::BATCH_BYTES {
        return Err(HelperRecordError::FrameTooLarge);
    }
    let payload_len =
        u32::try_from(payload.len()).map_err(|_| HelperRecordError::FrameTooLarge)?;
    let mut encoded = Vec::with_capacity(HELPER_HEADER_BYTES + payload.len());
    encoded.extend_from_slice(&HELPER_MAGIC);
    encoded.extend_from_slice(&HELPER_VERSION.to_le_bytes());
    encoded.push(kind as u8);
    encoded.extend_from_slice(&payload_len.to_le_bytes());
    encoded.extend_from_slice(&payload);
    Ok(encoded)
}

pub fn decode_record(encoded: &[u8]) -> Result<HelperRecord, HelperRecordError> {
    let header = encoded
        .get(..HELPER_HEADER_BYTES)
        .ok_or(HelperRecordError::FrameTooShort)?;
    if header[..4] != HELPER_MAGIC {
        return Err(HelperRecordError::InvalidMagic);
    }
    if u16::from_le_bytes([header[4], header[5]]) != HELPER_VERSION {
        return Err(HelperRecordError::UnsupportedVersion);
    }
    let kind = RecordKind::try_from(header[6])?;
    let declared =
        u32::from_le_bytes([header[7], header[8], header[9], header[10]]) as usize;
    if declared > SuggestionLimits::BATCH_BYTES {
        return Err(HelperRecordError::FrameTooLarge);
    }
    let payload = encoded
        .get(HELPER_HEADER_BYTES..)
        .ok_or(HelperRecordError::FrameTooShort)?;
    if payload.len() != declared {
        return Err(HelperRecordError::LengthMismatch);
    }
    let mut reader = PayloadReader::new(payload);
    let record = match kind {
        RecordKind::Request => {
            let buffer = reader.string(SuggestionLimits::BUFFER_BYTES)?;
            let cursor_byte = reader.usize()?;
            let adapter_generation = reader.u64()?;
            let replacement_span = reader.span()?;
            let selection = match reader.u8()? {
                0 => None,
                1 => Some(reader.span()?),
                _ => return Err(HelperRecordError::InvalidPayload),
            };
            let quote_context = decode_quote(reader.u8()?)?;
            let count = reader.usize()?;
            if count > SuggestionLimits::CANDIDATE_COUNT {
                return Err(HelperRecordError::InvalidPayload);
            }
            let mut native_candidates = Vec::with_capacity(count);
            for _ in 0..count {
                native_candidates.push(reader.string(SuggestionLimits::CANDIDATE_BYTES)?);
            }
            HelperRecord::Request(HelperRequest {
                buffer,
                cursor_byte,
                adapter_generation,
                replacement_span,
                selection,
                quote_context,
                native_candidates,
            })
        }
        RecordKind::Accept => HelperRecord::Accept(HelperAccept {
            adapter_generation: reader.u64()?,
            request_id: reader.u64()?,
            candidate_id: reader.u64()?,
        }),
        RecordKind::Dismiss => HelperRecord::Dismiss(HelperDismiss {
            adapter_generation: reader.u64()?,
            reason: HelperDismissReason::try_from(reader.u8()?)?,
        }),
        RecordKind::Replace => HelperRecord::Replace(HelperReplace {
            adapter_generation: reader.u64()?,
            request_id: reader.u64()?,
            replacement_span: reader.span()?,
            insertion: reader.string(SuggestionLimits::CANDIDATE_BYTES)?,
        }),
        RecordKind::Status => HelperRecord::Status(HelperStatus {
            code: HelperStatusCode::try_from(reader.u8()?)?,
        }),
    };
    if !reader.is_empty() {
        return Err(HelperRecordError::InvalidPayload);
    }
    record.validate()?;
    Ok(record)
}

fn put_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn put_usize(output: &mut Vec<u8>, value: usize) -> Result<(), HelperRecordError> {
    let value = u32::try_from(value).map_err(|_| HelperRecordError::FrameTooLarge)?;
    output.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

fn put_span(
    output: &mut Vec<u8>,
    span: ReplacementSpan,
) -> Result<(), HelperRecordError> {
    put_usize(output, span.start)?;
    put_usize(output, span.end)
}

fn put_string(output: &mut Vec<u8>, value: &str) -> Result<(), HelperRecordError> {
    put_usize(output, value.len())?;
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_quote(value: QuoteContext) -> u8 {
    match value {
        QuoteContext::Unquoted => 1,
        QuoteContext::SingleQuoted => 2,
        QuoteContext::DoubleQuoted => 3,
        QuoteContext::Escaped => 4,
        QuoteContext::ShellSpecific => 5,
    }
}

fn decode_quote(value: u8) -> Result<QuoteContext, HelperRecordError> {
    match value {
        1 => Ok(QuoteContext::Unquoted),
        2 => Ok(QuoteContext::SingleQuoted),
        3 => Ok(QuoteContext::DoubleQuoted),
        4 => Ok(QuoteContext::Escaped),
        5 => Ok(QuoteContext::ShellSpecific),
        _ => Err(HelperRecordError::InvalidPayload),
    }
}

struct PayloadReader<'a> {
    remaining: &'a [u8],
}

impl<'a> PayloadReader<'a> {
    const fn new(remaining: &'a [u8]) -> Self {
        Self { remaining }
    }

    const fn is_empty(&self) -> bool {
        self.remaining.is_empty()
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], HelperRecordError> {
        let Some((value, rest)) = self.remaining.split_at_checked(count) else {
            return Err(HelperRecordError::FrameTooShort);
        };
        self.remaining = rest;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, HelperRecordError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, HelperRecordError> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| HelperRecordError::FrameTooShort)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, HelperRecordError> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| HelperRecordError::FrameTooShort)?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn usize(&mut self) -> Result<usize, HelperRecordError> {
        Ok(self.u32()? as usize)
    }

    fn span(&mut self) -> Result<ReplacementSpan, HelperRecordError> {
        Ok(ReplacementSpan {
            start: self.usize()?,
            end: self.usize()?,
        })
    }

    fn string(&mut self, maximum: usize) -> Result<String, HelperRecordError> {
        let length = self.usize()?;
        if length > maximum {
            return Err(HelperRecordError::FrameTooLarge);
        }
        let bytes = self.take(length)?;
        let value =
            std::str::from_utf8(bytes).map_err(|_| HelperRecordError::InvalidUtf8)?;
        Ok(value.to_owned())
    }
}
