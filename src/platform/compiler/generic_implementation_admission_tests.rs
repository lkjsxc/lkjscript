//! Complete generic implementation meaning survives lowering and independently rehashed admission.
use super::*;
use crate::platform::kernel::{DeclarationReference, ImplementationOperand, KernelSnapshot};

const SOURCE: &str = r#"declarations.begin
(units (module create generic-implementations
  (external create cell-read (visibility private) (implementation core.cell.read)
    (parameter create cell (type OwnedI64Cell) (use borrow)) (returns I64))
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Item (constraint owned))
    (method method_33000000000000000000000000000001 first
      (parameters (Self borrow) (Self borrow)) (returns Item (borrow-from 0))))
  (function create project (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (type-parameter create A (constraint owned))
    (type-parameter create B (constraint owned))
    (parameter create packet (type (owned-product (field payload T))) (use borrow))
    (parameter create other (type (owned-product (field payload T))) (use borrow))
    (returns T (borrow-from packet))
    (body (borrow-owned-field (type (owned-product (field payload T))) (local packet)
      (field payload (binding view (type T))) (in (local view)))))
  (function create project-other (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (type-parameter create A (constraint owned))
    (type-parameter create B (constraint owned))
    (parameter create packet (type (owned-product (field payload T))) (use borrow))
    (parameter create other (type (owned-product (field payload T))) (use borrow))
    (returns T (borrow-from other))
    (body (borrow-owned-field (type (owned-product (field payload T))) (local other)
      (field payload (binding view (type T))) (in (local view)))))
  (owned-implementation create ProductReader (visibility public)
    (type-parameter create T (constraint owned))
    (type-parameter create A (constraint owned))
    (type-parameter create B (constraint owned))
    (contract Reader) (self (owned-product (field payload T))) (types T)
    (method method_33000000000000000000000000000001 project (types T A B)))
  (owned-implementation create AlternateReader (visibility public)
    (type-parameter create T (constraint owned))
    (type-parameter create A (constraint owned))
    (type-parameter create B (constraint owned))
    (contract Reader) (self (owned-product (field payload T))) (types T)
    (method method_33000000000000000000000000000001 project (types T B A)))
  (owned-implementation create HiddenReader (visibility private)
    (type-parameter create T (constraint owned))
    (type-parameter create A (constraint owned))
    (type-parameter create B (constraint owned))
    (contract Reader) (self (owned-product (field payload T))) (types T)
    (method method_33000000000000000000000000000001 project (types T A B)))
  (function create generic-first (visibility public) (effect pure)
    (type-parameter create Storage (constraint owned))
    (type-parameter create Item (constraint owned))
    (implementation-parameter implparam_33000000000000000000000000000001 reader Reader Storage (types Item))
    (parameter create packet (type Storage) (use borrow))
    (parameter create other (type Storage) (use borrow))
    (returns Item (borrow-from packet))
    (body (borrow-call
      (method-call parameter@generic-first@implparam_33000000000000000000000000000001 Reader
        method_33000000000000000000000000000001 (local packet) (local other))
      (binding view (type Item)) (in (local view)))))
  (function create inspect (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field payload OwnedI64Cell))) (use borrow))
    (parameter create other (type (owned-product (field payload OwnedI64Cell))) (use borrow))
    (returns I64)
    (body (if (bool true) (i64 7)
      (sequence
        (borrow-call
          (method-call (implementation ProductReader (types OwnedI64Cell ByteBuffer OwnedI64Cell)) Reader
            method_33000000000000000000000000000001 (local packet) (local other))
          (binding first (type OwnedI64Cell)) (in (call cell-read (local first))))
        (borrow-call
          (implementation-call generic-first
            (types (owned-product (field payload OwnedI64Cell)) OwnedI64Cell)
            (implementations (implementation ProductReader (types OwnedI64Cell OwnedI64Cell ByteBuffer)))
            (local packet) (local other))
          (binding second (type OwnedI64Cell)) (in (call cell-read (local second))))))))))
declarations.end"#;

fn fixture() -> (KernelSnapshot, LoadedArtifact) {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .expect("generic implementations authored through the native public surface");
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let directory = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&directory.path().join("generic"), &source, None)
        .unwrap()
        .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let linked = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    (
        source,
        load_artifact(&linked.artifact.bytes).expect("complete generic artifact admits"),
    )
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

