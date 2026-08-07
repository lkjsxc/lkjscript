use super::*;

#[test]
fn strict_codec_rejects_every_json_boundary_and_removed_profile_field() {
    let root = case_dir("codec").join("main.lkjscript");
    std::fs::write(
        &root,
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n",
    )
    .expect("write source");
    let valid =
        String::from_utf8(request(&root, "{\"kind\":\"snapshot\"}")).expect("request UTF-8");
    let duplicate = valid.replacen(
        "{",
        &format!("{{\"schema\":\"{}\",", crate::semantic::SCHEMA),
        1,
    );
    let unknown_field = valid.replacen("\"contract\":", "\"unknown\":false,\"contract\":", 1);
    let removed_profile = valid.replacen("\"root\":", "\"profile\":\"default\",\"root\":", 1);
    let invalid = [
        duplicate,
        unknown_field,
        removed_profile,
        valid.replace(&crate::semantic::CONTRACT.to_hex(), &"0".repeat(64)),
        valid.replace(crate::semantic::SCHEMA, "lkjscript.agent-foundation"),
        valid.replace("\"kind\":\"snapshot\"", "\"kind\":\"invented\""),
        format!("{valid} false"),
    ];
    for malformed in invalid {
        assert!(
            crate::semantic::execute(malformed.as_bytes()).is_err(),
            "accepted {malformed}"
        );
    }
    assert!(crate::semantic::execute(&[b'{', b'"', 0xff, b'"', b'}']).is_err());
    let oversized = vec![b' '; crate::semantic::MAX_REQUEST_BYTES + 1];
    assert!(crate::semantic::execute(&oversized).is_err());
}

#[test]
fn deeply_nested_json_decodes_and_malformed_input_is_deterministic() {
    let root = case_dir("deep-json").join("main.lkjscript");
    let depth = 1024;
    let mut expression = "{\"kind\":\"unit\"}".to_string();
    for _ in 0..depth {
        expression = format!("{{\"kind\":\"do\",\"expressions\":[{expression}]}}");
    }
    let operation = format!(
        concat!(
            "{{\"kind\":\"apply-transaction\",\"mode\":\"preview\",",
            "\"base_revision\":\"r\",\"file_preconditions\":[],",
            "\"operations\":[{{\"kind\":\"replace-expression\",",
            "\"declaration_key\":\"k\",\"entity_fingerprint\":\"f\",",
            "\"node\":0,\"node_fingerprint\":\"n\",\"expression\":{expression}}}]}}"
        ),
        expression = expression,
    );
    let encoded = request(&root, &operation);
    assert!(encoded.len() < crate::semantic::MAX_REQUEST_BYTES);
    let malformed = encoded[..encoded.len() - depth].to_vec();
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let decoded = crate::semantic::codec::decode_request(&encoded).expect("deep request");
            assert_eq!(
                crate::semantic::codec::measure_json(&decoded).expect("measure deep request"),
                encoded.len()
            );
            drop(decoded);
            let first = crate::semantic::codec::decode_request(&malformed)
                .expect_err("malformed deep request")
                .message;
            let second = crate::semantic::codec::decode_request(&malformed)
                .expect_err("repeat malformed deep request")
                .message;
            assert_eq!(first, second);
        })
        .expect("spawn small-stack JSON test")
        .join()
        .expect("deep JSON test thread");
}

