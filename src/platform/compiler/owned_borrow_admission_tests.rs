//! Rehashed artifacts must retain exact child loans and their lexical custody.
use super::*;
use crate::platform::kernel::{
    DeclarationPayload, EncodedOwnerKey, ExpressionValidationLimits, KernelSnapshot,
    OwnedProductBinding, ParameterUse, decode_owner_binding, encode_owner_binding,
    validate_affine_roots_with_limits, validate_expression_roots_with_limits,
};

// The new operations live in untaken syntax deliberately: artifact admission must
// prove the complete meaning, independently of the selected runtime branch.
const SOURCE: &str = r#"
declarations.begin
(units (module create borrowed-artifact
  (external create length (visibility private) (implementation core.buffer.length)
    (parameter create value (type ByteBuffer) (use borrow)) (returns I64))
  (function create read-product (visibility private) (effect pure)
    (parameter create packet
      (type (owned-product (field payload ByteBuffer) (field sibling ByteBuffer) (field tag I64))) (use borrow))
    (parameter create other
      (type (owned-product (field payload ByteBuffer) (field sibling ByteBuffer) (field tag I64))) (use borrow))
    (returns I64)
    (body (if (bool true) (i64 7)
      (borrow-owned-field
        (type (owned-product (field payload ByteBuffer) (field sibling ByteBuffer) (field tag I64)))
        (local packet) (field payload (binding view (type ByteBuffer)))
        (in (call length (local view)))))))
  (function create read-choice (visibility private) (effect pure)
    (parameter create outcome (type (owned-choice (case accepted I64) (case rejected ByteBuffer))) (use borrow))
    (parameter create other (type (owned-choice (case accepted I64) (case rejected ByteBuffer))) (use borrow))
    (returns I64)
    (body (if (bool true) (i64 7)
      (match-borrowed-owned (type (owned-choice (case accepted I64) (case rejected ByteBuffer))) (local outcome)
        (case accepted (binding value (type I64)) (in (local value)))
        (case rejected (binding view (type ByteBuffer)) (in (call length (local view))))))))
  (function create read-nested (visibility private) (effect pure)
    (parameter create envelope
      (type (owned-product (field packet (owned-product (field payload ByteBuffer))))) (use borrow))
    (returns I64)
    (body (borrow-owned-field
      (type (owned-product (field packet (owned-product (field payload ByteBuffer))))) (local envelope)
      (field packet (binding packet (type (owned-product (field payload ByteBuffer)))))
      (in (borrow-owned-field (type (owned-product (field payload ByteBuffer))) (local packet)
        (field payload (binding view (type ByteBuffer))) (in (call length (local view))))))))
  (function create read-owner-role (visibility private) (effect pure)
    (parameter create packet (type (owned-product (field payload ByteBuffer))) (use borrow))
    (returns I64)
    (body (if (bool true) (i64 7)
      (borrow-owned-field (type (owned-product (field payload ByteBuffer))) (local packet)
        (field payload (binding view (type ByteBuffer))) (in (call length (local view)))))))))
declarations.end
"#;

fn fixture() -> (KernelSnapshot, LoadedArtifact) {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .expect("new scopes authored through the native change surface");
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let dir = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&dir.path().join("borrowed"), &source, None)
        .unwrap()
        .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let artifact = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    let loaded = load_artifact(&artifact.artifact.bytes).expect("accepted native control loads");
    (source, loaded)
}

fn function_unit(
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
        .expect("compiled fixture function")
}

fn code(unit: &CompilationUnit) -> &CompiledCode {
    let CompilationPayload::Function { code, .. } = &unit.payload else {
        panic!("fixture function");
    };
    code
}

fn code_mut(unit: &mut CompilationUnit) -> &mut CompiledCode {
    let CompilationPayload::Function { code, .. } = &mut unit.payload else {
        panic!("fixture function");
    };
    code
}

