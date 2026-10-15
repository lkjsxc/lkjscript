//! Producer hashes cannot authorize phantom storage categories or predecessor declarations.
use super::*;
use crate::platform::kernel::{
    DeclarationPayload, ImplementationOperand, KernelSnapshot, TypeParameterConstraints,
};

const SOURCE: &str = r#"declarations.begin
(units (module create mixed-storage-phantom
  (owned-contract create Marker (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Item (constraint transferable))
    (type-parameter create Phantom (constraint capture-safe transferable))
    (method method_37000000000000000000000000000001 seven (parameters) (returns I64)))
  (function create seven (visibility public) (returns I64) (effect pure) (body (i64 7)))
  (owned-implementation create CellMarker (visibility public)
    (type-parameter create T (constraint transferable))
    (type-parameter create U (constraint capture-safe transferable))
    (contract Marker) (self OwnedI64Cell) (types T U)
    (method method_37000000000000000000000000000001 seven))
  (function create inspect (visibility public) (returns I64) (effect pure)
    (body (if (bool true) (i64 7)
      (method-call (implementation CellMarker (types I64 Bool)) Marker
        method_37000000000000000000000000000001))))))
declarations.end"#;

fn fixture() -> (KernelSnapshot, LoadedArtifact) {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .expect("mixed phantom contract and scheme author through the public surface");
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let directory = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&directory.path().join("mixed"), &source, None)
        .unwrap()
        .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let linked = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    (
        source,
        load_artifact(&linked.artifact.bytes).expect("complete mixed artifact admits"),
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

#[test]
fn mixed_storage_artifact_retains_exact_phantom_contract_and_scheme_bounds() {
    let (source, loaded) = fixture();
    for declaration in ["Marker", "CellMarker"] {
        let (_, unit) = unit(&source, &loaded, declaration);
        let parameters = match &unit.payload {
            CompilationPayload::OwnedContract(contract) => &contract.type_parameters,
            CompilationPayload::OwnedImplementation(implementation) => {
                &implementation.type_parameters
            }
            _ => unreachable!(),
        };
        for (parameter, constraint) in parameters.iter().zip([
            TypeParameterConstraints::Transferable,
            TypeParameterConstraints::CaptureSafeTransferable,
        ]) {
            let owner = OwnerKey::TypeParameter(*parameter);
            let binding = loaded.manifest.packages[0]
                .runtime_owners
                .iter()
                .find(|binding| binding.owner == owner)
                .unwrap();
            let record = decode_owner(
                &loaded.objects
                    [&ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes())],
                owner,
                binding.kind,
                binding.object,
            )
            .unwrap();
            let OwnerRecord::TypeParameter(record) = record else {
                unreachable!()
            };
            assert_eq!(record.constraints, constraint);
            assert_eq!(
                record.header.contract_version,
                source.owners[&owner].header().contract_version,
                "runtime metadata preserves the actual canonical owner generation"
            );
        }
    }
}

#[test]
fn mixed_storage_artifact_rejects_rehashed_untaken_owned_phantom_actuals() {
    let (source, loaded) = fixture();
    let (key, original) = unit(&source, &loaded, "inspect");
    load_artifact(&effect_tests::replace_unit(&loaded, key, &original, vec![])).unwrap();
    let cell = encode_type_object(&TypeObject::new(TypeForm::OwnedI64Cell).unwrap())
        .unwrap()
        .0;
    for position in 0..2 {
        let mut changed = original.clone();
        let CompilationPayload::Function { code, .. } = &mut changed.payload else {
            unreachable!()
        };
        let witness = code
            .instructions
            .iter_mut()
            .find_map(|instruction| match instruction {
                CompiledInstruction::MethodCall { witness, .. } => Some(witness),
                _ => None,
            })
            .unwrap();
        let ImplementationOperand::Concrete { type_arguments, .. } = witness else {
            unreachable!()
        };
        type_arguments[position] = cell;
        if !changed.tables.types.contains(&cell) {
            changed.tables.types.push(cell);
        }
        let error = load_artifact(&effect_tests::replace_unit(&loaded, key, &changed, vec![]))
            .expect_err("complete admission rejects the untaken phantom owner");
        assert!(
            matches!(
                error.class,
                crate::platform::DiagnosticClass::Corrupt
                    | crate::platform::DiagnosticClass::Semantic
            ),
            "{error:?}"
        );
    }
}

#[test]
fn mixed_storage_phantom_parameter_owner_requires_current_contract() {
    let (source, _) = fixture();
    for declaration in ["Marker", "CellMarker"] {
        let owner = OwnerKey::Declaration(declaration_named(&source, declaration));
        let OwnerRecord::Declaration(record) = &source.owners[&owner] else {
            unreachable!()
        };
        let parameters = match &record.payload {
            DeclarationPayload::OwnedContract(contract) => &contract.type_parameters,
            DeclarationPayload::OwnedImplementation(implementation) => {
                &implementation.type_parameters
            }
            _ => unreachable!(),
        };
        for parameter in parameters {
            let mut record = source.owners[&OwnerKey::TypeParameter(*parameter)].clone();
            record.set_encoding_for_edit(
                crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION ^ 1,
            );
            assert_eq!(
                encode_owner(&record).unwrap_err().code,
                "kernel_owner_header"
            );
        }
    }
}