#[test]
fn closed_nested_kinds_and_operations_reject_unknown_input() {
    use crate::semantic::schema::{
        ClosedBuiltinOperation, SemanticNodeKind, SemanticNodeValue, TypeExpression,
    };

    assert!(serde_json::from_str::<SemanticNodeKind>("\"invented\"").is_err());
    assert!(serde_json::from_str::<SemanticNodeValue>("{\"kind\":\"invented\"}").is_err());
    assert!(serde_json::from_str::<ClosedBuiltinOperation>("\"invented\"").is_err());
    assert!(serde_json::from_str::<TypeExpression>("{\"kind\":\"unit\",\"extra\":false}").is_err());
    let root = case_dir("nested-codec").join("main.lkjscript");
    for expression in [
        "{\"kind\":\"invented\"}",
        "{\"kind\":\"unit\",\"extra\":false}",
        "{\"kind\":\"builtin-call\",\"operation\":\"invented\",\"arguments\":[]}",
    ] {
        let operation = format!(
            concat!(
                "{{\"kind\":\"apply-transaction\",\"mode\":\"preview\",",
                "\"base_revision\":\"r\",\"file_preconditions\":[],",
                "\"operations\":[{{\"kind\":\"replace-expression\",",
                "\"declaration_key\":\"k\",\"entity_fingerprint\":\"f\",",
                "\"node\":0,\"node_fingerprint\":\"n\",\"expression\":{expression}}}]}}"
            ),
            expression = expression
        );
        assert!(
            crate::semantic::execute(&request(&root, &operation)).is_err(),
            "accepted closed expression {expression}"
        );
    }
}

#[test]
fn transaction_operation_count_is_not_a_codec_admission_quota() {
    let root = case_dir("operation-count").join("main.lkjscript");
    std::fs::write(
        &root,
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n",
    )
    .expect("write source");
    let rename = concat!(
        "{\"kind\":\"rename-declaration\",\"declaration_key\":\"k\",",
        "\"entity_fingerprint\":\"f\",\"new_name\":\"n\"}",
    );
    let operations = std::iter::repeat_n(rename, 65)
        .collect::<Vec<_>>()
        .join(",");
    let operation = format!(
        concat!(
            "{{\"kind\":\"apply-transaction\",\"mode\":\"preview\",",
            "\"base_revision\":\"r\",\"file_preconditions\":[],",
            "\"operations\":[{operations}]}}",
        ),
        operations = operations
    );
    let encoded = crate::semantic::execute(&request(&root, &operation))
        .expect("operation count reaches semantic validation rather than codec rejection");
    assert!(matches!(
        response(&encoded).result,
        crate::semantic::schema::ResponseResult::Error { .. }
    ));
}

#[test]
fn response_output_byte_policy_fails_before_publication() {
    let response = crate::semantic::schema::Response {
        schema: crate::semantic::SCHEMA.to_string(),
        contract: crate::semantic::CONTRACT.to_hex(),
        compiler_build: "x".repeat(crate::semantic::codec::MAX_OUTPUT_BYTES),
        revision: None,
        charges: crate::semantic::schema::Charges::default(),
        result: crate::semantic::schema::ResponseResult::Error {
            error: Box::new(crate::semantic::schema::ProtocolError {
                code: crate::semantic::schema::ProtocolErrorCode::OutputLimit,
                message: "bounded".to_string(),
                diagnostic: None,
            }),
            diagnostic: None,
        },
    };
    assert!(crate::semantic::codec::prepare_response(
        response,
        crate::semantic::codec::MAX_OUTPUT_BYTES,
    )
    .is_err());
}

#[test]
fn snapshot_is_deterministic_and_unicode_is_preserved() {
    let root = case_dir("determinism").join("main.lkjscript");
    std::fs::write(
        &root,
        "main/\nsig/\ninputs/\n/inputs\noutput/\nstring\n/output\n/sig\nstring-literal/\nλ and \u{1f642}\n/string-literal\n/main\n",
    )
    .expect("write Unicode source");
    let request = request(&root, "{\"kind\":\"snapshot\"}");
    let first = crate::semantic::execute(&request).expect("first snapshot");
    let second = crate::semantic::execute(&request).expect("second snapshot");
    assert_eq!(first, second);
    let decoded = response(&first);
    assert_eq!(decoded.schema, crate::semantic::SCHEMA);
    assert!(String::from_utf8(first)
        .expect("UTF-8 response")
        .contains("λ and 🙂"));
}