fn reject_changed_unit(
    loaded: &LoadedArtifact,
    key: ObjectKey,
    unit: &CompilationUnit,
    attack: &str,
) {
    // Admit structurally meaningful forgeries normally; intentionally malformed
    // scope protocols also travel through the complete digest-repaired envelope.
    let bytes = match unit.validate() {
        Ok(()) => effect_tests::replace_unit(loaded, key, unit, vec![]),
        Err(error) => effect_tests::replace_rejected_unit(loaded, key, unit, &error.code),
    };
    let error = load_artifact(&bytes)
        .expect_err("matching hashes cannot replace checked borrowing meaning");
    println!("owned read {attack}: {}", error.code);
}

#[test]
fn owned_field_artifact_rejects_rehashed_source_field_type_and_cleanup_forgery() {
    let (source, loaded) = fixture();
    let (key, original) = function_unit(&source, &loaded, "read-product");
    load_artifact(&effect_tests::replace_unit(&loaded, key, &original, vec![]))
        .expect("neutral rehash preserves borrowing admission");
    let begin = code(&original)
        .instructions
        .iter()
        .position(|i| matches!(i, CompiledInstruction::BorrowOwnedField { .. }))
        .unwrap();
    let end = code(&original)
        .instructions
        .iter()
        .position(|i| matches!(i, CompiledInstruction::EndOwnedBorrow { .. }))
        .unwrap();
    assert!(matches!(
        code(&original).instructions[begin],
        CompiledInstruction::BorrowOwnedField {
            source_local: 0,
            field: 0,
            ..
        }
    ));
    for attack in [
        "source",
        "field",
        "type",
        "binding",
        "erase-begin",
        "erase-end",
        "mispair-end",
    ] {
        let mut changed = original.clone();
        let instructions = &mut code_mut(&mut changed).instructions;
        match attack {
            "erase-begin" => instructions[begin] = CompiledInstruction::Jump(begin as u32 + 1),
            "erase-end" => instructions[end] = CompiledInstruction::Jump(end as u32 + 1),
            "mispair-end" => {
                let CompiledInstruction::BorrowOwnedField { source_local, .. } =
                    instructions[begin]
                else {
                    unreachable!()
                };
                instructions[end] = CompiledInstruction::EndOwnedBorrow {
                    binding_local: source_local,
                };
            }
            _ => {
                let CompiledInstruction::BorrowOwnedField {
                    product_type,
                    source_local,
                    field,
                    binding_local,
                    binding_type,
                } = &mut instructions[begin]
                else {
                    unreachable!()
                };
                match attack {
                    // The other source has the exact same type; the sibling has
                    // the exact same payload type. Both still change meaning.
                    "source" => *source_local = 1,
                    "field" => *field = 1,
                    "type" => *binding_type = *product_type,
                    "binding" => *binding_local = *source_local,
                    _ => unreachable!(),
                }
            }
        }
        reject_changed_unit(&loaded, key, &changed, attack);
    }
}

#[test]
fn borrowed_choice_artifact_rejects_rehashed_targets_payload_types_and_loan_ends() {
    let (source, loaded) = fixture();
    let (key, original) = function_unit(&source, &loaded, "read-choice");
    load_artifact(&effect_tests::replace_unit(&loaded, key, &original, vec![]))
        .expect("neutral choice rehash preserves admission");
    let begin = code(&original)
        .instructions
        .iter()
        .position(|i| matches!(i, CompiledInstruction::MatchBorrowedOwned { .. }))
        .unwrap();
    assert!(matches!(
        code(&original).instructions[begin],
        CompiledInstruction::MatchBorrowedOwned {
            source_local: 0,
            ..
        }
    ));
    for attack in [
        "source",
        "swap-cases",
        "target",
        "type",
        "binding",
        "missing-case",
        "erase-end",
        "mispair-end",
    ] {
        let mut changed = original.clone();
        let instructions = &mut code_mut(&mut changed).instructions;
        if matches!(attack, "erase-end" | "mispair-end") {
            let end = instructions
                .iter()
                .position(|i| matches!(i, CompiledInstruction::EndOwnedBorrow { .. }))
                .unwrap();
            instructions[end] = if attack == "erase-end" {
                CompiledInstruction::Jump(end as u32 + 1)
            } else {
                let CompiledInstruction::MatchBorrowedOwned { cases, .. } = &instructions[begin]
                else {
                    unreachable!()
                };
                CompiledInstruction::EndOwnedBorrow {
                    binding_local: cases[1].binding_local,
                }
            };
        } else {
            let CompiledInstruction::MatchBorrowedOwned {
                source_local,
                cases,
                ..
            } = &mut instructions[begin]
            else {
                unreachable!()
            };
            assert_eq!(cases.len(), 2);
            match attack {
                "source" => *source_local = 1,
                "swap-cases" => cases.swap(0, 1),
                "target" => cases[0].target = cases[1].target,
                "type" => cases[0].binding_type = cases[1].binding_type,
                "binding" => cases[0].binding_local = cases[1].binding_local,
                "missing-case" => {
                    cases.pop();
                }
                _ => unreachable!(),
            }
        }
        reject_changed_unit(&loaded, key, &changed, attack);
    }
}

