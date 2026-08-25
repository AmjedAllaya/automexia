use std::io::{self, Cursor, Read, Write};

use automexia_devops::suggestions::helper::{
    encode_record, HelperRecord, HelperRecordError, HelperStatus, HelperStatusCode,
    HELPER_HEADER_BYTES,
};
use automexia_devops::suggestions::SuggestionLimits;
use automexia_terminal::automexia::suggestions::{
    read_helper_record, write_helper_record, HelperTransportError,
};

struct Fragmented {
    inner: Cursor<Vec<u8>>,
    maximum: usize,
}

impl Read for Fragmented {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = output.len().min(self.maximum);
        self.inner.read(&mut output[..count])
    }
}

struct ShortWriter {
    bytes: Vec<u8>,
    maximum: usize,
}

impl Write for ShortWriter {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        let count = input.len().min(self.maximum);
        self.bytes.extend_from_slice(&input[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn ready() -> HelperRecord {
    HelperRecord::Status(HelperStatus {
        code: HelperStatusCode::Ready,
    })
}

#[test]
fn transport_accepts_every_fragmentation_and_short_write_pattern() {
    let expected = encode_record(&ready()).unwrap();
    for maximum in 1..=HELPER_HEADER_BYTES + 1 {
        let mut reader = Fragmented {
            inner: Cursor::new(expected.clone()),
            maximum,
        };
        assert_eq!(read_helper_record(&mut reader).unwrap(), ready());

        let mut writer = ShortWriter {
            bytes: Vec::new(),
            maximum,
        };
        write_helper_record(&mut writer, &ready()).unwrap();
        assert_eq!(writer.bytes, expected);
    }
}

#[test]
fn declared_limit_is_rejected_before_payload_read_or_allocation() {
    let mut header = encode_record(&ready()).unwrap()[..HELPER_HEADER_BYTES].to_vec();
    header[7..11].copy_from_slice(
        &u32::try_from(SuggestionLimits::BATCH_BYTES + 1)
            .unwrap()
            .to_le_bytes(),
    );
    let mut reader = Cursor::new(header);
    assert!(matches!(
        read_helper_record(&mut reader),
        Err(HelperTransportError::Record(
            HelperRecordError::FrameTooLarge
        ))
    ));
}

#[test]
fn truncated_header_and_payload_remain_distinct_io_failures() {
    let encoded = encode_record(&ready()).unwrap();
    for end in 0..encoded.len() {
        let mut reader = Cursor::new(&encoded[..end]);
        assert!(matches!(
            read_helper_record(&mut reader),
            Err(HelperTransportError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof
        ));
    }
}