fn reject(loaded: &LoadedArtifact, key: ObjectKey, unit: &CompilationUnit, attack: &str) {
    let bytes = match unit.validate() {
        Ok(()) => effect_tests::replace_unit(loaded, key, unit, vec![]),
        Err(error) => effect_tests::replace_rejected_unit(loaded, key, unit, &error.code),
    };
    let error = load_artifact(&bytes)
        .expect_err("repaired hashes do not authorize changed generic meaning");
    println!("generic implementation {attack}: {}", error.code);
}

#[test]
fn generic_implementation_artifact_retains_all_scheme_and_mapped_argument_roots() {
    let (source, loaded) = fixture();
    let (key, original) = unit(&source, &loaded, "ProductReader");
    load_artifact(&effect_tests::replace_unit(&loaded, key, &original, vec![])).unwrap();
    let CompilationPayload::OwnedImplementation(implementation) = &original.payload else {
        unreachable!()
    };
    assert_eq!(implementation.type_parameters.len(), 3);
    assert_eq!(implementation.methods[0].type_arguments.len(), 3);
    assert!(
        implementation.methods[0]
            .type_arguments
            .iter()
            .all(|ty| original.tables.types.contains(ty))
    );
    for parameter in &implementation.type_parameters {
        assert!(
            loaded.manifest.packages[0]
                .runtime_owners
                .iter()
                .any(
                    |binding| binding.owner == OwnerKey::TypeParameter(*parameter)
                        && binding.kind == OwnerKind::TypeParameter
                )
        );
    }
    assert!(loaded.work.callable_analysis_steps > 0);
    for attack in [
        "parameter-order",
        "parameter-arity",
        "mapped-order",
        "mapped-arity",
        "mapped-type-root",
    ] {
        let mut changed = original.clone();
        let CompilationPayload::OwnedImplementation(i) = &mut changed.payload else {
            unreachable!()
        };
        match attack {
            "parameter-order" => i.type_parameters.swap(1, 2),
            "parameter-arity" => {
                i.type_parameters.pop();
            }
            "mapped-order" => i.methods[0].type_arguments.swap(1, 2),
            "mapped-arity" => {
                i.methods[0].type_arguments.pop();
            }
            "mapped-type-root" => {
                let unused = i.methods[0].type_arguments[2];
                changed.tables.types.retain(|ty| *ty != unused);
            }
            _ => unreachable!(),
        }
        reject(&loaded, key, &changed, attack);
    }
}

#[test]
fn generic_implementation_artifact_rejects_rebound_unused_witness_arguments_in_untaken_code() {
    let (source, loaded) = fixture();
    let (key, original) = unit(&source, &loaded, "inspect");
    load_artifact(&effect_tests::replace_unit(&loaded, key, &original, vec![])).unwrap();
    for instruction_kind in ["method", "forwarded-call"] {
        for attack in ["order", "arity", "declaration", "type-root"] {
            let mut changed = original.clone();
            let CompilationPayload::Function { code, .. } = &mut changed.payload else {
                unreachable!()
            };
            let operand = code
                .instructions
                .iter_mut()
                .find_map(|instruction| match (instruction_kind, instruction) {
                    ("method", CompiledInstruction::MethodCall { witness, .. }) => Some(witness),
                    (
                        "forwarded-call",
                        CompiledInstruction::ImplementationCall {
                            implementations, ..
                        },
                    ) => implementations.first_mut(),
                    _ => None,
                })
                .unwrap();
            let ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                ..
            } = operand
            else {
                unreachable!()
            };
            match attack {
                "order" => type_arguments.swap(1, 2),
                "arity" => {
                    type_arguments.pop();
                }
                "declaration" => {
                    *implementation = DeclarationReference {
                        package: source.root.package_id,
                        declaration: declaration_named(&source, "AlternateReader"),
                    };
                    if !changed.tables.declarations.contains(implementation) {
                        changed.tables.declarations.push(*implementation);
                    }
                }
                "type-root" => {
                    let buffer = type_arguments
                        .iter()
                        .copied()
                        .find(|ty| {
                            source
                                .types
                                .get(ty)
                                .is_some_and(|t| matches!(t.form, TypeForm::ByteBuffer))
                        })
                        .unwrap();
                    // The other application still uses this type. Unit admission must reject
                    // every erased witness root, regardless of global artifact availability.
                    changed.tables.types.retain(|ty| *ty != buffer);
                }
                _ => unreachable!(),
            }
            reject(
                &loaded,
                key,
                &changed,
                &format!("{instruction_kind}-{attack}"),
            );
        }
    }
}

