//! Focused normalized compiler-unit tests.

use super::*;
use crate::platform::change::{AuthoredChange, AuthoredChangeSet, ChangeBudget, PrimitiveEdit};
use crate::platform::kernel::{
    BindingKind, BindingRecord, CaseRecord, CaseReference, DeclarationVisibility,
    ExpressionOperation, ExpressionRecord, FieldSelector, LocalValueReference, MapExpressionEntry,
    MatchExpressionArm, Name, OwnerHeader, OwnerKey, OwnerKind, OwnerRecord, RequirementReference,
    TextValue, TypeForm, TypeObject, TypeObjectDigest, decode_owner, encode_owner,
    encode_type_object,
};
use crate::platform::persistent_map::{MapRoot, MapWork, MemoryPageStore, PersistentMap};
use crate::platform::publication::{GraphRepository, PublicationOptions, PublicationOutcome};
use crate::platform::semantic_id::{BindingId, CaseId, ExpressionId};
use crate::platform::storage::object::{ObjectDomain, ObjectKey};
use crate::platform::storage::page_store::ObjectPageReader;
use crate::platform::witness::rebuild_full_witness;

fn declaration_named(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    name: &str,
) -> crate::platform::semantic_id::DeclarationId {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record))
                if record.name.as_str() == name =>
            {
                Some(*declaration)
            }
            _ => None,
        })
        .expect("fixture declaration")
}

fn module_named(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    name: &str,
) -> crate::platform::semantic_id::ModuleId {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Module(module), OwnerRecord::Module(record))
                if record.name.as_str() == name =>
            {
                Some(*module)
            }
            _ => None,
        })
        .expect("fixture module")
}

fn compile_memory(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    owner: OwnerKey,
) -> CompilationReceipt {
    let witness = rebuild_full_witness(snapshot).expect("full witness");
    compile_unit(
        snapshot,
        &witness,
        owner,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("normalized compilation")
}

fn add_expression(
    snapshot: &mut crate::platform::kernel::KernelSnapshot,
    ordinal: u64,
    operation: ExpressionOperation,
) -> ExpressionId {
    let id = ExpressionId::migrate(b"graph-5-compiler-expression-coverage", ordinal);
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(id),
                OwnerRecord::Expression(
                    ExpressionRecord::new(id, operation).expect("coverage expression"),
                ),
            )
            .is_none()
    );
    id
}

fn artifact_map_entries(artifact: &LoadedArtifact, root: MapRoot) -> Vec<(Vec<u8>, Vec<u8>)> {
    let reader = ObjectPageReader::new(artifact);
    let mut work = MapWork::default();
    let mut entries = Vec::new();
    PersistentMap::from_root(root)
        .for_each(&reader, &mut work, |key, value| {
            entries.push((key.to_vec(), value.to_vec()));
            Ok(())
        })
        .expect("read exact artifact map");
    entries
}

fn replace_artifact_map(
    objects: &mut std::collections::BTreeMap<ObjectKey, Vec<u8>>,
    entries: Vec<(Vec<u8>, Vec<u8>)>,
) -> MapRoot {
    let mut pages = MemoryPageStore::default();
    let mut work = MapWork::default();
    let map = PersistentMap::from_sorted(&mut pages, entries, &mut work)
        .expect("build replacement artifact map");
    for (digest, bytes) in pages.objects() {
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes()),
            bytes.to_vec(),
        );
    }
    map.root()
}

fn structurally_empty_snapshot(seed: &[u8]) -> crate::platform::kernel::KernelSnapshot {
    let root_placeholder = |marker| {
        MapRoot::from_parts(
            crate::platform::persistent_map::PageDigest::from_bytes([marker; 32]),
            0,
            crate::platform::persistent_map::MapContentDigest::from_bytes([marker; 32]),
        )
    };
    crate::platform::kernel::KernelSnapshot {
        root: crate::platform::kernel::SemanticRoot {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: crate::platform::semantic_id::RepositoryId::migrate(seed, 0),
            package_id: crate::platform::kernel::PackageId::migrate(seed, 0),
            package_name: Name::new("empty_compiler").unwrap(),
            owners: root_placeholder(1),
            dependencies: root_placeholder(2),
            retirements: root_placeholder(3),
        },
        owners: std::collections::BTreeMap::new(),
        types: std::collections::BTreeMap::new(),
        dependency_interfaces: std::collections::BTreeMap::new(),
        dependency_types: std::collections::BTreeMap::new(),
        blobs: std::collections::BTreeMap::new(),
        dependencies: std::collections::BTreeMap::new(),
        retirements: std::collections::BTreeMap::new(),
    }
}

