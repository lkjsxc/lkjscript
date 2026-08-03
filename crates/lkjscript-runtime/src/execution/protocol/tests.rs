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
        expected_entry: [6; 32],
        expected_prepared: PreparedProgramIdentity::new([7; 32]).expect("prepared"),
        expected_return_semantic: [8; 32],
        expected_root_witness_group: [9; 32],
        expected_root_witness_member: [10; 32],
        capabilities: vec![CapabilityKind::Arguments, CapabilityKind::Stdio],
        execution: ExecutionConfig::default(),
    }
}

pub(super) fn provenance() -> ProcessProgramProvenance {
    ProcessProgramProvenance {
        platform_revision: 4,
        contract: [1; 32],
        application: 3,
        incarnation: 4,
        package: [5; 32],
        entry: [6; 32],
        prepared: PreparedProgramIdentity::new([7; 32]).expect("prepared"),
        return_semantic: [8; 32],
        root_witness_group: [9; 32],
        root_witness_member: [10; 32],
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
        provenance: provenance(),
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
fn process_protocol_rejects_each_provenance_mutation() {
    let expected = provenance();
    let mut mutations = Vec::new();
    for field in 0..10 {
        let mut value = expected.clone();
        match field {
            0 => value.platform_revision += 1,
            1 => value.contract = [11; 32],
            2 => value.application += 1,
            3 => value.incarnation += 1,
            4 => value.package = [12; 32],
            5 => value.entry = [13; 32],
            6 => value.prepared = PreparedProgramIdentity::new([14; 32]).expect("mutation"),
            7 => value.return_semantic = [15; 32],
            8 => value.root_witness_group = [16; 32],
            _ => value.root_witness_member = [17; 32],
        }
        mutations.push(value);
    }
    for mutation in mutations {
        assert!(validate_process_provenance(&expected, &mutation).is_err());
    }
    assert!(validate_process_provenance(&expected, &expected).is_ok());
}

#[test]
fn process_protocol_rejects_every_zero_provenance_digest() {
    let mut mutations = Vec::new();
    for field in 0..7 {
        let mut value = provenance();
        match field {
            0 => value.contract = [0; 32],
            1 => value.package = [0; 32],
            2 => value.entry = [0; 32],
            3 => value.prepared = PreparedProgramIdentity::UNBOUND,
            4 => value.return_semantic = [0; 32],
            5 => value.root_witness_group = [0; 32],
            _ => value.root_witness_member = [0; 32],
        }
        mutations.push(value);
    }
    for provenance in mutations {
        assert!(write_response(
            &mut Vec::new(),
            &ProcessResponse::Ready {
                process: 1,
                provenance,
            },
        )
        .is_err());
    }
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
