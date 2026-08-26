//! Checked record IO for the session helper's inherited channels.

use std::fmt;
use std::io::{self, Read, Write};

use automexia_command_productivity::suggestions::helper::{
    decode_record, encode_record, HelperRecord, HelperRecordError, HELPER_HEADER_BYTES,
};
use automexia_command_productivity::suggestions::SuggestionLimits;

#[derive(Debug)]
pub enum HelperTransportError {
    Io(io::Error),
    Record(HelperRecordError),
}

impl fmt::Display for HelperTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "suggestion helper IO failed: {error}"),
            Self::Record(error) => {
                write!(formatter, "suggestion helper record failed: {error}")
            }
        }
    }
}

impl std::error::Error for HelperTransportError {}

pub fn read_helper_record(
    reader: &mut impl Read,
) -> Result<HelperRecord, HelperTransportError> {
    let mut header = [0_u8; HELPER_HEADER_BYTES];
    reader
        .read_exact(&mut header)
        .map_err(HelperTransportError::Io)?;
    let declared =
        u32::from_le_bytes(header[7..11].try_into().expect("fixed header")) as usize;
    if declared > SuggestionLimits::BATCH_BYTES {
        return Err(HelperTransportError::Record(
            HelperRecordError::FrameTooLarge,
        ));
    }
    let mut encoded = Vec::with_capacity(HELPER_HEADER_BYTES + declared);
    encoded.extend_from_slice(&header);
    encoded.resize(HELPER_HEADER_BYTES + declared, 0);
    reader
        .read_exact(&mut encoded[HELPER_HEADER_BYTES..])
        .map_err(HelperTransportError::Io)?;
    decode_record(&encoded).map_err(HelperTransportError::Record)
}

pub fn write_helper_record(
    writer: &mut impl Write,
    record: &HelperRecord,
) -> Result<(), HelperTransportError> {
    let encoded = encode_record(record).map_err(HelperTransportError::Record)?;
    writer
        .write_all(&encoded)
        .map_err(HelperTransportError::Io)?;
    writer.flush().map_err(HelperTransportError::Io)
}