pub(crate) fn complete_expression_snapshot() -> crate::platform::kernel::KernelSnapshot {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let package = snapshot.root.package_id;
    let caller = declaration_named(&snapshot, "caller");
    let callee = declaration_named(&snapshot, "callee");
    let variant = declaration_named(&snapshot, "State");
    let unit_type = snapshot
        .types
        .iter()
        .find_map(|(digest, object)| matches!(object.form, TypeForm::Unit).then_some(*digest))
        .expect("unit type");
    let i64_object = TypeObject::new(TypeForm::I64).expect("i64 type");
    let (i64_type, _) =
        crate::platform::kernel::encode_type_object(&i64_object).expect("i64 encoding");
    snapshot.types.insert(i64_type, i64_object);
    let requirement = snapshot
        .owners
        .keys()
        .find_map(|owner| match owner {
            OwnerKey::Requirement(requirement) => Some(*requirement),
            _ => None,
        })
        .expect("fixture requirement");

    let payload_case = CaseId::migrate(b"graph-5-compiler-expression-coverage", 0);
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Case(payload_case),
                OwnerRecord::Case(CaseRecord {
                    header: OwnerHeader::new(OwnerKey::Case(payload_case), OwnerKind::Case),
                    declaration: variant,
                    name: Name::new("Payload").unwrap(),
                    payload: Some(unit_type),
                }),
            )
            .is_none()
    );
    let OwnerRecord::Declaration(variant_record) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(variant))
        .expect("variant declaration")
    else {
        panic!("variant declaration kind")
    };
    let crate::platform::kernel::DeclarationPayload::Variant { cases } =
        &mut variant_record.payload
    else {
        panic!("variant payload")
    };
    cases.push(payload_case);
    cases.sort_unstable();

    let match_binding = BindingId::migrate(b"graph-5-compiler-expression-coverage", 0);
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Binding(match_binding),
                OwnerRecord::Binding(BindingRecord {
                    header: OwnerHeader::new(OwnerKey::Binding(match_binding), OwnerKind::Binding),
                    name: Name::new("payload").unwrap(),
                    kind: BindingKind::MatchPayload,
                    value: None,
                    declared_type: Some(unit_type),
                }),
            )
            .is_none()
    );
    let match_payload_body = add_expression(
        &mut snapshot,
        0,
        ExpressionOperation::Local {
            value: LocalValueReference::MatchPayload(match_binding),
        },
    );
    let match_expression = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Expression(expression), OwnerRecord::Expression(record))
                if matches!(record.operation, ExpressionOperation::Match { .. }) =>
            {
                Some(*expression)
            }
            _ => None,
        })
        .expect("fixture match");
    let OwnerRecord::Expression(record) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(match_expression))
        .expect("match expression")
    else {
        panic!("match expression kind")
    };
    let ExpressionOperation::Match { arms, .. } = &mut record.operation else {
        panic!("match operation")
    };
    arms.push(MatchExpressionArm {
        case: CaseReference {
            package,
            case: payload_case,
        },
        payload_binding: Some(match_binding),
        body: match_payload_body,
    });
    arms.sort_by_key(|arm| arm.case);

    let condition = add_expression(&mut snapshot, 1, ExpressionOperation::Bool { value: true });
    let when_true = add_expression(&mut snapshot, 2, ExpressionOperation::Unit {});
    let when_false = add_expression(&mut snapshot, 3, ExpressionOperation::Unit {});
    let conditional = add_expression(
        &mut snapshot,
        4,
        ExpressionOperation::If {
            condition,
            when_true,
            when_false,
        },
    );
    let text = add_expression(
        &mut snapshot,
        5,
        ExpressionOperation::Text {
            value: TextValue::Inline {
                text: "dynamic".to_owned(),
            },
        },
    );
    let static_text = add_expression(
        &mut snapshot,
        6,
        ExpressionOperation::StaticText {
            value: TextValue::Inline {
                text: "static".to_owned(),
            },
        },
    );
    let integer = add_expression(&mut snapshot, 7, ExpressionOperation::I64 { value: 7 });
    let function_value = add_expression(
        &mut snapshot,
        8,
        ExpressionOperation::FunctionValue {
            function: crate::platform::kernel::DeclarationReference {
                package,
                declaration: callee,
            },
            type_arguments: Vec::new(),
        },
    );
    let invoke_callee = add_expression(
        &mut snapshot,
        9,
        ExpressionOperation::FunctionValue {
            function: crate::platform::kernel::DeclarationReference {
                package,
                declaration: callee,
            },
            type_arguments: Vec::new(),
        },
    );
    let invoke_argument = add_expression(&mut snapshot, 10, ExpressionOperation::Unit {});
    let invoke = add_expression(
        &mut snapshot,
        11,
        ExpressionOperation::Invoke {
            callee: invoke_callee,
            arguments: vec![invoke_argument],
        },
    );
    let structural_value = add_expression(&mut snapshot, 12, ExpressionOperation::Unit {});
    let structural_name = Name::new("structural").unwrap();
    let structural_record = add_expression(
        &mut snapshot,
        13,
        ExpressionOperation::Record {
            nominal_type: None,
            fields: vec![crate::platform::kernel::RecordExpressionField {
                selector: FieldSelector::Structural(structural_name.clone()),
                value: structural_value,
            }],
        },
    );
    let structural_field = add_expression(
        &mut snapshot,
        14,
        ExpressionOperation::Field {
            value: structural_record,
            selector: FieldSelector::Structural(structural_name),
        },
    );
    let list_item = add_expression(&mut snapshot, 15, ExpressionOperation::Unit {});
    let list = add_expression(
        &mut snapshot,
        16,
        ExpressionOperation::List {
            item_type: unit_type,
            items: vec![list_item],
        },
    );
    let map_key = add_expression(&mut snapshot, 17, ExpressionOperation::I64 { value: 1 });
    let map_value = add_expression(&mut snapshot, 18, ExpressionOperation::Unit {});
    let map = add_expression(
        &mut snapshot,
        19,
        ExpressionOperation::Map {
            key_type: i64_type,
            value_type: unit_type,
            entries: vec![MapExpressionEntry {
                key: map_key,
                value: map_value,
            }],
        },
    );
    let transaction_binding = BindingId::migrate(b"graph-5-compiler-expression-coverage", 1);
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Binding(transaction_binding),
                OwnerRecord::Binding(BindingRecord {
                    header: OwnerHeader::new(
                        OwnerKey::Binding(transaction_binding),
                        OwnerKind::Binding,
                    ),
                    name: Name::new("transaction").unwrap(),
                    kind: BindingKind::Transaction,
                    value: None,
                    declared_type: Some(unit_type),
                }),
            )
            .is_none()
    );
    let transaction_body = add_expression(
        &mut snapshot,
        20,
        ExpressionOperation::Local {
            value: LocalValueReference::TransactionBinding(transaction_binding),
        },
    );
    let transaction = add_expression(
        &mut snapshot,
        21,
        ExpressionOperation::Transaction {
            requirement: RequirementReference {
                package,
                requirement,
            },
            binding: transaction_binding,
            body: transaction_body,
        },
    );

    let caller_body = match &snapshot.owners[&OwnerKey::Declaration(caller)] {
        OwnerRecord::Declaration(record) => match &record.payload {
            crate::platform::kernel::DeclarationPayload::Function(function) => function.body,
            _ => panic!("caller function"),
        },
        _ => panic!("caller declaration"),
    };
    let OwnerRecord::Expression(root) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(caller_body))
        .expect("caller root")
    else {
        panic!("caller root kind")
    };
    let ExpressionOperation::Sequence { items } = &mut root.operation else {
        panic!("caller sequence")
    };
    items.splice(
        0..0,
        [
            conditional,
            text,
            static_text,
            integer,
            function_value,
            invoke,
            structural_field,
            list,
            map,
            transaction,
        ],
    );
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    snapshot
}

#[test]
fn every_fixture_declaration_and_target_compiles_and_round_trips() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let witness = rebuild_full_witness(&snapshot).expect("full witness");
    let selected = snapshot
        .owners
        .keys()
        .copied()
        .filter(|owner| matches!(owner, OwnerKey::Declaration(_) | OwnerKey::Target(_)))
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 11);

    for owner in selected {
        let first = compile_unit(
            &snapshot,
            &witness,
            owner,
            OptimizationPolicy::DeterministicBaseline,
        )
        .expect("fixture unit compiles");
        let second = compile_unit(
            &snapshot,
            &witness,
            owner,
            OptimizationPolicy::DeterministicBaseline,
        )
        .expect("fixture unit recompiles");
        assert_eq!(first.key, second.key);
        assert_eq!(first.object, second.object);
        assert_eq!(first.bytes, second.bytes);
        assert_eq!(first.object.domain, ObjectDomain::CompilerUnit);
        assert_eq!(
            CompilationUnit::decode(&first.bytes, first.object).expect("unit decode"),
            first.unit
        );
        assert!(first.work.owner_records_read <= snapshot.owners.len() as u64);
        let code_count = first.unit.payload.codes().count();
        assert_eq!(first.work.expression_records_read > 0, code_count > 0);
        assert!(first.work.instructions_emitted >= first.work.expression_records_read);
    }
}