#[test]
fn nested_owned_read_artifact_rejects_outer_before_inner_cleanup() {
    let (source, loaded) = fixture();
    let (key, original) = function_unit(&source, &loaded, "read-nested");
    load_artifact(&effect_tests::replace_unit(&loaded, key, &original, vec![])).unwrap();
    let ends = code(&original)
        .instructions
        .iter()
        .enumerate()
        .filter_map(|(index, i)| {
            matches!(i, CompiledInstruction::EndOwnedBorrow { .. }).then_some(index)
        })
        .collect::<Vec<_>>();
    assert_eq!(ends.len(), 2);
    let mut changed = original.clone();
    code_mut(&mut changed).instructions.swap(ends[0], ends[1]);
    reject_changed_unit(&loaded, key, &changed, "ancestor cleanup before child");
}

#[test]
fn owned_read_artifact_rejects_coherent_untaken_view_to_owner_counterfeit() {
    let (source, loaded) = fixture();
    let (key, mut unit) = function_unit(&source, &loaded, "read-owner-role");
    load_artifact(&effect_tests::replace_unit(&loaded, key, &unit, vec![])).unwrap();
    let owner = unit.source.owner;
    let OwnerRecord::Declaration(declaration) = &source.owners[&owner] else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &declaration.payload else {
        unreachable!()
    };
    let root = function.body;
    let parameters = function.parameters.clone();
    let OwnerRecord::Expression(root_record) = &source.owners[&OwnerKey::Expression(root)] else {
        unreachable!()
    };
    let ExpressionOperation::If {
        when_false: scope, ..
    } = root_record.operation
    else {
        unreachable!()
    };
    let scope_owner = OwnerKey::Expression(scope);
    let OwnerRecord::Expression(scope_record) = &source.owners[&scope_owner] else {
        unreachable!()
    };
    let ExpressionOperation::BorrowOwnedField {
        product_type,
        source: parent,
        ref field,
        binding,
        body,
    } = scope_record.operation
    else {
        unreachable!()
    };
    let field = field.clone();
    let binding_owner = OwnerKey::Binding(binding);
    let mut forged = source.clone();
    let OwnerRecord::Binding(record) = forged.owners.get_mut(&binding_owner).unwrap() else {
        unreachable!()
    };
    assert_eq!(record.kind, BindingKind::OwnedBorrow);
    record.kind = BindingKind::OwnedUnpack;
    let OwnerRecord::Expression(record) = forged.owners.get_mut(&scope_owner).unwrap() else {
        unreachable!()
    };
    record.operation = ExpressionOperation::UnpackOwned {
        product_type,
        source: parent,
        fields: vec![OwnedProductBinding {
            name: field,
            binding,
        }],
        body,
    };
    let limits = ExpressionValidationLimits {
        maximum_steps: crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK,
        maximum_diagnostics: 1,
    };
    let mut diagnostics = vec![];
    validate_expression_roots_with_limits(&forged, [owner], &mut diagnostics, &mut 0, limits)
        .unwrap();
    assert!(
        diagnostics.is_empty(),
        "counterfeit remains type-correct: {diagnostics:?}"
    );
    assert!(!crate::platform::kernel::memory_reference::accepts(&forged));
    validate_affine_roots_with_limits(&forged, [owner], &mut diagnostics, &mut 0, limits).unwrap();
    assert_eq!(
        diagnostics.len(),
        1,
        "borrowed parent cannot confer consumption: {diagnostics:?}"
    );

    // Make the compiled body agree exactly with the forged canonical scope. This
    // defeat must come from independent ownership admission, not correspondence.
    *code_mut(&mut unit) = super::super::lower::canonical_code(
        &forged,
        unit.source.package,
        Some(declaration_named(&forged, "read-owner-role")),
        &unit.tables,
        root,
        &parameters,
    )
    .unwrap();
    let facts = crate::platform::witness::rebuild_canonical_facts(&forged).unwrap();
    let summary = &facts.summaries[&owner];
    unit.source = CompilationSource {
        package: forged.root.package_id,
        owner,
        kind: summary.kind,
        semantic_interface: summary.semantic_interface,
        implementation: summary.implementation,
        type_digest: summary.type_digest,
        effect: summary.effect,
        capability: summary.capability,
        test: summary.test,
        validation_dependencies: summary.validation_dependencies,
    };
    unit.key = CompilationUnitKey::derive_generation(
        &unit.source,
        unit.optimization,
        unit.contract_version,
        unit.bytecode_contract_version,
        unit.graph_contract_version,
    )
    .unwrap();
    unit.validate()
        .expect("coherent counterfeit has valid compiled structure");
    let altered = replace_reference_owners(
        &loaded,
        &[
            (scope_owner, forged.owners[&scope_owner].clone()),
            (binding_owner, forged.owners[&binding_owner].clone()),
        ],
    );
    let bytes = effect_tests::replace_unit(&altered, key, &unit, vec![]);
    let error = load_artifact(&bytes)
        .expect_err("a view cannot become an owner through coherent rehashing");
    assert_eq!(error.code, "artifact_affine_meaning", "{error:?}");
}

