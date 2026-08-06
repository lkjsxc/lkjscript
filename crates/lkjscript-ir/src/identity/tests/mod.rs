#![allow(clippy::expect_used)]

mod sensitivity;

use crate::*;

fn base() -> Program {
    crate::tests::fixtures::one_block_program()
}
fn identity(program: Program) -> VerifiedProgramIdentity {
    verified_program_identity(&verify(program).expect("identity fixture must verify"))
        .expect("identity must compute")
}
fn hex(value: VerifiedProgramIdentity) -> String {
    value
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn canonical_known_vector_and_repeat_stability() {
    let first = identity(base());
    assert!(first == identity(base()));
    assert_eq!(
        hex(first),
        "5d90e618aa8f24a8e260f0825da3d772d831764a67c143eca9e025d87aa6933b"
    );
}

#[test]
fn prepared_binding_rejects_zero_and_stale_identity() {
    let verified = verify(base()).expect("verify unbound fixture");
    assert!(bind_prepared_identity(
        verified.clone(),
        lkjscript_contracts::PreparedProgramIdentity::UNBOUND,
    )
    .is_err());
    let first = lkjscript_contracts::PreparedProgramIdentity::new([21; 32])
        .expect("first prepared identity");
    let second = lkjscript_contracts::PreparedProgramIdentity::new([22; 32])
        .expect("second prepared identity");
    let bound = bind_prepared_identity(verified, first).expect("bind prepared identity");
    assert!(bound.require_prepared_identity(first).is_ok());
    assert!(bound.require_prepared_identity(second).is_err());
    assert!(bind_prepared_identity(bound, second).is_err());
}

#[test]
fn f64_identity_preserves_exact_bits() {
    fn floating(bits: u64) -> Program {
        let mut program = base();
        let function = &mut program.functions[0];
        *function.signature.result = SsaType::F64;
        let instruction = &mut function.blocks[0].instructions[0];
        instruction.ty = SsaType::F64;
        instruction.kind = InstructionKind::Constant(Constant::F64(f64::from_bits(bits)));
        program
    }
    assert!(identity(floating(0)) != identity(floating(1_u64 << 63)));
    assert!(identity(floating(0x7ff8_0000_0000_0001)) != identity(floating(0x7ff8_0000_0000_0002)));
}

#[test]
fn runtime_process_state_is_not_identity_input() {
    let before = identity(base());
    let allocation: Vec<_> = (0_u64..4096).map(u64::to_be_bytes).collect();
    let from_thread = std::thread::spawn(|| identity(base()))
        .join()
        .expect("thread must finish");
    assert_eq!(allocation.len(), 4096);
    assert!(before == from_thread);
    assert!(before == identity(base()));
}

#[test]
fn identity_authority_does_not_use_debug_text() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/identity");
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