#[test]
fn task_unit_uses_exact_dense_nominal_and_capability_operands() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let caller = declaration_named(&snapshot, "caller");
    let receipt = compile_memory(&snapshot, OwnerKey::Declaration(caller));
    let CompilationPayload::Function { signature, code } = &receipt.unit.payload else {
        panic!("caller must compile as a function")
    };
    assert_eq!(signature.task_requirements.len(), 1);
    let declared_requirement =
        receipt.unit.tables.requirements[signature.task_requirements[0] as usize];
    let OwnerRecord::Declaration(caller_record) = &snapshot.owners[&OwnerKey::Declaration(caller)]
    else {
        panic!("caller declaration")
    };
    let crate::platform::kernel::DeclarationPayload::Function(function) = &caller_record.payload
    else {
        panic!("caller function")
    };
    let crate::platform::kernel::FunctionEffect::Task { requirements } = &function.effect else {
        panic!("caller task effect")
    };
    assert_eq!(requirements, &[declared_requirement]);

    let mut saw_record = false;
    let mut saw_field = false;
    let mut saw_variant = false;
    let mut saw_match = false;
    let mut saw_perform = false;
    for instruction in &code.instructions {
        match instruction {
            CompiledInstruction::Record {
                nominal_type: Some(declaration),
                fields,
            } => {
                saw_record = true;
                assert!((*declaration as usize) < receipt.unit.tables.declarations.len());
                assert!(fields.iter().all(|field| matches!(
                    field,
                    super::unit::CompiledFieldSelector::Nominal(index)
                        if (*index as usize) < receipt.unit.tables.fields.len()
                )));
            }
            CompiledInstruction::Field(super::unit::CompiledFieldSelector::Nominal(field)) => {
                saw_field = true;
                assert!((*field as usize) < receipt.unit.tables.fields.len());
            }
            CompiledInstruction::Variant { case, .. } => {
                saw_variant = true;
                assert!((*case as usize) < receipt.unit.tables.cases.len());
            }
            CompiledInstruction::SwitchVariant(arms) => {
                saw_match = true;
                assert!(
                    arms.iter()
                        .all(|arm| (arm.case as usize) < receipt.unit.tables.cases.len())
                );
            }
            CompiledInstruction::Perform {
                requirement,
                operation,
                ..
            } => {
                saw_perform = true;
                assert_eq!(
                    receipt.unit.tables.requirements[*requirement as usize],
                    declared_requirement
                );
                assert!((*operation as usize) < receipt.unit.tables.operations.len());
            }
            _ => {}
        }
    }
    assert!(saw_record && saw_field && saw_variant && saw_match && saw_perform);
    for forbidden in [b"caller".as_slice(), b"store", b"read", b"Ready"] {
        assert!(
            !receipt
                .bytes
                .windows(forbidden.len())
                .any(|window| window == forbidden),
            "hot task unit retained presentation string {:?}",
            String::from_utf8_lossy(forbidden)
        );
    }
}

#[test]
fn every_graph5_expression_form_lowers_with_verified_control_flow() {
    let snapshot = complete_expression_snapshot();
    let caller = declaration_named(&snapshot, "caller");
    let receipt = compile_memory(&snapshot, OwnerKey::Declaration(caller));
    let CompilationPayload::Function { code, .. } = &receipt.unit.payload else {
        panic!("caller function")
    };
    let has = |predicate: fn(&CompiledInstruction) -> bool| code.instructions.iter().any(predicate);
    assert!(has(|value| matches!(value, CompiledInstruction::Bool(_))));
    assert!(has(|value| matches!(value, CompiledInstruction::I64(_))));
    assert!(has(|value| matches!(value, CompiledInstruction::Text(_))));
    assert!(has(|value| matches!(
        value,
        CompiledInstruction::StaticText(_)
    )));
    assert!(has(|value| matches!(
        value,
        CompiledInstruction::JumpIfFalse(_)
    )));
    assert!(has(|value| matches!(
        value,
        CompiledInstruction::FunctionValue { .. }
    )));
    assert!(has(|value| matches!(
        value,
        CompiledInstruction::Invoke { .. }
    )));
    assert!(has(|value| matches!(
        value,
        CompiledInstruction::Record {
            nominal_type: None,
            fields
        } if fields.iter().all(|field| matches!(field, super::unit::CompiledFieldSelector::Structural(_)))
    )));
    assert!(has(|value| matches!(
        value,
        CompiledInstruction::Field(super::unit::CompiledFieldSelector::Structural(_))
    )));
    assert!(has(|value| matches!(
        value,
        CompiledInstruction::List { .. }
    )));
    assert!(has(|value| matches!(
        value,
        CompiledInstruction::Map { .. }
    )));
    assert!(has(|value| matches!(
        value,
        CompiledInstruction::BeginTransaction { .. }
    )));
    assert!(has(|value| matches!(
        value,
        CompiledInstruction::CommitTransaction { .. }
    )));
    assert!(code.instructions.iter().any(|value| matches!(
        value,
        CompiledInstruction::SwitchVariant(arms)
            if arms.iter().any(|arm| arm.binding_local.is_some())
    )));
    assert_eq!(receipt.unit.tables.texts.len(), 2);
    assert_eq!(receipt.unit.tables.structural_names.len(), 1);
    CompilationUnit::decode(&receipt.bytes, receipt.object)
        .expect("all-form unit passes strict control-flow verification");
}

#[test]
fn transaction_scope_binding_is_a_unit_marker_not_a_live_language_handle() {
    let mut snapshot = complete_expression_snapshot();
    let i64_type = snapshot
        .types
        .iter()
        .find_map(|(digest, object)| matches!(object.form, TypeForm::I64).then_some(*digest))
        .expect("coverage i64 type");
    let transaction = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Binding(binding), OwnerRecord::Binding(record))
                if record.kind == BindingKind::Transaction =>
            {
                Some(*binding)
            }
            _ => None,
        })
        .expect("transaction binding");
    let OwnerRecord::Binding(record) = snapshot
        .owners
        .get_mut(&OwnerKey::Binding(transaction))
        .expect("transaction binding record")
    else {
        panic!("transaction binding owner kind")
    };
    record.declared_type = Some(i64_type);

    let diagnostics = crate::platform::kernel::validate_full(&snapshot)
        .expect_err("a transaction binding cannot impersonate an ordinary value type");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "kernel_type_transaction_binding")
    );
}

#[test]
fn rename_and_move_preserve_selected_and_caller_unit_bytes() {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let callee = declaration_named(&snapshot, "callee");
    let caller = declaration_named(&snapshot, "caller");
    let destination = module_named(&snapshot, "second");
    let before_callee = compile_memory(&snapshot, OwnerKey::Declaration(callee));
    let before_caller = compile_memory(&snapshot, OwnerKey::Declaration(caller));

    let OwnerRecord::Declaration(record) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(callee))
        .expect("callee declaration")
    else {
        panic!("callee declaration kind")
    };
    record.name = Name::new("renamed_callee").expect("renamed callee");
    record.module = destination;

    let after_callee = compile_memory(&snapshot, OwnerKey::Declaration(callee));
    let after_caller = compile_memory(&snapshot, OwnerKey::Declaration(caller));
    assert_eq!(before_callee.key, after_callee.key);
    assert_eq!(before_callee.bytes, after_callee.bytes);
    assert_eq!(before_caller.key, after_caller.key);
    assert_eq!(before_caller.bytes, after_caller.bytes);

    let CompilationPayload::Function { code, .. } = &after_caller.unit.payload else {
        panic!("caller function")
    };
    let called = code.instructions.iter().find_map(|instruction| {
        let CompiledInstruction::Call { function, .. } = instruction else {
            return None;
        };
        let reference = after_caller.unit.tables.declarations[*function as usize];
        (reference.declaration == callee).then_some(reference)
    });
    assert!(
        called.is_some(),
        "exact caller reference survived rename and move"
    );
}

