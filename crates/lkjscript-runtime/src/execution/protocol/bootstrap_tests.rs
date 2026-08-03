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

fn encoded(value: &ProcessBootstrap) -> Vec<u8> {
    let mut bytes = Vec::new();
    write_bootstrap(&mut bytes, value).expect("encode bootstrap");
    bytes
}

#[test]
fn every_bootstrap_field_changes_the_exact_frame() {
    let expected = bootstrap();
    let frame = encoded(&expected);
    let mut mutations = Vec::new();
    for field in 0..14 {
        let mut value = expected.clone();
        match field {
            0 => value.platform_revision += 1,
            1 => value.contract = [11; 32],
            2 => value.coordinator += 1,
            3 => value.application += 1,
            4 => value.incarnation += 1,
            5 => value.package = [12; 32],
            6 => value.entry.push('x'),
            7 => value.expected_entry = [13; 32],
            8 => {
                value.expected_prepared =
                    PreparedProgramIdentity::new([14; 32]).expect("prepared mutation")
            }
            9 => value.expected_return_semantic = [15; 32],
            10 => value.expected_root_witness_group = [16; 32],
            11 => value.expected_root_witness_member = [17; 32],
            12 => {
                value.capabilities.pop();
            }
            _ => value.execution.instruction_fuel += 1,
        }
        mutations.push(value);
    }
    for mutation in mutations {
        let changed = encoded(&mutation);
        assert_ne!(changed, frame);
        assert_eq!(
            read_bootstrap(&mut changed.as_slice()).expect("decode"),
            mutation
        );
    }
}

#[test]
fn bootstrap_rejects_each_zero_expected_identity() {
    for field in 0..5 {
        let mut value = bootstrap();
        match field {
            0 => value.expected_entry = [0; 32],
            1 => value.expected_prepared = PreparedProgramIdentity::UNBOUND,
            2 => value.expected_return_semantic = [0; 32],
            3 => value.expected_root_witness_group = [0; 32],
            _ => value.expected_root_witness_member = [0; 32],
        }
        assert!(write_bootstrap(&mut Vec::new(), &value).is_err());
    }
}
