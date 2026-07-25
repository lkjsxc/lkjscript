use super::*;

#[test]
fn strict_codec_rejects_every_json_boundary() {
    let root = case_dir("codec").join("main.lkjscript");
    std::fs::write(&root, "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n").expect("write source");
    let valid =
        String::from_utf8(request(&root, "{\"kind\":\"snapshot\"}")).expect("request UTF-8");
    let duplicate = valid.replacen(
        "{",
        &format!("{{\"schema\":\"{}\",", crate::semantic::SCHEMA),
        1,
    );
    let unknown_field = valid.replacen("\"version\":1", "\"version\":1,\"unknown\":false", 1);
    let invalid = [
        duplicate,
        unknown_field,
        valid.replace("\"version\":1", "\"version\":2"),
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
    let deeply_nested = format!("{}0{}", "[".repeat(65), "]".repeat(65));
    assert!(crate::semantic::execute(deeply_nested.as_bytes()).is_err());
    let oversized = vec![b' '; crate::semantic::MAX_REQUEST_BYTES + 1];
    assert!(crate::semantic::execute(&oversized).is_err());
}

#[test]
fn aggregate_operation_string_work_and_output_limits_fail_closed() {
    let root = case_dir("aggregate-limits").join("main.lkjscript");
    let rename = concat!(
        "{\"kind\":\"rename_declaration\",\"declaration_key\":\"k\",",
        "\"entity_fingerprint\":\"f\",\"new_name\":\"n\"}",
    );
    let operations = std::iter::repeat_n(rename, crate::semantic::codec::MAX_OPERATIONS + 1)
        .collect::<Vec<_>>()
        .join(",");
    let operation = format!(
        concat!(
            "{{\"kind\":\"apply_transaction\",\"mode\":\"preview\",",
            "\"base_revision\":\"r\",\"file_preconditions\":[],",
            "\"operations\":[{operations}]}}",
        ),
        operations = operations
    );
    assert!(crate::semantic::execute(&request(&root, &operation)).is_err());
    let huge = "x".repeat(crate::semantic::codec::MAX_STRING_BYTES as usize + 1);
    let operation = format!(
        "{{\"kind\":\"snapshot\",\"expected_repository_identity\":{}}}",
        serde_json::to_string(&huge).expect("encode huge string")
    );
    assert!(crate::semantic::execute(&request(&root, &operation)).is_err());
    let charges = crate::semantic::schema::Charges {
        source_nodes: crate::semantic::codec::MAX_SCHEMA_NODES + 1,
        ..crate::semantic::schema::Charges::default()
    };
    let limits = crate::semantic::charges::ProtocolLimits::for_profile(
        crate::semantic::schema::ResourceProfile::Default,
    );
    assert!(limits.check_charges(&charges).is_err());
    let charges = crate::semantic::schema::Charges {
        work_units: crate::semantic::codec::MAX_WORK_UNITS + 1,
        ..crate::semantic::schema::Charges::default()
    };
    assert!(limits.check_charges(&charges).is_err());
    let sandbox = crate::semantic::charges::ProtocolLimits::for_profile(
        crate::semantic::schema::ResourceProfile::Sandbox,
    );
    let charges = crate::semantic::schema::Charges {
        work_units: 786_433,
        ..crate::semantic::schema::Charges::default()
    };
    assert!(sandbox.check_charges(&charges).is_err());
    assert!(limits.check_charges(&charges).is_ok());
    let response = crate::semantic::schema::Response {
        schema: crate::semantic::SCHEMA.to_string(),
        version: 1,
        compiler_build: "x".repeat(crate::semantic::codec::MAX_OUTPUT_BYTES),
        profile: crate::semantic::schema::ResourceProfile::Default,
        profile_identity: crate::semantic::charges::identity(
            crate::semantic::schema::ResourceProfile::Default,
        ),
        limits: crate::semantic::charges::ProtocolLimits::for_profile(
            crate::semantic::schema::ResourceProfile::Default,
        )
        .record(),
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
    assert!(crate::semantic::codec::encode_response(response).is_err());
}

#[test]
fn all_core_profiles_are_closed_protocol_selections() {
    let root = case_dir("profiles").join("main.lkjscript");
    std::fs::write(&root, "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n").expect("write source");
    let default =
        String::from_utf8(request(&root, "{\"kind\":\"snapshot\"}")).expect("request UTF-8");
    for profile in [
        "sandbox",
        "default",
        "build",
        "trusted-local",
        "deterministic",
    ] {
        let selected = default.replace(
            "\"profile\":\"default\"",
            &format!("\"profile\":\"{profile}\""),
        );
        let encoded = crate::semantic::execute(selected.as_bytes()).expect("selected profile");
        let decoded = response(&encoded);
        assert_eq!(decoded.profile.core().name().as_str(), profile);
        assert_eq!(decoded.profile_identity.ceilings_sha256.len(), 64);
    }
    let unknown = default.replace("\"profile\":\"default\"", "\"profile\":\"standard\"");
    assert!(crate::semantic::execute(unknown.as_bytes()).is_err());
}

#[test]
fn snapshot_is_deterministic_and_unicode_is_preserved() {
    let root = case_dir("determinism").join("main.lkjscript");
    std::fs::write(
        &root,
        "main/\nsig/\n->\nStr\n/sig\nstr/\nλ and \u{1f642}\n/str\n/main\n",
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