#[test]
fn semantic_interface_change_invalidates_the_compiler_unit_key() {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let callee = declaration_named(&snapshot, "callee");
    let before = compile_memory(&snapshot, OwnerKey::Declaration(callee));

    let OwnerRecord::Declaration(record) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(callee))
        .expect("callee declaration")
    else {
        panic!("callee declaration kind")
    };
    record.visibility = match record.visibility {
        DeclarationVisibility::Private => DeclarationVisibility::Package,
        DeclarationVisibility::Package | DeclarationVisibility::Public => {
            DeclarationVisibility::Private
        }
    };

    let after = compile_memory(&snapshot, OwnerKey::Declaration(callee));
    assert_ne!(
        before.unit.source.semantic_interface,
        after.unit.source.semantic_interface
    );
    assert_ne!(before.key, after.key);
    assert_ne!(before.bytes, after.bytes);
}

#[test]
fn body_reorder_changes_unit_key_but_repository_point_lowering_matches_memory() {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let caller = declaration_named(&snapshot, "caller");
    let before = compile_memory(&snapshot, OwnerKey::Declaration(caller));
    let body = match &snapshot.owners[&OwnerKey::Declaration(caller)] {
        OwnerRecord::Declaration(record) => match &record.payload {
            crate::platform::kernel::DeclarationPayload::Function(function) => function.body,
            _ => panic!("caller function"),
        },
        _ => panic!("caller declaration"),
    };
    let OwnerRecord::Expression(root) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(body))
        .expect("caller root")
    else {
        panic!("caller root expression")
    };
    let ExpressionOperation::Sequence { items } = &mut root.operation else {
        panic!("caller sequence")
    };
    items.swap(0, 1);
    let after = compile_memory(&snapshot, OwnerKey::Declaration(caller));
    assert_ne!(before.key, after.key);
    assert_ne!(before.bytes, after.bytes);

    let temporary = tempfile::tempdir().expect("compiler repository parent");
    let created = GraphRepository::create(&temporary.path().join("repository"), &snapshot, None)
        .expect("Graph 5 repository");
    let view = created
        .repository
        .view_current()
        .expect("revision-pinned view");
    let repository = compile_unit(
        &view,
        &view,
        OwnerKey::Declaration(caller),
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("repository point lowering");
    assert_eq!(repository.key, after.key);
    assert_eq!(repository.bytes, after.bytes);
    assert!(repository.work.owner_records_read < snapshot.owners.len() as u64);
    assert!(repository.work.canonical.point_reads < snapshot.owners.len() as u64);
    assert_eq!(repository.work.witness.point_reads, 1);
}

#[test]
fn compiler_unit_decoder_rejects_foreign_identity_predecessor_and_bad_dense_index() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let caller = declaration_named(&snapshot, "caller");
    let receipt = compile_memory(&snapshot, OwnerKey::Declaration(caller));

    let foreign = ObjectKey::from_digest(ObjectDomain::Owner, receipt.object.digest.bytes());
    assert_eq!(
        CompilationUnit::decode(&receipt.bytes, foreign)
            .expect_err("foreign object domain must reject")
            .code,
        "compiler_unit_digest"
    );

    let mut predecessor = receipt.bytes.clone();
    predecessor[..8].copy_from_slice(b"LKJCUN00");
    let predecessor_key = ObjectKey::for_bytes(ObjectDomain::CompilerUnit, &predecessor);
    assert_eq!(
        CompilationUnit::decode(&predecessor, predecessor_key)
            .expect_err("predecessor compiler-unit magic must reject")
            .code,
        "packed_contract"
    );

    let mut invalid = receipt.unit;
    let CompilationPayload::Function { code, .. } = &mut invalid.payload else {
        panic!("caller function")
    };
    let perform = code
        .instructions
        .iter_mut()
        .find(|instruction| matches!(instruction, CompiledInstruction::Perform { .. }))
        .expect("capability instruction");
    let CompiledInstruction::Perform { requirement, .. } = perform else {
        unreachable!()
    };
    *requirement = u32::MAX;
    assert_eq!(
        invalid
            .encode()
            .expect_err("invalid dense index must reject")
            .code,
        "compiler_unit_index"
    );
}

#[test]
fn clean_compilation_manifest_persists_and_reopens_exactly() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let temporary = tempfile::tempdir().expect("compiler manifest parent");
    let root = temporary.path().join("repository");
    let created = GraphRepository::create(&root, &snapshot, None).expect("Graph 5 repository");

    let built = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("clean normalized compilation");
    assert_eq!(built.profile, CompilationBuildProfile::Clean);
    assert_eq!(built.units_compiled, 11);
    assert_eq!(built.units_reused, 0);
    assert_eq!(built.units_removed, 0);
    assert_eq!(built.manifest.units.entries(), 11);
    assert_eq!(built.work.inventory_bindings, snapshot.owners.len() as u64);
    assert!(built.work.compilation.owner_records_read < snapshot.owners.len() as u64 * 11);

    let cached = load_current_compilation(&created.repository)
        .expect("load current compilation")
        .expect("current cache head");
    assert_eq!(cached.digest, built.manifest_digest);
    assert_eq!(cached.manifest, built.manifest);
    let validation = validate_current_compilation(&created.repository, built.manifest_digest)
        .expect("full compilation validation");
    assert_eq!(validation.units, 11);
    assert_eq!(validation.map.entries_visited, 22);

    drop(created);
    let reopened = GraphRepository::open(&root).expect("reopen Graph 5 repository");
    let cached = load_current_compilation(&reopened)
        .expect("load reopened compilation")
        .expect("reopened cache head");
    assert_eq!(cached.digest, built.manifest_digest);
    assert_eq!(
        validate_current_compilation(&reopened, cached.digest)
            .expect("validate reopened compilation")
            .units,
        11
    );
}