fn replace_reference_owners(
    loaded: &LoadedArtifact,
    replacements: &[(OwnerKey, OwnerRecord)],
) -> LoadedArtifact {
    let mut altered = loaded.clone();
    let package = altered
        .manifest
        .packages
        .iter_mut()
        .find(|p| p.package == loaded.manifest.root_package)
        .unwrap();
    let old_root = package.reference_owners;
    let mut entries = artifact_map_entries(loaded, old_root);
    for (owner, record) in replacements {
        let encoded = EncodedOwnerKey::new(*owner).bytes().to_vec();
        let (_, value) = entries.iter_mut().find(|(key, _)| *key == encoded).unwrap();
        let mut binding = decode_owner_binding(value, *owner).unwrap();
        assert!(
            package
                .runtime_owners
                .iter()
                .all(|entry| entry.object != binding.object)
        );
        let old = ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
        let (digest, bytes) = encode_owner(record).unwrap();
        binding.object = digest;
        *value = encode_owner_binding(&binding);
        altered.objects.remove(&old).unwrap();
        altered.objects.insert(
            ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
            bytes,
        );
    }
    package.reference_owners = replace_artifact_map(&mut altered.objects, entries);
    assert_eq!(package.reference_owners.entries(), old_root.entries());
    let mut old_pages = MemoryPageStore::default();
    PersistentMap::from_root(old_root)
        .copy_reachable(
            &ObjectPageReader::new(loaded),
            &mut old_pages,
            &mut MapWork::default(),
        )
        .unwrap();
    let mut live_pages = MemoryPageStore::default();
    for package in &altered.manifest.packages {
        let compilation = CompilationManifest::decode(
            &altered.objects[&package.compilation.object_key()],
            package.compilation,
        )
        .unwrap();
        for root in [
            package.interface_owners,
            package.reference_owners,
            compilation.units,
        ] {
            PersistentMap::from_root(root)
                .copy_reachable(
                    &ObjectPageReader::new(&altered),
                    &mut live_pages,
                    &mut MapWork::default(),
                )
                .unwrap();
        }
    }
    let live = live_pages
        .objects()
        .map(|(digest, _)| digest)
        .collect::<BTreeSet<_>>();
    for (digest, _) in old_pages.objects() {
        if !live.contains(&digest) {
            altered.objects.remove(&ObjectKey::from_digest(
                ObjectDomain::MapPage,
                digest.bytes(),
            ));
        }
    }
    altered
}
