//! Recursive witness meaning and unused mappings survive independently rehashed boundaries.
use super::*;
use crate::platform::kernel::{DeclarationReference, ImplementationOperand, KernelSnapshot};

pub(crate) const SOURCE: &str = r#"declarations.begin
(units (module create prerequisite-readers
  (external create cell-read (visibility private) (implementation core.cell.read)
    (parameter create cell (type OwnedI64Cell) (use borrow)) (returns I64))
  (owned-contract create Reader (visibility private)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_ba000000000000000000000000000001 first
      (parameters (Self borrow) (Self borrow)) (returns Self (borrow-from 0))))
  (owned-contract create OtherReader (visibility private)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_ba000000000000000000000000000002 other
      (parameters (Self borrow) (Self borrow)) (returns Self (borrow-from 1))))
  (function create first (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create a (type T) (use borrow))
    (parameter create b (type T) (use borrow))
    (returns T (borrow-from a)) (body (local a)))
  (function create other (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create a (type T) (use borrow))
    (parameter create b (type T) (use borrow))
    (returns T (borrow-from b)) (body (local b)))
  (function create delegate (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_ba000000000000000000000000000001 reader Reader T)
    (implementation-parameter implparam_ba000000000000000000000000000002 other OtherReader T)
    (parameter create a (type T) (use borrow))
    (parameter create b (type T) (use borrow))
    (returns T (borrow-from a))
    (body (borrow-call
      (method-call parameter@delegate@implparam_ba000000000000000000000000000001
        Reader method_ba000000000000000000000000000001 (local a) (local b))
      (binding selected (type T)) (in (local selected)))))
  (owned-implementation create CellReader (visibility private)
    (contract Reader) (self OwnedI64Cell)
    (method method_ba000000000000000000000000000001 first (types OwnedI64Cell)))
  (owned-implementation create CellOther (visibility private)
    (contract OtherReader) (self OwnedI64Cell)
    (method method_ba000000000000000000000000000002 other (types OwnedI64Cell)))
  (owned-implementation create Delegating (visibility private)
    (type-parameter create T (constraint owned))
    (type-parameter create Phantom (constraint owned))
    (implementation-parameter implparam_ba000000000000000000000000000003 reader Reader T)
    (implementation-parameter implparam_ba000000000000000000000000000004 other OtherReader T)
    (contract Reader) (self T)
    (method method_ba000000000000000000000000000001 delegate (types T)
      (implementations
        parameter@Delegating@implparam_ba000000000000000000000000000003
        parameter@Delegating@implparam_ba000000000000000000000000000004)))
  (owned-implementation create UnusedNested (visibility private)
    (type-parameter create T (constraint owned))
    (type-parameter create Phantom (constraint owned))
    (implementation-parameter implparam_ba000000000000000000000000000005 reader Reader T)
    (implementation-parameter implparam_ba000000000000000000000000000006 other OtherReader T)
    (contract Reader) (self OwnedI64Cell)
    (method method_ba000000000000000000000000000001 delegate (types OwnedI64Cell)
      (implementations
        (implementation Delegating (types OwnedI64Cell Phantom)
          (implementations CellReader CellOther))
        CellOther)))
  (function create inspect (visibility public) (effect pure)
    (parameter create a (type OwnedI64Cell) (use borrow))
    (parameter create b (type OwnedI64Cell) (use borrow)) (returns I64)
    (body (if (bool true) (i64 0)
      (borrow-call
        (method-call
          (implementation Delegating (types OwnedI64Cell ByteBuffer)
            (implementations
              (implementation Delegating (types OwnedI64Cell OwnedI64Cell)
                (implementations CellReader CellOther))
              CellOther))
          Reader method_ba000000000000000000000000000001 (local a) (local b))
        (binding selected (type OwnedI64Cell))
        (in (call cell-read (local selected)))))))))
declarations.end"#;