#[test]
fn structurally_empty_package_builds_one_valid_empty_manifest() {
    let snapshot = structurally_empty_snapshot(b"empty-compiler-manifest");
    let temporary = tempfile::tempdir().expect("empty compiler parent");
    let created = GraphRepository::create(&temporary.path().join("repository"), &snapshot, None)
        .expect("empty Graph 5 repository");

    let built = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("empty clean compilation");
    assert_eq!(built.units_compiled, 0);
    assert_eq!(built.units_reused, 0);
    assert_eq!(built.units_removed, 0);
    assert_eq!(built.manifest.units.entries(), 0);
    assert_eq!(
        validate_current_compilation(&created.repository, built.manifest_digest)
            .expect("validate empty manifest")
            .units,
        0
    );
    let artifact = link_artifact(&created.repository, built.manifest_digest, &[])
        .expect("link empty package artifact");
    let loaded = load_artifact(&artifact.artifact.bytes).expect("load empty package artifact");
    assert_eq!(loaded.manifest.packages.len(), 1);
    assert!(
        loaded
            .root_package()
            .expect("root package")
            .runtime_owners
            .is_empty()
    );
}

#[test]
fn body_edit_incremental_manifest_equals_a_clean_rebuild() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let caller = declaration_named(&snapshot, "caller");
    let body = match &snapshot.owners[&OwnerKey::Declaration(caller)] {
        OwnerRecord::Declaration(record) => match &record.payload {
            crate::platform::kernel::DeclarationPayload::Function(function) => function.body,
            _ => panic!("caller function"),
        },
        _ => panic!("caller declaration"),
    };
    let temporary = tempfile::tempdir().expect("incremental manifest parent");
    let created = GraphRepository::create(&temporary.path().join("repository"), &snapshot, None)
        .expect("Graph 5 repository");
    let base = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("base clean compilation");
    let before_caller = super::cache::read_current_binding(
        &created.repository,
        base.manifest_digest,
        OwnerKey::Declaration(caller),
    )
    .expect("base caller binding")
    .expect("base caller unit");

    let view = created.repository.view_current().expect("body edit view");
    let mut replacement = view
        .owner(OwnerKey::Expression(body))
        .expect("body read")
        .value
        .expect("body owner");
    let expected = crate::platform::kernel::encode_owner(&replacement)
        .expect("base body encoding")
        .0;
    let OwnerRecord::Expression(expression) = &mut replacement else {
        panic!("body expression")
    };
    let ExpressionOperation::Sequence { items } = &mut expression.operation else {
        panic!("caller sequence")
    };
    items.swap(0, 1);
    let prepared = view
        .prepare_change(
            vec![PrimitiveEdit::ReplaceOwner {
                expected,
                record: replacement,
            }],
            PublicationOptions::default(),
        )
        .expect("prepare body edit");
    assert!(
        prepared
            .compiler_units
            .contains(&OwnerKey::Declaration(caller))
    );
    assert_eq!(prepared.compiler_units.len(), 2);
    assert!(matches!(
        created
            .repository
            .publish(&prepared)
            .expect("publish body edit"),
        PublicationOutcome::Accepted { .. }
    ));
    assert!(
        load_current_compilation(&created.repository)
            .expect("stale cache lookup")
            .is_none()
    );

    let mut underreported = prepared.clone();
    underreported.compiler_units.clear();
    assert_eq!(
        build_incremental(&created.repository, base.manifest_digest, &underreported,)
            .expect_err("an underreported prepared compiler plan must reject")
            .code,
        "compilation_incremental_prepared_binding"
    );

    let incremental = build_incremental(&created.repository, base.manifest_digest, &prepared)
        .expect("incremental compilation");
    assert_eq!(incremental.profile, CompilationBuildProfile::Incremental);
    assert_eq!(incremental.units_compiled, 2);
    assert_eq!(incremental.units_reused, 9);
    assert_eq!(incremental.units_removed, 0);
    assert_ne!(incremental.manifest.units, base.manifest.units);
    let after_caller = super::cache::read_current_binding(
        &created.repository,
        incremental.manifest_digest,
        OwnerKey::Declaration(caller),
    )
    .expect("current caller binding")
    .expect("current caller unit");
    assert_ne!(before_caller, after_caller);
    let incremental_artifact = link_artifact(&created.repository, incremental.manifest_digest, &[])
        .expect("link incremental compilation");

    let clean = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("clean oracle compilation");
    assert_eq!(incremental.manifest_digest, clean.manifest_digest);
    assert_eq!(incremental.manifest_bytes, clean.manifest_bytes);
    assert_eq!(incremental.manifest, clean.manifest);
    let clean_artifact = link_artifact(&created.repository, clean.manifest_digest, &[])
        .expect("link clean compilation");
    assert_eq!(
        incremental_artifact.artifact.bytes,
        clean_artifact.artifact.bytes
    );
    assert_eq!(
        incremental_artifact.artifact.bundle_digest,
        clean_artifact.artifact.bundle_digest
    );
    assert_eq!(
        validate_current_compilation(&created.repository, incremental.manifest_digest)
            .expect("validate incremental manifest")
            .units,
        11
    );
}

#[test]
fn rename_and_move_reuse_the_complete_compilation_unit_map() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let callee = declaration_named(&snapshot, "callee");
    let caller = declaration_named(&snapshot, "caller");
    let destination = module_named(&snapshot, "second");
    let temporary = tempfile::tempdir().expect("rename manifest parent");
    let created = GraphRepository::create(&temporary.path().join("repository"), &snapshot, None)
        .expect("Graph 5 repository");
    let base = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("base clean compilation");
    let base_callee = super::cache::read_current_binding(
        &created.repository,
        base.manifest_digest,
        OwnerKey::Declaration(callee),
    )
    .unwrap()
    .unwrap();
    let base_caller = super::cache::read_current_binding(
        &created.repository,
        base.manifest_digest,
        OwnerKey::Declaration(caller),
    )
    .unwrap()
    .unwrap();

    let view = created.repository.view_current().expect("rename move view");
    let mut replacement = view
        .owner(OwnerKey::Declaration(callee))
        .unwrap()
        .value
        .expect("callee owner");
    let expected = crate::platform::kernel::encode_owner(&replacement)
        .expect("callee encoding")
        .0;
    let OwnerRecord::Declaration(record) = &mut replacement else {
        panic!("callee declaration")
    };
    record.name = Name::new("renamed_callee").unwrap();
    record.module = destination;
    let prepared = view
        .prepare_change(
            vec![PrimitiveEdit::ReplaceOwner {
                expected,
                record: replacement,
            }],
            PublicationOptions::default(),
        )
        .expect("prepare rename and move");
    assert!(prepared.compiler_units.is_empty());
    assert!(matches!(
        created
            .repository
            .publish(&prepared)
            .expect("publish rename and move"),
        PublicationOutcome::Accepted { .. }
    ));

    let incremental = build_incremental(&created.repository, base.manifest_digest, &prepared)
        .expect("presentation-only incremental compilation");
    assert_eq!(incremental.units_compiled, 0);
    assert_eq!(incremental.units_reused, 11);
    assert_eq!(incremental.units_removed, 0);
    assert_eq!(incremental.manifest.units, base.manifest.units);
    assert_ne!(incremental.manifest_digest, base.manifest_digest);
    assert_eq!(
        super::cache::read_current_binding(
            &created.repository,
            incremental.manifest_digest,
            OwnerKey::Declaration(callee),
        )
        .unwrap()
        .unwrap(),
        base_callee
    );
    assert_eq!(
        super::cache::read_current_binding(
            &created.repository,
            incremental.manifest_digest,
            OwnerKey::Declaration(caller),
        )
        .unwrap()
        .unwrap(),
        base_caller
    );
    assert_eq!(
        validate_current_compilation(&created.repository, incremental.manifest_digest)
            .expect("validate reused unit map")
            .units,
        11
    );
}

