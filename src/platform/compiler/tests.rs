//! Focused normalized compiler-unit tests.

use super::*;
use crate::platform::kernel::{
    BindingKind, BindingRecord, CaseRecord, CaseReference, DeclarationVisibility,
    ExpressionOperation, ExpressionRecord, FieldSelector, LocalValueReference, MapExpressionEntry,
    MatchExpressionArm, Name, OwnerHeader, OwnerKey, OwnerKind, OwnerRecord, RequirementReference,
    TextValue, TypeForm, TypeObject,
};
use crate::platform::persistent_map::MapRoot;
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::{BindingId, CaseId, ExpressionId};
use crate::platform::storage::object::{ObjectDomain, ObjectKey};
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

fn complete_expression_snapshot() -> crate::platform::kernel::KernelSnapshot {
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
    snapshot.root.owners =
        MapRoot::from_parts(snapshot.root.owners.page(), snapshot.owners.len() as u64);
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
