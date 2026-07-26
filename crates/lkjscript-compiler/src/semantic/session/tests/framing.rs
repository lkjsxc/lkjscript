use std::io::Cursor;

use super::*;
use crate::semantic::session::{
    MAX_SESSION_CUMULATIVE_INPUT_BYTES, MAX_SESSION_CUMULATIVE_OUTPUT_BYTES,
    MAX_SESSION_FRAME_BYTES,
};

#[test]
fn clean_eof_has_no_output() {
    let mut input = Cursor::new(Vec::<u8>::new());
    let mut output = Vec::new();
    crate::semantic::session::serve(&mut input, &mut output).expect("clean session EOF");
    assert!(output.is_empty());
}

#[test]
fn framing_rejects_partial_and_oversized_input_before_payload_allocation() {
    let cases = [
        (vec![0, 0, 0], "partial_header"),
        (
            {
                let mut bytes = 2_u64.to_be_bytes().to_vec();
                bytes.push(b'{');
                bytes
            },
            "partial_payload",
        ),
        (
            (MAX_SESSION_FRAME_BYTES + 1).to_be_bytes().to_vec(),
            "frame_too_large",
        ),
        (u64::MAX.to_be_bytes().to_vec(), "frame_too_large"),
    ];
    for (bytes, code) in cases {
        let mut input = Cursor::new(bytes);
        let mut output = Vec::new();
        let error = crate::semantic::session::serve(&mut input, &mut output)
            .expect_err("malformed frame must fail");
        assert!(error.to_string().contains(code), "{error}");
        assert!(output.is_empty());
    }
}

#[test]
fn cumulative_input_is_checked_before_payload_read() {
    let payload = session_request("shutdown", 0, "{\"kind\":\"shutdown\"}");
    let mut session = SemanticSession::new();
    session.input_bytes = MAX_SESSION_CUMULATIVE_INPUT_BYTES;
    let mut input = Cursor::new(frame(&payload));
    let mut output = Vec::new();
    let error = session
        .serve(&mut input, &mut output)
        .expect_err("cumulative input must fail");
    assert!(error.to_string().contains("frame_too_large"));
    assert!(output.is_empty());
}

#[test]
fn cumulative_output_is_reserved_before_write() {
    let mut session = SemanticSession::new();
    session.output_bytes = MAX_SESSION_CUMULATIVE_OUTPUT_BYTES;
    let mut output = Vec::new();
    let error = crate::semantic::session::framing::write_frame(&mut output, b"{}", &mut session)
        .expect_err("cumulative output must fail");
    assert!(error.to_string().contains("frame_too_large"));
    assert!(output.is_empty());
}

#[test]
fn shutdown_is_one_exact_frame_without_contamination() {
    let payload = session_request("shutdown", 0, "{\"kind\":\"shutdown\"}");
    let mut input = Cursor::new(frame(&payload));
    let mut output = Vec::new();
    crate::semantic::session::serve(&mut input, &mut output).expect("serve shutdown");
    assert!(output.len() >= 8);
    let length = u64::from_be_bytes(output[..8].try_into().expect("response header")) as usize;
    assert_eq!(output.len(), length + 8);
    let response: serde_json::Value =
        serde_json::from_slice(&output[8..]).expect("framed response JSON");
    assert_eq!(response["request_id"], "shutdown");
    assert_eq!(response["revision"], 0);
    assert_eq!(response["response"]["kind"], "shutdown");
    assert_eq!(response["response"]["acknowledged"], true);
}

#[test]
fn strict_session_envelope_rejects_malformed_duplicate_and_unknown_input() {
    let valid = String::from_utf8(session_request("strict", 0, "{\"kind\":\"shutdown\"}"))
        .expect("session request UTF-8");
    for invalid in [
        format!("{valid} false"),
        valid.replacen("{", "{\"schema\":\"duplicate\",", 1),
        valid.replacen("\"contract\":", "\"extra\":false,\"contract\":", 1),
        valid.replace(
            &crate::semantic::session::CONTRACT.to_hex(),
            &"0".repeat(64),
        ),
        valid.replace("\"kind\":\"shutdown\"", "\"kind\":\"invented\""),
    ] {
        let error = SemanticSession::new()
            .handle(invalid.as_bytes())
            .expect_err("strict session input must fail");
        assert!(error.to_string().contains("invalid_json"));
    }
    assert!(SemanticSession::new().handle(&[0xff]).is_err());
}
