//! Exact witness calls retain caller effect and requirement applications in derived bytes.
use super::*;
use crate::platform::kernel::{EffectRow, KernelSnapshot};

const SOURCE: &str = r#"declarations.begin
(units (module create owned-effects
  (interface create Clock (visibility public)
    (operation create now (returns I64)
      (idempotency idempotent) (external-visibility none)))
  (function create ready (visibility private) (returns I64) (effect pure) (body (i64 0)))
  (component create authority (visibility public)
    (requirement create first (interface Clock) (operations Clock::now)
      (limits (maximum_calls 4 calls)))
    (requirement create second (interface Clock) (operations Clock::now)
      (limits (maximum_calls 4 calls)))
    (port create ready (type (function () I64)) (function ready)))
  (owned-contract create Storage (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_88000000000000000000000000000001 read
      (parameters (Self borrow)) (returns I64)))
  (external create read-cell (visibility private) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (function create cell-read (visibility public) (effect pure)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64)
    (body (call read-cell (local value))))
  (owned-implementation create Cell (visibility public) (contract Storage) (self OwnedI64Cell)
    (method method_88000000000000000000000000000001 cell-read))
  (function create relay (visibility public)
    (type-parameter create T (constraint owned))
    (effect-parameter create E)
    (requirement-parameter create R (interface Clock) (operations Clock::now))
    (implementation-parameter implparam_88000000000000000000000000000001 ops Storage T)
    (parameter create value (type T) (use consume)) (returns T)
    (effect (task (parameter E) (requirement R)))
    (body (local value)))
  (function create entry (visibility public)
    (parameter create value (type OwnedI64Cell) (use consume)) (returns OwnedI64Cell)
    (effect (task (requirement authority::first) (requirement authority::second)))
    (body (implementation-call relay (types OwnedI64Cell)
      (effects (row (requirement authority::first))) (requirements authority::second)
      (implementations concrete@Cell) (local value))))
  (function create alternate (visibility public)
    (parameter create value (type OwnedI64Cell) (use consume)) (returns OwnedI64Cell)
    (effect (task (requirement authority::first) (requirement authority::second)))
    (body (implementation-call relay (types OwnedI64Cell)
      (effects (row (requirement authority::second))) (requirements authority::first)
      (implementations concrete@Cell) (local value))))))
declarations.end
"#;

fn fixture() -> (KernelSnapshot, LoadedArtifact) {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let repository =
        GraphRepository::create(&directory.path().join("owned-effects"), &source, None)
            .unwrap()
            .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let linked = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    (source, load_artifact(&linked.artifact.bytes).unwrap())
}

#[test]
fn owned_effect_applications_preserve_exact_operands_and_requirement_tables() {
    let (source, loaded) = fixture();
    let mut applications = Vec::new();
    for bytes in loaded.objects.values() {
        if !bytes.starts_with(super::super::unit::COMPILER_UNIT_MAGIC.as_slice()) {
            continue;
        }
        let key = ObjectKey::for_bytes(ObjectDomain::CompilerUnit, bytes);
        let unit = CompilationUnit::decode(bytes, key).unwrap();
        let CompilationPayload::Function { code, .. } = &unit.payload else {
            continue;
        };
        for instruction in &code.instructions {
            if let CompiledInstruction::ImplementationCall {
                requirement_arguments,
                effect_arguments,
                ..
            } = instruction
            {
                let OwnerRecord::Declaration(declaration) = &source.owners[&unit.source.owner]
                else {
                    panic!("callable owner");
                };
                let (expected_effect, expected_requirement) = match declaration.name.as_str() {
                    "entry" => ("first", "second"),
                    "alternate" => ("second", "first"),
                    _ => panic!("unexpected witness caller"),
                };
                assert_eq!(requirement_arguments.len(), 1);
                assert_eq!(effect_arguments.len(), 1);
                assert_eq!(effect_arguments[0].requirements.len(), 1);
                for (operand, expected) in [
                    (effect_arguments[0].requirements[0], expected_effect),
                    (requirement_arguments[0], expected_requirement),
                ] {
                    let OwnerRecord::Requirement(requirement) = &source.owners[&operand.owner()]
                    else {
                        panic!("concrete authority");
                    };
                    assert_eq!(requirement.name.as_str(), expected);
                }
                for operand in requirement_arguments
                    .iter()
                    .chain(&effect_arguments[0].requirements)
                {
                    assert!(
                        unit.tables
                            .requirements
                            .contains(&operand.concrete().unwrap())
                    );
                }
                applications.push((requirement_arguments.clone(), effect_arguments.clone()));
            }
        }
    }
    assert_eq!(applications.len(), 2);
    assert_ne!(applications[0], applications[1]);
}

#[test]
fn owned_effect_artifact_rejects_rehashed_erasure_and_requirement_rebinding() {
    let (source, loaded) = fixture();
    let entry = declaration_named(&source, "entry");
    let (old, original) = loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .find(|(_, unit)| unit.source.owner == OwnerKey::Declaration(entry))
        .unwrap();
    for fault in [
        "effect-erasure",
        "requirement-erasure",
        "requirement-rebinding",
    ] {
        let mut changed = original.clone();
        let CompilationPayload::Function { code, .. } = &mut changed.payload else {
            panic!("entry function");
        };
        let (effects, requirements) = code
            .instructions
            .iter_mut()
            .find_map(|instruction| match instruction {
                CompiledInstruction::ImplementationCall {
                    effect_arguments,
                    requirement_arguments,
                    ..
                } => Some((effect_arguments, requirement_arguments)),
                _ => None,
            })
            .unwrap();
        match fault {
            "effect-erasure" => effects[0] = EffectRow::default(),
            "requirement-erasure" => requirements.clear(),
            "requirement-rebinding" => requirements[0] = effects[0].requirements[0],
            _ => unreachable!(),
        }
        let bytes = effect_tests::replace_unit(&loaded, old, &changed, vec![]);
        let error = load_artifact(&bytes).expect_err("rehashing cannot change canonical authority");
        assert_eq!(
            error.code, "artifact_nominal_instruction_meaning",
            "{fault}: {error:?}"
        );
    }
}

#[test]
fn owned_effect_compiler22_bytes_require_rebuild_before_payload_decoding() {
    let bytes = b"LKJCUN22invalid predecessor payload";
    let key = ObjectKey::for_bytes(ObjectDomain::CompilerUnit, bytes);
    let error = CompilationUnit::decode(bytes, key).unwrap_err();
    assert_eq!(error.class, crate::platform::DiagnosticClass::Source);
    assert_eq!(error.code, "compiler_unit_contract");
}
