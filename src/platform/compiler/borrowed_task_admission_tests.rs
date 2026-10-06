//! Borrowed task boundaries preserve exact declared bounds, modes and read sources.
use super::*;
use crate::platform::kernel::{KernelSnapshot, ParameterUse, TypeParameterConstraints};

const SOURCE: &str = r#"declarations.begin
(units (module create borrowed-task-artifact
  (owned-contract create Inspector (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_b3000000000000000000000000000001 inspect
      (parameters (Self borrow)) (returns I64) (effect (task))))
  (function create read (visibility public) (effect (task))
    (type-parameter create T (constraint owned shareable))
    (type-parameter create Unused (constraint owned))
    (parameter create payload (type T) (use borrow))
    (returns I64) (body (i64 17)))
  (function create mapped-read (visibility public) (effect (task))
    (type-parameter create T (constraint owned shareable))
    (parameter create payload (type T) (use borrow))
    (returns I64) (body (i64 23)))
  (owned-implementation create GenericInspector (visibility public)
    (type-parameter create T (constraint owned shareable))
    (contract Inspector) (self T)
    (method method_b3000000000000000000000000000001 mapped-read (types T)))
  (function create synchronous (visibility public) (effect (task))
    (type-parameter create T (constraint owned))
    (parameter create payload (type T) (use borrow))
    (returns I64) (body (i64 29)))
  (function create joined (visibility public) (effect (task))
    (type-parameter create T (constraint owned shareable))
    (type-parameter create Unused (constraint owned))
    (parameter create payload (type T) (use borrow))
    (parameter create other (type T) (use borrow))
    (returns (record (left I64) (right I64)))
    (body (parallel (call read (types T Unused) (local payload))
                    (call read (types T Unused) (local payload)))))
  (function create method-reader (visibility public) (effect (task))
    (type-parameter create T (constraint owned shareable))
    (parameter create payload (type T) (use borrow))
    (returns I64)
    (body (method-call (implementation GenericInspector (types T)) Inspector
      method_b3000000000000000000000000000001 (local payload))))))
declarations.end"#;

fn fixture(literal: &str) -> (KernelSnapshot, LoadedArtifact) {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(literal)
            .expect("borrowed tasks authored through the native surface");
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let directory = tempfile::tempdir().unwrap();
    let repository =
        GraphRepository::create(&directory.path().join("borrowed-tasks"), &source, None)
            .unwrap()
            .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let linked = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    let loaded = load_artifact(&linked.artifact.bytes).expect("borrowed task artifact admits");
    (source, loaded)
}

fn unit(
    source: &KernelSnapshot,
    loaded: &LoadedArtifact,
    name: &str,
) -> (ObjectKey, CompilationUnit) {
    let owner = OwnerKey::Declaration(declaration_named(source, name));
    loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .find(|(_, unit)| unit.source.owner == owner)
        .unwrap()
}

#[test]
fn borrowed_task_artifact_admits_share_only_reads_and_stronger_implementation_bounds() {
    for bound in ["owned shareable", "owned transferable shareable"] {
        let (source, loaded) = fixture(&SOURCE.replace("owned shareable", bound));
        for name in [
            "read",
            "mapped-read",
            "joined",
            "method-reader",
            "synchronous",
        ] {
            let (key, original) = unit(&source, &loaded, name);
            load_artifact(&effect_tests::replace_unit(&loaded, key, &original, vec![]))
                .expect("neutral fully rehashed task preserves exact admission");
            let CompilationPayload::Function { signature, .. } = &original.payload else {
                unreachable!()
            };
            assert_eq!(original.source.kind, OwnerKind::TaskFunction);
            assert!(
                signature
                    .parameters
                    .iter()
                    .all(|p| p.use_mode == ParameterUse::Borrow)
            );
            assert_eq!(
                signature.type_parameter_constraints[0],
                if name == "synchronous" {
                    TypeParameterConstraints::Owned
                } else if bound == "owned shareable" {
                    TypeParameterConstraints::OwnedShareable
                } else {
                    TypeParameterConstraints::OwnedTransferableShareable
                }
            );
            if name == "read" || name == "joined" {
                assert_eq!(
                    signature.type_parameter_constraints[1],
                    TypeParameterConstraints::Owned
                );
            }
        }
        let (_, joined) = unit(&source, &loaded, "joined");
        let CompilationPayload::Function { code, .. } = joined.payload else {
            unreachable!()
        };
        assert_eq!(
            code.instructions
                .iter()
                .filter(|instruction| matches!(
                    instruction,
                    CompiledInstruction::LoadLocal {
                        local: 0,
                        use_mode: ParameterUse::Borrow
                    }
                ))
                .count(),
            2
        );
        assert!(code.instructions.iter().any(|instruction| matches!(
            instruction,
            CompiledInstruction::Parallel {
                left_arguments: 1,
                right_arguments: 1,
                ..
            }
        )));
    }
}