#[test]
fn declaration_deletion_removes_one_unit_and_matches_a_clean_rebuild() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let constant = declaration_named(&snapshot, "unit_constant");
    let value = match &snapshot.owners[&OwnerKey::Declaration(constant)] {
        OwnerRecord::Declaration(record) => match record.payload {
            crate::platform::kernel::DeclarationPayload::Constant { value, .. } => value,
            _ => panic!("constant declaration"),
        },
        _ => panic!("constant owner"),
    };
    let temporary = tempfile::tempdir().expect("deletion manifest parent");
    let created = GraphRepository::create(&temporary.path().join("repository"), &snapshot, None)
        .expect("Graph 5 repository");
    let base = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("base clean compilation");
    let view = created.repository.view_current().expect("deletion view");
    let constant_record = view
        .owner(OwnerKey::Declaration(constant))
        .unwrap()
        .value
        .expect("constant record");
    let value_record = view
        .owner(OwnerKey::Expression(value))
        .unwrap()
        .value
        .expect("constant value record");
    let prepared = view
        .prepare_change(
            vec![
                PrimitiveEdit::DeleteOwner {
                    owner: OwnerKey::Declaration(constant),
                    expected: crate::platform::kernel::encode_owner(&constant_record)
                        .unwrap()
                        .0,
                },
                PrimitiveEdit::DeleteOwner {
                    owner: OwnerKey::Expression(value),
                    expected: crate::platform::kernel::encode_owner(&value_record)
                        .unwrap()
                        .0,
                },
            ],
            PublicationOptions::default(),
        )
        .expect("prepare declaration deletion");
    assert_eq!(
        prepared.compiler_units,
        [OwnerKey::Declaration(constant)].into_iter().collect()
    );
    assert!(matches!(
        created
            .repository
            .publish(&prepared)
            .expect("publish declaration deletion"),
        PublicationOutcome::Accepted { .. }
    ));

    let incremental = build_incremental(&created.repository, base.manifest_digest, &prepared)
        .expect("incremental deletion compilation");
    assert_eq!(incremental.units_compiled, 0);
    assert_eq!(incremental.units_reused, 10);
    assert_eq!(incremental.units_removed, 1);
    assert_eq!(incremental.manifest.units.entries(), 10);
    assert!(
        super::cache::read_current_binding(
            &created.repository,
            incremental.manifest_digest,
            OwnerKey::Declaration(constant),
        )
        .unwrap()
        .is_none()
    );

    let clean = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("clean deletion oracle");
    assert_eq!(incremental.manifest_digest, clean.manifest_digest);
    assert_eq!(incremental.manifest_bytes, clean.manifest_bytes);
}

#[test]
fn missing_and_corrupt_cache_heads_rebuild_without_changing_authority() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let temporary = tempfile::tempdir().expect("cache recovery parent");
    let root = temporary.path().join("repository");
    let created = GraphRepository::create(&root, &snapshot, None).expect("Graph 5 repository");
    let original_head = created.repository.current().unwrap().head;
    let first = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("first clean compilation");
    let cache_head = root.join("derived/compiler/CURRENT");

    std::fs::remove_file(&cache_head).expect("remove disposable cache head");
    assert!(
        load_current_compilation(&created.repository)
            .unwrap()
            .is_none()
    );
    let rebuilt = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("rebuild missing cache");
    assert_eq!(rebuilt.manifest_digest, first.manifest_digest);

    std::fs::write(&cache_head, b"corrupt derived cache head").expect("corrupt cache head");
    assert!(load_current_compilation(&created.repository).is_err());
    let recovered = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("replace corrupt cache head");
    assert_eq!(recovered.manifest_digest, first.manifest_digest);
    assert_eq!(
        load_current_compilation(&created.repository)
            .expect("load recovered cache")
            .expect("recovered cache hit")
            .digest,
        first.manifest_digest
    );
    assert_eq!(created.repository.current().unwrap().head, original_head);
}

#[test]
fn compilation_manifest_rejects_predecessor_magic_and_wrong_object_digest() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let temporary = tempfile::tempdir().expect("manifest decoder parent");
    let created = GraphRepository::create(&temporary.path().join("repository"), &snapshot, None)
        .expect("Graph 5 repository");
    let built = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("clean compilation");

    let mut predecessor = built.manifest_bytes.clone();
    predecessor[..8].copy_from_slice(b"LKJCMF00");
    let predecessor_key = ObjectKey::for_bytes(ObjectDomain::CompilationManifest, &predecessor);
    assert_eq!(
        CompilationManifest::decode(
            &predecessor,
            CompilationManifestDigest::from_bytes(predecessor_key.digest.bytes()),
        )
        .expect_err("predecessor manifest magic must reject")
        .code,
        "packed_contract"
    );

    let wrong = CompilationManifestDigest::from_bytes([7; 32]);
    assert_eq!(
        CompilationManifest::decode(&built.manifest_bytes, wrong)
            .expect_err("wrong object digest must reject")
            .code,
        "object_digest_mismatch"
    );
}

#[test]
fn compilation_cache_never_follows_head_or_lock_symlinks() {
    use std::os::unix::fs::symlink;

    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let temporary = tempfile::tempdir().expect("cache symlink parent");
    let root = temporary.path().join("repository");
    let outside = temporary.path().join("outside");
    let sentinel = b"outside cache sentinel";
    std::fs::write(&outside, sentinel).expect("outside sentinel");
    let created = GraphRepository::create(&root, &snapshot, None).expect("Graph 5 repository");
    let initial_head = created.repository.current().unwrap().head;
    let first = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("initial clean compilation");
    let cache_directory = root.join("derived/compiler");
    let current = cache_directory.join("CURRENT");
    let lock = cache_directory.join("LOCK");

    std::fs::remove_file(&current).expect("remove current cache head");
    symlink(&outside, &current).expect("symlink cache head");
    assert_eq!(
        load_current_compilation(&created.repository)
            .expect_err("symlinked cache head must reject")
            .code,
        "compilation_cache_regular_open"
    );
    let recovered = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("atomically replace cache-head symlink");
    assert_eq!(recovered.manifest_digest, first.manifest_digest);
    assert!(
        !std::fs::symlink_metadata(&current)
            .expect("replacement cache head")
            .file_type()
            .is_symlink()
    );
    assert_eq!(std::fs::read(&outside).unwrap(), sentinel);

    std::fs::remove_file(&lock).expect("remove cache lock");
    symlink(&outside, &lock).expect("symlink cache lock");
    assert_eq!(
        build_clean(
            &created.repository,
            OptimizationPolicy::DeterministicBaseline,
        )
        .expect_err("symlinked cache lock must reject")
        .code,
        "compilation_cache_lock_open"
    );
    assert_eq!(std::fs::read(&outside).unwrap(), sentinel);
    assert_eq!(created.repository.current().unwrap().head, initial_head);
}