#[test]
fn generic_implementation_artifact_rejects_changed_mapped_target_and_borrow_source() {
    let (source, loaded) = fixture();
    let (key, original) = unit(&source, &loaded, "ProductReader");
    let mut changed = original.clone();
    let CompilationPayload::OwnedImplementation(i) = &mut changed.payload else {
        unreachable!()
    };
    i.methods[0].function = DeclarationReference {
        package: source.root.package_id,
        declaration: declaration_named(&source, "project-other"),
    };
    changed.tables.declarations.push(i.methods[0].function);
    reject(&loaded, key, &changed, "mapped-target-borrow-source");

    let (key, original) = unit(&source, &loaded, "project");
    let mut changed = original.clone();
    let CompilationPayload::Function { signature, .. } = &mut changed.payload else {
        unreachable!()
    };
    signature.result_borrow = Some(signature.parameters[1].parameter);
    reject(&loaded, key, &changed, "generic-target-provenance");
}

#[test]
fn generic_implementation_artifact_independently_rejects_coherent_unused_invalid_mappings() {
    let (source, loaded) = fixture();
    let (key, original) = unit(&source, &loaded, "HiddenReader");
    for attack in ["borrow-source", "mapped-owned-constraint", "mapped-arity"] {
        let mut invalid = source.clone();
        let mut changed = original.clone();
        let CompilationPayload::OwnedImplementation(i) = &mut changed.payload else {
            unreachable!()
        };
        match attack {
            "borrow-source" => {
                i.methods[0].function = DeclarationReference {
                    package: source.root.package_id,
                    declaration: declaration_named(&source, "project-other"),
                };
                changed.tables.declarations.push(i.methods[0].function);
            }
            "mapped-owned-constraint" => {
                let i64_type = source
                    .types
                    .iter()
                    .find_map(|(ty, object)| matches!(object.form, TypeForm::I64).then_some(*ty))
                    .unwrap();
                i.methods[0].type_arguments[0] = i64_type;
                changed.tables.types.push(i64_type);
            }
            "mapped-arity" => {
                i.methods[0].type_arguments.pop();
            }
            _ => unreachable!(),
        }
        let OwnerRecord::Declaration(declaration) =
            invalid.owners.get_mut(&changed.source.owner).unwrap()
        else {
            unreachable!()
        };
        declaration.payload =
            crate::platform::kernel::DeclarationPayload::OwnedImplementation(i.clone());
        let canonical = declaration.clone();
        // Keep the producer's source summary and unit key coherent with its changed source too.
        // Complete independent admission must reject an unused mapping despite this agreement.
        let facts = crate::platform::witness::rebuild_canonical_facts(&invalid).unwrap();
        let summary = &facts.summaries[&changed.source.owner];
        changed.source = CompilationSource {
            package: invalid.root.package_id,
            owner: changed.source.owner,
            kind: summary.kind,
            semantic_interface: summary.semantic_interface,
            implementation: summary.implementation,
            type_digest: summary.type_digest,
            effect: summary.effect,
            capability: summary.capability,
            test: summary.test,
            validation_dependencies: summary.validation_dependencies,
        };
        changed.key = CompilationUnitKey::derive(&changed.source, changed.optimization).unwrap();
        let mut forged = loaded.clone();
        let binding = forged.manifest.packages[0]
            .runtime_owners
            .iter_mut()
            .find(|binding| binding.owner == changed.source.owner)
            .unwrap();
        let old_owner = ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
        let (digest, bytes) = encode_owner(&OwnerRecord::Declaration(canonical)).unwrap();
        binding.object = digest;
        forged.objects.remove(&old_owner).unwrap();
        forged.objects.insert(
            ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
            bytes,
        );
        let bytes = effect_tests::replace_unit(&forged, key, &changed, vec![]);
        let error = load_artifact(&bytes)
            .expect_err("coherent producer cannot hide an invalid unused generic mapping");
        assert_eq!(error.code, "artifact_affine_meaning", "{attack}: {error:?}");
        assert!(
            error.message.contains("kernel_owned_contract"),
            "{attack}: {error:?}"
        );
    }
}
