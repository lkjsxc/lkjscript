#![allow(clippy::expect_used)]

use super::*;

fn bootstrap() -> ProcessBootstrap {
    ProcessBootstrap {
        platform_revision: 4,
        contract: [1; 32],
        coordinator: 2,
        application: 3,
        incarnation: 4,
        package: [5; 32],
        entry: "/tmp/package/main.lkjscript".into(),
        capabilities: vec![CapabilityKind::Arguments, CapabilityKind::Stdio],
        execution: ExecutionConfig::default(),
    }
}

#[test]
fn process_protocol_round_trips_closed_messages() {
    let mut bytes = Vec::new();
    write_bootstrap(&mut bytes, &bootstrap()).expect("bootstrap encode");
    assert_eq!(
        read_bootstrap(&mut bytes.as_slice()).expect("bootstrap decode"),
        bootstrap()
    );

    for request in [
        ProcessRequest::Invoke {
            cell: 9,
            arguments: vec!["one".into(), "two".into()],
        },
        ProcessRequest::Stop,
    ] {
        bytes.clear();
        write_request(&mut bytes, &request).expect("request encode");
        assert_eq!(
            read_request(&mut bytes.as_slice()).expect("request decode"),
            request
        );
    }

    let response = ProcessResponse::Outcome {
        cell: 9,
        outcome: ExecutionOutcome::Returned(
            lkjscript_core::OwnedValue::from_unique_bytes(vec![1, 2, 3]).expect("owned bytes"),
        ),
        output: b"private output".to_vec(),
        flushes: 1,
    };
    bytes.clear();
    write_response(&mut bytes, &response).expect("response encode");
    assert_eq!(
        read_response(&mut bytes.as_slice()).expect("response decode"),
        response
    );
}

#[test]
fn process_protocol_rejects_bounds_unknown_tags_and_trailing_bytes() {
    let mut oversized = Vec::new();
    oversized.extend_from_slice(&u32::MAX.to_le_bytes());
    assert!(read_request(&mut oversized.as_slice()).is_err());

    let mut unknown = Vec::new();
    write_frame(&mut unknown, vec![99]).expect("raw frame");
    assert!(read_request(&mut unknown.as_slice()).is_err());

    let request = ProcessRequest::Invoke {
        cell: 1,
        arguments: vec!["x".repeat(MAX_ARGUMENT_BYTES + 1)],
    };
    assert!(write_request(&mut Vec::new(), &request).is_err());

    let mut trailing = Vec::new();
    write_frame(&mut trailing, vec![2, 0]).expect("raw trailing frame");
    assert!(read_request(&mut trailing.as_slice()).is_err());
}