#[test]
fn graph5_artifact_links_deterministically_and_reopens_without_graph4_modules() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let temporary = tempfile::tempdir().expect("artifact link parent");
    let root = temporary.path().join("repository");
    let created = GraphRepository::create(&root, &snapshot, None).expect("Graph 5 repository");
    let compilation = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("clean normalized compilation");

    let first = link_artifact(&created.repository, compilation.manifest_digest, &[])
        .expect("link Graph 5 artifact");
    let second = link_artifact(&created.repository, compilation.manifest_digest, &[])
        .expect("repeat Graph 5 link");
    assert_eq!(first.artifact.bytes, second.artifact.bytes);
    assert_eq!(first.artifact.bundle_digest, second.artifact.bundle_digest);
    assert_eq!(first.work.compiler_units, 11);
    assert_eq!(first.work.runtime_owners, 8);
    assert_eq!(first.work.packages, 1);
    assert!(
        !first
            .artifact
            .bytes
            .windows("MeaningModule".len())
            .any(|window| window == b"MeaningModule")
    );

    let loaded = load_artifact(&first.artifact.bytes).expect("strict artifact load");
    assert_eq!(loaded.manifest_digest, first.artifact.manifest_digest);
    assert_eq!(loaded.bundle_digest, first.artifact.bundle_digest);
    assert_eq!(
        loaded.root_package().expect("root package").package,
        snapshot.root.package_id
    );
    assert_eq!(
        loaded
            .root_package()
            .expect("root package")
            .runtime_owners
            .len(),
        8
    );

    drop(created);
    let reopened = GraphRepository::open(&root).expect("reopen artifact repository");
    let after_restart = link_artifact(&reopened, compilation.manifest_digest, &[])
        .expect("link after repository restart");
    assert_eq!(after_restart.artifact.bytes, first.artifact.bytes);
}