#[test]
fn borrowed_task_artifact_rejects_rehashed_bound_mode_source_and_witness_forgery() {
    let (source, loaded) = fixture(SOURCE);
    for attack in [
        "erase-share",
        "add-transfer",
        "consume-mode",
        "read-source",
        "witness",
    ] {
        let name = match attack {
            "read-source" => "joined",
            "witness" => "method-reader",
            _ => "read",
        };
        let (key, mut changed) = unit(&source, &loaded, name);
        let CompilationPayload::Function { signature, code } = &mut changed.payload else {
            unreachable!()
        };
        match attack {
            "erase-share" => {
                signature.type_parameter_constraints[0] = TypeParameterConstraints::Owned
            }
            "add-transfer" => {
                signature.type_parameter_constraints[0] =
                    TypeParameterConstraints::OwnedTransferableShareable
            }
            "consume-mode" => signature.parameters[0].use_mode = ParameterUse::Consume,
            "read-source" => {
                let CompiledInstruction::LoadLocal { local, .. } = code
                    .instructions
                    .iter_mut()
                    .find(|instruction| {
                        matches!(instruction, CompiledInstruction::LoadLocal { .. })
                    })
                    .unwrap()
                else {
                    unreachable!()
                };
                *local = 1;
            }
            "witness" => {
                let CompiledInstruction::MethodCall { witness, .. } = code
                    .instructions
                    .iter_mut()
                    .find(|instruction| {
                        matches!(instruction, CompiledInstruction::MethodCall { .. })
                    })
                    .unwrap()
                else {
                    unreachable!()
                };
                let crate::platform::kernel::ImplementationOperand::Concrete {
                    type_arguments, ..
                } = witness
                else {
                    unreachable!()
                };
                type_arguments.clear();
            }
            _ => unreachable!(),
        }
        let bytes = match changed.validate() {
            Ok(()) => effect_tests::replace_unit(&loaded, key, &changed, vec![]),
            Err(error) => effect_tests::replace_rejected_unit(&loaded, key, &changed, &error.code),
        };
        let error = load_artifact(&bytes)
            .expect_err("repaired hashes cannot change borrowed task contracts");
        assert!(
            matches!(
                error.class,
                crate::platform::DiagnosticClass::Corrupt
                    | crate::platform::DiagnosticClass::Semantic
            ),
            "{attack}: {error:?}"
        );
    }
}

#[test]
fn borrowed_task_artifact_requires_the_current_scoped_child_envelope() {
    let (source, loaded) = fixture(SOURCE);
    let (key, original) = unit(&source, &loaded, "joined");
    let current = effect_tests::replace_unit(&loaded, key, &original, vec![]);
    for generation in 30_u16..37 {
        let mut previous = current.clone();
        previous[..8].copy_from_slice(format!("LKJART{generation}").as_bytes());
        previous[8..10].copy_from_slice(&generation.to_be_bytes());
        let error = load_artifact(&previous)
            .expect_err("Graph 30 cannot lend current child rights through an older envelope");
        assert_eq!(
            error.code, "artifact_bundle_contract",
            "{generation}: {error:?}"
        );
        assert_eq!(error.class, crate::platform::DiagnosticClass::Source);
    }
}