fn fixture() -> (KernelSnapshot, LoadedArtifact) {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .expect("prerequisite witnesses author through the public surface");
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let directory = tempfile::tempdir().unwrap();
    let repository =
        GraphRepository::create(&directory.path().join("prerequisites"), &source, None)
            .unwrap()
            .repository;
    let compiled = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let linked = link_artifact(&repository, compiled.manifest_digest, &[]).unwrap();
    (source, load_artifact(&linked.artifact.bytes).unwrap())
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
fn prerequisite_artifact_retains_order_scope_and_every_nested_phantom_type() {
    let (source, loaded) = fixture();
    let (key, implementation) = unit(&source, &loaded, "Delegating");
    let CompilationPayload::OwnedImplementation(i) = &implementation.payload else {
        unreachable!()
    };
    assert_eq!(i.implementation_parameters.len(), 2);
    assert_eq!(i.methods[0].implementations.len(), 2);
    for operand in &i.methods[0].implementations {
        let ImplementationOperand::Parameter { scope, .. } = operand else {
            unreachable!()
        };
        assert_eq!(
            *scope,
            DeclarationReference {
                package: source.root.package_id,
                declaration: declaration_named(&source, "Delegating")
            }
        );
    }
    load_artifact(&effect_tests::replace_unit(
        &loaded,
        key,
        &implementation,
        vec![],
    ))
    .unwrap();
    let (_, inspect) = unit(&source, &loaded, "inspect");
    let CompilationPayload::Function { code, .. } = &inspect.payload else {
        unreachable!()
    };
    let witness = code
        .instructions
        .iter()
        .find_map(|instruction| match instruction {
            CompiledInstruction::MethodCall { witness, .. } => Some(witness),
            _ => None,
        })
        .unwrap();
    assert_eq!(witness.walk().count(), 5);
    let buffer = source
        .types
        .iter()
        .find_map(|(ty, object)| matches!(object.form, TypeForm::ByteBuffer).then_some(*ty))
        .unwrap();
    assert!(inspect.tables.types.contains(&buffer));
}

#[test]
fn prerequisite_artifact_independently_rejects_coherent_unused_mapping_faults() {
    let (source, loaded) = fixture();
    let (key, original) = unit(&source, &loaded, "UnusedNested");
    for attack in ["missing", "reordered", "wrong-self", "borrow-source"] {
        let mut invalid = source.clone();
        let mut changed = original.clone();
        let CompilationPayload::OwnedImplementation(i) = &mut changed.payload else {
            unreachable!()
        };
        match attack {
            "missing" => {
                i.methods[0].implementations.pop();
            }
            "reordered" => i.methods[0].implementations.swap(0, 1),
            "wrong-self" => {
                i.implementation_parameters[0].self_type = *source
                    .types
                    .iter()
                    .find_map(|(ty, object)| {
                        matches!(object.form, TypeForm::OwnedI64Cell).then_some(ty)
                    })
                    .unwrap();
                if !changed
                    .tables
                    .types
                    .contains(&i.implementation_parameters[0].self_type)
                {
                    changed
                        .tables
                        .types
                        .push(i.implementation_parameters[0].self_type);
                }
            }
            "borrow-source" => {
                i.methods[0].function = DeclarationReference {
                    package: source.root.package_id,
                    declaration: declaration_named(&source, "other"),
                };
                i.methods[0].implementations.clear();
                changed.tables.declarations.push(i.methods[0].function);
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
        assert!(
            !crate::platform::kernel::memory_reference::accepts(&invalid),
            "memory oracle accepted {attack}"
        );
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
            .find(|b| b.owner == changed.source.owner)
            .unwrap();
        let old = ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
        let (digest, bytes) = encode_owner(&OwnerRecord::Declaration(canonical)).unwrap();
        binding.object = digest;
        forged.objects.remove(&old).unwrap();
        forged.objects.insert(
            ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
            bytes,
        );
        let error =
            load_artifact(&effect_tests::replace_unit(&forged, key, &changed, vec![])).unwrap_err();
        assert_eq!(error.code, "artifact_affine_meaning", "{attack}: {error:?}");
        assert!(
            error.message.contains("kernel_owned_contract"),
            "{attack}: {error:?}"
        );
    }
}

#[test]
fn prerequisite_artifact_rejects_nested_scope_identity_and_order_changes() {
    let (source, loaded) = fixture();
    let (key, original) = unit(&source, &loaded, "inspect");
    for attack in ["arity", "order", "identity", "phantom"] {
        let mut changed = original.clone();
        let CompilationPayload::Function { code, .. } = &mut changed.payload else {
            unreachable!()
        };
        let operand = code
            .instructions
            .iter_mut()
            .find_map(|instruction| match instruction {
                CompiledInstruction::MethodCall { witness, .. } => Some(witness),
                _ => None,
            })
            .unwrap();
        let ImplementationOperand::Concrete {
            implementations,
            type_arguments,
            ..
        } = operand
        else {
            unreachable!()
        };
        match attack {
            "arity" => {
                implementations.pop();
            }
            "order" => implementations.swap(0, 1),
            "identity" => {
                let ImplementationOperand::Concrete { implementation, .. } =
                    &mut implementations[0]
                else {
                    unreachable!()
                };
                *implementation = DeclarationReference {
                    package: source.root.package_id,
                    declaration: declaration_named(&source, "CellReader"),
                };
                if !changed.tables.declarations.contains(implementation) {
                    changed.tables.declarations.push(*implementation);
                }
            }
            "phantom" => type_arguments.swap(0, 1),
            _ => unreachable!(),
        }
        let error =
            load_artifact(&effect_tests::replace_unit(&loaded, key, &changed, vec![])).unwrap_err();
        assert!(!error.code.is_empty(), "{attack}");
    }
    let (key, original) = unit(&source, &loaded, "Delegating");
    let mut changed = original.clone();
    let CompilationPayload::OwnedImplementation(i) = &mut changed.payload else {
        unreachable!()
    };
    let ImplementationOperand::Parameter { scope, .. } = &mut i.methods[0].implementations[0]
    else {
        unreachable!()
    };
    *scope = DeclarationReference {
        package: source.root.package_id,
        declaration: declaration_named(&source, "delegate"),
    };
    assert_eq!(
        changed.validate().unwrap_err().code,
        "compiler_unit_witness_scope"
    );
    let error = changed.validate().unwrap_err();
    load_artifact(&effect_tests::replace_rejected_unit(
        &loaded,
        key,
        &changed,
        &error.code,
    ))
    .unwrap_err();
}

#[test]
fn prerequisite_compiler_unit_rejects_every_hidden_mapping_root_omission() {
    let (source, loaded) = fixture();
    let (_, original) = unit(&source, &loaded, "UnusedNested");
    let CompilationPayload::OwnedImplementation(i) = &original.payload else {
        unreachable!()
    };
    let hidden_type = i.methods[0].implementations[0].type_arguments()[1];
    assert!(original.tables.types.contains(&hidden_type));
    let ImplementationOperand::Concrete {
        implementation: hidden_declaration,
        ..
    } = &i.methods[0].implementations[0]
    else {
        unreachable!()
    };
    assert!(original.tables.declarations.contains(hidden_declaration));
    let mut no_phantom = original.clone();
    no_phantom.tables.types.retain(|ty| *ty != hidden_type);
    assert_eq!(
        no_phantom.validate().unwrap_err().code,
        "compiler_unit_owned_implementation_type"
    );
    let mut no_concrete = original.clone();
    no_concrete
        .tables
        .declarations
        .retain(|reference| reference != hidden_declaration);
    assert_eq!(
        no_concrete.validate().unwrap_err().code,
        "compiler_unit_witness_declaration"
    );
    let mut no_contract = original.clone();
    no_contract
        .tables
        .declarations
        .retain(|reference| *reference != i.implementation_parameters[1].contract);
    assert_eq!(
        no_contract.validate().unwrap_err().code,
        "compiler_unit_owned_implementation_declaration"
    );
    let (_, mut no_scope) = unit(&source, &loaded, "Delegating");
    no_scope.tables.declarations.retain(|reference| {
        *reference
            != DeclarationReference {
                package: source.root.package_id,
                declaration: declaration_named(&source, "Delegating"),
            }
    });
    assert_eq!(
        no_scope.validate().unwrap_err().code,
        "compiler_unit_witness_declaration"
    );
}
