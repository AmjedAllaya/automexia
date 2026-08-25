//! Shell-readable response envelope emitted by the persistent helper.
//!
//! POSIX shell variables cannot safely retain NUL bytes, so the response uses
//! a strict, newline-terminated ASCII envelope with uppercase hexadecimal UTF-8
//! insertion bytes. Requests remain compact binary records.

use super::helper::{
    HelperRecord, HelperRecordError, HelperReplace, HelperStatus, HelperStatusCode,
};
use super::{ReplacementSpan, SuggestionLimits};

const PREFIX: &str = "AXSR1";
const MAX_RESPONSE_BYTES: usize = 128 + SuggestionLimits::CANDIDATE_BYTES * 2;

pub fn encode_shell_response(
    record: &HelperRecord,
) -> Result<Vec<u8>, HelperRecordError> {
    record.validate()?;
    let line = match record {
        HelperRecord::Replace(replacement) => format!(
            "{PREFIX}\tR\t{}\t{}\t{}\t{}\t{}\n",
            replacement.adapter_generation,
            replacement.request_id,
            replacement.replacement_span.start,
            replacement.replacement_span.end,
            encode_hex(replacement.insertion.as_bytes())
        ),
        HelperRecord::Status(status) => {
            format!("{PREFIX}\tS\t{}\n", status.code as u8)
        }
        _ => return Err(HelperRecordError::UnknownKind),
    };
    if line.len() > MAX_RESPONSE_BYTES {
        return Err(HelperRecordError::FrameTooLarge);
    }
    Ok(line.into_bytes())
}

pub fn decode_shell_response(line: &[u8]) -> Result<HelperRecord, HelperRecordError> {
    if line.len() > MAX_RESPONSE_BYTES {
        return Err(HelperRecordError::FrameTooLarge);
    }
    let text = std::str::from_utf8(line).map_err(|_| HelperRecordError::InvalidUtf8)?;
    if !text.ends_with('\n') || text.contains('\r') || !text.is_ascii() {
        return Err(HelperRecordError::InvalidPayload);
    }
    let fields = text[..text.len() - 1].split('\t').collect::<Vec<_>>();
    if fields.first() != Some(&PREFIX) {
        return Err(HelperRecordError::InvalidMagic);
    }
    let record = match fields.as_slice() {
        [_, "R", generation, request, start, end, insertion] => {
            HelperRecord::Replace(HelperReplace {
                adapter_generation: parse_u64(generation)?,
                request_id: parse_u64(request)?,
                replacement_span: ReplacementSpan {
                    start: parse_usize(start)?,
                    end: parse_usize(end)?,
                },
                insertion: String::from_utf8(decode_hex(insertion)?)
                    .map_err(|_| HelperRecordError::InvalidUtf8)?,
            })
        }
        [_, "S", code] => HelperRecord::Status(HelperStatus {
            code: HelperStatusCode::try_from(parse_u8(code)?)?,
        }),
        _ => return Err(HelperRecordError::InvalidPayload),
    };
    record.validate()?;
    Ok(record)
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn decode_hex(value: &str) -> Result<Vec<u8>, HelperRecordError> {
    if value.is_empty()
        || !value.len().is_multiple_of(2)
        || value.len() > SuggestionLimits::CANDIDATE_BYTES * 2
    {
        return Err(HelperRecordError::InvalidPayload);
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = decode_nibble(pair[0])?;
            let low = decode_nibble(pair[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn decode_nibble(value: u8) -> Result<u8, HelperRecordError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(HelperRecordError::InvalidPayload),
    }
}

fn parse_u64(value: &str) -> Result<u64, HelperRecordError> {
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value != 0)
        .ok_or(HelperRecordError::InvalidPayload)
}

fn parse_usize(value: &str) -> Result<usize, HelperRecordError> {
    value
        .parse::<usize>()
        .map_err(|_| HelperRecordError::InvalidPayload)
}

fn parse_u8(value: &str) -> Result<u8, HelperRecordError> {
    value
        .parse::<u8>()
        .map_err(|_| HelperRecordError::InvalidPayload)
}