#[test]
fn graph5_artifact_links_exact_compiled_dependency_closure() {
    let temporary = tempfile::tempdir().expect("dependency artifact parent");
    let source_snapshot = crate::platform::kernel::tests::witness_snapshot();
    let source = GraphRepository::create(&temporary.path().join("source"), &source_snapshot, None)
        .expect("source Graph 5 repository");
    let source_compilation = build_clean(
        &source.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("source clean compilation");
    let source_artifact =
        link_artifact(&source.repository, source_compilation.manifest_digest, &[])
            .expect("source artifact");
    let source_loaded =
        load_artifact(&source_artifact.artifact.bytes).expect("source strict artifact");
    let exported = source
        .repository
        .export_package_object()
        .expect("source package object");

    let target_snapshot = structurally_empty_snapshot(b"artifact-dependency-target");
    let target = GraphRepository::create(&temporary.path().join("target"), &target_snapshot, None)
        .expect("target Graph 5 repository");
    target
        .repository
        .stage_package_object(exported.digest, &exported.packs)
        .expect("stage exact source interface");
    let request = AuthoredChangeSet {
        base: target.current.head.revision,
        preconditions: Vec::new(),
        changes: vec![AuthoredChange::AddDependency {
            package: exported.object.package,
            semantic_revision: exported.object.semantic_revision,
            package_object: exported.digest,
        }],
        budget: ChangeBudget::default(),
    };
    let prepared = target
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .expect("prepare target dependency");
    assert!(matches!(
        target
            .repository
            .publish(&prepared.publication)
            .expect("publish target dependency"),
        PublicationOutcome::Accepted { .. }
    ));
    let target_compilation = build_clean(
        &target.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("target clean compilation");
    assert_eq!(
        link_artifact(&target.repository, target_compilation.manifest_digest, &[])
            .expect_err("missing dependency artifact must reject")
            .code,
        "artifact_link_dependency_missing"
    );
    let linked = link_artifact(
        &target.repository,
        target_compilation.manifest_digest,
        std::slice::from_ref(&source_loaded),
    )
    .expect("link exact dependency closure");
    assert_eq!(linked.work.dependency_artifacts, 1);
    assert_eq!(linked.work.packages, 2);
    let loaded = load_artifact(&linked.artifact.bytes).expect("load linked dependency artifact");
    assert_eq!(
        loaded.manifest.root_package,
        target_snapshot.root.package_id
    );
    assert!(loaded.package(source_snapshot.root.package_id).is_some());
    assert!(loaded.package(target_snapshot.root.package_id).is_some());

    let source_reference_root = loaded
        .package(source_snapshot.root.package_id)
        .expect("source artifact package")
        .reference_owners;
    let foreign_entry = artifact_map_entries(&loaded, source_reference_root)
        .into_iter()
        .next()
        .expect("source reference-execution owner");
    let mut foreign_objects = loaded.objects.clone();
    let mut foreign_manifest = loaded.manifest.clone();
    let target_package = foreign_manifest
        .packages
        .iter_mut()
        .find(|package| package.package == target_snapshot.root.package_id)
        .expect("target artifact package");
    let mut target_entries = artifact_map_entries(&loaded, target_package.reference_owners);
    target_entries.push(foreign_entry);
    target_entries.sort_by(|left, right| left.0.cmp(&right.0));
    target_package.reference_owners = replace_artifact_map(&mut foreign_objects, target_entries);
    let (closure, count, bytes) = super::artifact::closure_facts(&foreign_objects).unwrap();
    foreign_manifest.closure = closure;
    foreign_manifest.object_count = count;
    foreign_manifest.object_bytes = bytes;
    assert_eq!(
        super::artifact::encode_artifact(foreign_manifest, &foreign_objects)
            .expect_err("foreign package reference owner must reject")
            .code,
        "artifact_reference_owner_unreachable"
    );

    let unrelated_snapshot = structurally_empty_snapshot(b"artifact-unrelated-package");
    let unrelated = GraphRepository::create(
        &temporary.path().join("unrelated"),
        &unrelated_snapshot,
        None,
    )
    .expect("unrelated Graph 5 repository");
    let unrelated_compilation = build_clean(
        &unrelated.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("unrelated clean compilation");
    let unrelated_artifact = link_artifact(
        &unrelated.repository,
        unrelated_compilation.manifest_digest,
        &[],
    )
    .expect("unrelated artifact");
    let unrelated_loaded =
        load_artifact(&unrelated_artifact.artifact.bytes).expect("unrelated strict artifact");
    assert_eq!(
        link_artifact(
            &target.repository,
            target_compilation.manifest_digest,
            &[source_loaded, unrelated_loaded],
        )
        .expect_err("unrelated dependency package must not enter the closure")
        .code,
        "artifact_package_closure"
    );
}

#[test]
fn graph5_artifact_rejects_predecessor_corruption_and_inexact_closures() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let temporary = tempfile::tempdir().expect("artifact rejection parent");
    let created = GraphRepository::create(&temporary.path().join("repository"), &snapshot, None)
        .expect("Graph 5 repository");
    let compilation = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("clean normalized compilation");
    let linked = link_artifact(&created.repository, compilation.manifest_digest, &[])
        .expect("link current artifact");
    let loaded = load_artifact(&linked.artifact.bytes).expect("load current artifact");

    let mut predecessor = linked.artifact.bytes.clone();
    predecessor[..8].copy_from_slice(b"LKJART05");
    assert_eq!(
        load_artifact(&predecessor)
            .expect_err("predecessor bundle must reject")
            .code,
        "artifact_bundle_contract"
    );

    let mut wrong_manifest = linked.artifact.bytes.clone();
    wrong_manifest[28] ^= 0x80;
    assert_eq!(
        load_artifact(&wrong_manifest)
            .expect_err("wrong manifest digest must reject")
            .code,
        "object_digest_mismatch"
    );

    let mut bad_checksum = linked.artifact.bytes.clone();
    let checksum_offset = bad_checksum.len() - 40;
    bad_checksum[checksum_offset] ^= 0x01;
    assert_eq!(
        load_artifact(&bad_checksum)
            .expect_err("corrupt bundle checksum must reject")
            .code,
        "artifact_bundle_checksum"
    );

    let mut missing_objects = loaded.objects.clone();
    let missing = missing_objects
        .keys()
        .find(|key| key.domain == ObjectDomain::CompilerUnit)
        .copied()
        .expect("compiler unit object");
    missing_objects.remove(&missing);
    let mut missing_manifest = loaded.manifest.clone();
    let (closure, count, bytes) = super::artifact::closure_facts(&missing_objects).unwrap();
    missing_manifest.closure = closure;
    missing_manifest.object_count = count;
    missing_manifest.object_bytes = bytes;
    assert_eq!(
        super::artifact::encode_artifact(missing_manifest, &missing_objects)
            .expect_err("missing reachable unit must reject")
            .code,
        "artifact_object_missing"
    );

    let mut missing_metadata = loaded.manifest.clone();
    missing_metadata.packages[0].runtime_owners.pop();
    assert_eq!(
        super::artifact::encode_artifact(missing_metadata, &loaded.objects)
            .expect_err("missing runtime metadata must reject")
            .code,
        "artifact_runtime_owner_count"
    );

    let mut missing_reference_objects = loaded.objects.clone();
    let mut missing_reference_manifest = loaded.manifest.clone();
    let reference_root = missing_reference_manifest.packages[0].reference_owners;
    let mut reference_entries = artifact_map_entries(&loaded, reference_root);
    assert!(reference_entries.pop().is_some());
    missing_reference_manifest.packages[0].reference_owners =
        replace_artifact_map(&mut missing_reference_objects, reference_entries);
    let (closure, count, bytes) =
        super::artifact::closure_facts(&missing_reference_objects).unwrap();
    missing_reference_manifest.closure = closure;
    missing_reference_manifest.object_count = count;
    missing_reference_manifest.object_bytes = bytes;
    assert_eq!(
        super::artifact::encode_artifact(missing_reference_manifest, &missing_reference_objects,)
            .expect_err("missing canonical reference owner must reject")
            .code,
        "artifact_reference_owner_missing"
    );

    let mut wrong_runtime_objects = loaded.objects.clone();
    let mut wrong_runtime_manifest = loaded.manifest.clone();
    let field_binding = wrong_runtime_manifest.packages[0]
        .runtime_owners
        .iter_mut()
        .find(|binding| matches!(binding.owner, OwnerKey::Field(_)))
        .expect("runtime field binding");
    let old_key = ObjectKey::from_digest(ObjectDomain::Owner, field_binding.object.bytes());
    let bytes = wrong_runtime_objects
        .remove(&old_key)
        .expect("runtime field owner object");
    let mut field_owner = decode_owner(
        &bytes,
        field_binding.owner,
        field_binding.kind,
        field_binding.object,
    )
    .expect("decode runtime field owner");
    let OwnerRecord::Field(field) = &mut field_owner else {
        panic!("runtime field owner kind")
    };
    field.declaration = declaration_named(&snapshot, "State");
    let (wrong_digest, wrong_bytes) =
        encode_owner(&field_owner).expect("encode wrong runtime field");
    field_binding.object = wrong_digest;
    assert!(
        wrong_runtime_objects
            .insert(
                ObjectKey::from_digest(ObjectDomain::Owner, wrong_digest.bytes()),
                wrong_bytes,
            )
            .is_none()
    );
    let (closure, count, bytes) = super::artifact::closure_facts(&wrong_runtime_objects).unwrap();
    wrong_runtime_manifest.closure = closure;
    wrong_runtime_manifest.object_count = count;
    wrong_runtime_manifest.object_bytes = bytes;
    assert_eq!(
        super::artifact::encode_artifact(wrong_runtime_manifest, &wrong_runtime_objects)
            .expect_err("runtime metadata disagreeing with compiled semantics must reject")
            .code,
        "artifact_runtime_owner_semantics"
    );

    let mut extra_objects = loaded.objects.clone();
    let child = extra_objects
        .keys()
        .find(|key| key.domain == ObjectDomain::Type)
        .map(|key| TypeObjectDigest::from_bytes(key.digest.bytes()))
        .expect("artifact type object");
    let extra = TypeObject::new(TypeForm::Function {
        parameters: vec![child, child, child],
        result: child,
    })
    .expect("valid unreachable type");
    let (extra_digest, extra_bytes) = encode_type_object(&extra).unwrap();
    let extra_key = ObjectKey::from_digest(ObjectDomain::Type, extra_digest.bytes());
    assert!(extra_objects.insert(extra_key, extra_bytes).is_none());
    let mut extra_manifest = loaded.manifest.clone();
    let (closure, count, bytes) = super::artifact::closure_facts(&extra_objects).unwrap();
    extra_manifest.closure = closure;
    extra_manifest.object_count = count;
    extra_manifest.object_bytes = bytes;
    assert_eq!(
        super::artifact::encode_artifact(extra_manifest, &extra_objects)
            .expect_err("unreachable object must reject")
            .code,
        "artifact_unreachable_object"
    );
}

trait PayloadCodes {
    fn codes(&self) -> Box<dyn Iterator<Item = &CompiledCode> + '_>;
}

impl PayloadCodes for CompilationPayload {
    fn codes(&self) -> Box<dyn Iterator<Item = &CompiledCode> + '_> {
        match self {
            Self::Function { code, .. } | Self::Constant { code, .. } => {
                Box::new(std::iter::once(code))
            }
            Self::Component { ports, .. } => Box::new(ports.iter().filter_map(|port| {
                let super::unit::CompiledPortImplementation::Expression(code) =
                    &port.implementation
                else {
                    return None;
                };
                Some(code)
            })),
            Self::Test {
                actual, expected, ..
            } => Box::new([actual, expected].into_iter()),
            Self::Record { .. }
            | Self::Variant { .. }
            | Self::Interface { .. }
            | Self::External { .. }
            | Self::Target { .. } => Box::new(std::iter::empty()),
        }
    }
}
