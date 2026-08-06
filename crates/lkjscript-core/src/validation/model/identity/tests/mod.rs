#![allow(clippy::expect_used)]

mod fixtures;
mod sensitivity;

use super::*;
use crate::{validate_chunk, Chunk, Op, ValidationPolicy};

fn unit() -> ValidatedChunk {
    let mut chunk = Chunk::new();
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    validate_chunk(chunk, ValidationPolicy::Unrestricted).expect("unit chunk must validate")
}
fn identity(value: &ValidatedChunk) -> ValidatedBytecodeIdentity {
    validated_bytecode_identity(value).expect("identity must compute")
}
fn hex(value: ValidatedBytecodeIdentity) -> String {
    value
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn canonical_known_vector_and_repeat_stability() {
    let chunk = unit();
    let first = identity(&chunk);
    assert!(first == identity(&chunk));
    assert_eq!(
        hex(first),
        "f11aabe8c09a1d8a07554b2bd80612e7590af49dac904c90154751e404198758"
    );
}
#[test]
fn prepared_binding_rejects_zero_and_stale_identity() {
    let validated = unit();
    assert!(crate::bind_prepared_identity(
        validated.clone(),
        lkjscript_contracts::PreparedProgramIdentity::UNBOUND,
    )
    .is_err());
    let first = lkjscript_contracts::PreparedProgramIdentity::new([31; 32])
        .expect("first prepared identity");
    let second = lkjscript_contracts::PreparedProgramIdentity::new([32; 32])
        .expect("second prepared identity");
    let bound = crate::bind_prepared_identity(validated, first).expect("bind prepared bytecode");
    assert!(bound.require_prepared_identity(first).is_ok());
    assert!(bound.require_prepared_identity(second).is_err());
    assert!(crate::bind_prepared_identity(bound, second).is_err());
}

#[test]
fn exact_f64_bits_are_bound() {
    fn with(bits: u64) -> ValidatedChunk {
        let mut chunk = Chunk::new();
        chunk
            .constants
            .push(crate::Constant::F64(f64::from_bits(bits)));
        chunk.main.emit(Op::Unit);
        chunk.main.emit(Op::Return);
        validate_chunk(chunk, ValidationPolicy::Unrestricted).expect("f64 chunk must validate")
    }
    assert!(identity(&with(0)) != identity(&with(1_u64 << 63)));
    assert!(identity(&with(0x7ff8_0000_0000_0001)) != identity(&with(0x7ff8_0000_0000_0002)));
}
#[test]
fn runtime_process_state_is_not_identity_input() {
    let chunk = unit();
    let before = identity(&chunk);
    let allocation: Vec<_> = (0_u64..4096).map(u64::to_be_bytes).collect();
    let from_thread = std::thread::spawn(move || identity(&chunk))
        .join()
        .expect("thread must finish");
    assert_eq!(allocation.len(), 4096);
    assert!(before == from_thread);
}
#[test]
fn identity_authority_does_not_use_debug_text() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/validation/model/identity");
    let mut pending = vec![root];
    while let Some(path) = pending.pop() {
        for entry in std::fs::read_dir(path).expect("identity source directory must exist") {
            let path = entry
                .expect("identity source entry must be readable")
                .path();
            if path.is_dir() {
                if path.file_name().and_then(std::ffi::OsStr::to_str) != Some("tests") {
                    pending.push(path);
                }
                continue;
            }
            if path.extension().and_then(std::ffi::OsStr::to_str) != Some("rs") {
                continue;
            }
            let source = std::fs::read_to_string(path).expect("identity source must be readable");
            assert!(!source.contains("{:?}"));
            assert!(!source.contains("format!(\"{value:?}"));
            assert!(!source.contains(".to_string()"));
        }
    }
}
