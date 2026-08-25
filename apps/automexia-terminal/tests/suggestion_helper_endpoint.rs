use std::io::{self, Cursor, Read, Write};

use automexia_devops::suggestions::{
    decode_submission_frame, encode_reply_frame, AcceptanceContext,
    NativeEditorReplacement, NativeEditorReply, NativeEditorStatus,
    NativeEditorStatusCode,
};
use automexia_terminal::automexia::suggestions::{
    FramedHelperEndpoint, HelperEndpointExchange, HelperSessionBinding,
    HelperSessionBridge,
};

mod support {
    include!("support/suggestion_helper_endpoint.rs");
}

struct Duplex {
    response: Cursor<Vec<u8>>,
    request: Vec<u8>,
    maximum: usize,
}

impl Read for Duplex {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = output.len().min(self.maximum);
        self.response.read(&mut output[..count])
    }
}

impl Write for Duplex {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        let count = input.len().min(self.maximum);
        self.request.extend_from_slice(&input[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn framed_endpoint_writes_authenticated_submission_and_reads_fragmented_replacement() {
    let binding: HelperSessionBinding = support::binding();
    let mut bridge = HelperSessionBridge::new(binding).unwrap();
    let submission = bridge
        .translate_request(support::helper_request(1, "checkout"))
        .unwrap();
    let replacement = NativeEditorReplacement::from_candidate(
        &submission.request,
        &submission.batches[0].candidates[0].candidate,
        &AcceptanceContext::from_request(&submission.request),
    )
    .unwrap();
    let expected_response =
        encode_reply_frame(&NativeEditorReply::Replacement(replacement.clone())).unwrap();

    for maximum in 1..=13 {
        let stream = Duplex {
            response: Cursor::new(expected_response.clone()),
            request: Vec::new(),
            maximum,
        };
        let mut endpoint = FramedHelperEndpoint::new(stream);
        assert_eq!(
            endpoint.exchange(&submission).unwrap(),
            Some(replacement.clone())
        );
        let stream = endpoint.into_inner();
        assert_eq!(
            decode_submission_frame(&stream.request).unwrap(),
            submission
        );
    }
}

#[test]
fn authenticated_status_is_accepted_and_stale_status_is_rejected() {
    let submission = HelperSessionBridge::new(support::binding())
        .unwrap()
        .translate_request(support::helper_request(1, "checkout"))
        .unwrap();
    let status = NativeEditorStatus::from_request(
        &submission.request,
        NativeEditorStatusCode::NoCandidates,
    );
    let response =
        encode_reply_frame(&NativeEditorReply::Status(status.clone())).unwrap();
    for maximum in 1..=13 {
        let stream = Duplex {
            response: Cursor::new(response.clone()),
            request: Vec::new(),
            maximum,
        };
        let mut endpoint = FramedHelperEndpoint::new(stream);
        assert_eq!(endpoint.exchange(&submission).unwrap(), None);
        assert_eq!(
            decode_submission_frame(&endpoint.into_inner().request).unwrap(),
            submission
        );
    }

    let mut stale = status;
    stale.buffer_generation += 1;
    let response = encode_reply_frame(&NativeEditorReply::Status(stale)).unwrap();
    let mut endpoint = FramedHelperEndpoint::new(Duplex {
        response: Cursor::new(response),
        request: Vec::new(),
        maximum: 17,
    });
    assert!(endpoint.exchange(&submission).is_err());
}
#[test]
fn oversized_response_prefix_is_rejected_before_payload_read() {
    let submission = HelperSessionBridge::new(support::binding())
        .unwrap()
        .translate_request(support::helper_request(1, "checkout"))
        .unwrap();
    let stream = Duplex {
        response: Cursor::new((1_048_577_u32).to_le_bytes().to_vec()),
        request: Vec::new(),
        maximum: 4,
    };
    let mut endpoint = FramedHelperEndpoint::new(stream);
    assert!(endpoint.exchange(&submission).is_err());
}
