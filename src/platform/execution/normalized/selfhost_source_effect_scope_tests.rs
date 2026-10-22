//! Contextual R/E scope is checked at each use, after structural DAG success.
use super::*;

#[derive(Clone, Copy)]
enum BinderKind {
    Requirement,
    Effect,
}

impl BinderKind {
    fn oracle(self) -> &'static str {
        match self {
            Self::Requirement => "kernel_requirement_parameter_scope",
            Self::Effect => "kernel_effect_parameter_scope",
        }
    }
}

fn callback_key(source: &KernelSnapshot, owner: &str, parameter: &str) -> OwnerKey {
    parameter_key(
        source,
        kernel::ParameterParent::Function(declaration(source, owner)),
        parameter,
    )
}

fn isolated_row(source: &KernelSnapshot, owner: &str, kind: BinderKind) -> kernel::EffectRow {
    let kernel::FunctionEffect::Task {
        requirements,
        effect_parameters,
    } = &function(source, owner).effect
    else {
        panic!("task function")
    };
    match kind {
        BinderKind::Requirement => {
            assert!(matches!(
                requirements[0],
                kernel::RequirementOperand::Parameter(_)
            ));
            kernel::EffectRow {
                requirements: vec![requirements[0]],
                parameters: vec![],
            }
        }
        BinderKind::Effect => kernel::EffectRow {
            requirements: vec![],
            parameters: vec![effect_parameters[0]],
        },
    }
}

fn isolated_task_type(
    source: &mut KernelSnapshot,
    owner: &str,
    kind: BinderKind,
) -> TypeObjectDigest {
    let key = callback_key(source, owner, "callback");
    let original = parameter_type(source, key);
    let row = isolated_row(source, owner, kind);
    let mut form = source.types[&original].form.clone();
    let kernel::TypeForm::TaskFunction { effect, .. } = &mut form else {
        panic!("task type")
    };
    *effect = row;
    intern(source, form)
}

#[test]
fn selfhost_source_effect_scope_rejects_foreign_direct_binders_with_same_names() {
    for kind in [BinderKind::Requirement, BinderKind::Effect] {
        for target in ["first", "empty-task"] {
            let mut source = fixture();
            let row = isolated_row(&source, "second", kind);
            function_mut(&mut source, target).effect = kernel::FunctionEffect::Task {
                requirements: row.requirements,
                effect_parameters: row.parameters,
            };
            expect(&source, 17, Some(kind.oracle()));
        }
    }
}

#[test]
fn selfhost_source_effect_scope_checks_shared_descriptors_in_both_context_orders() {
    for owner in ["first", "second"] {
        for kind in [BinderKind::Requirement, BinderKind::Effect] {
            let mut source = fixture();
            let shared = isolated_task_type(&mut source, owner, kind);
            for target in ["first", "second"] {
                let key = callback_key(&source, target, "callback");
                set_parameter_type(&mut source, key, shared);
            }
            expect(&source, 17, Some(kind.oracle()));
        }
    }
}

#[test]
fn selfhost_source_effect_scope_descends_through_task_inputs_results_and_records() {
    for kind in [BinderKind::Requirement, BinderKind::Effect] {
        for through_result in [false, true] {
            let mut source = fixture();
            let foreign = isolated_task_type(&mut source, "second", kind);
            let key = callback_key(&source, "first", "callback");
            let original = parameter_type(&source, key);
            let mut form = source.types[&original].form.clone();
            let kernel::TypeForm::TaskFunction {
                parameters,
                result,
                effect,
            } = &mut form
            else {
                panic!("task type")
            };
            *effect = kernel::EffectRow::default();
            if through_result {
                *result = foreign;
            } else {
                *parameters = vec![foreign];
            }
            let nested = intern(&mut source, form);
            set_parameter_type(&mut source, key, nested);
            expect(&source, 17, Some(kind.oracle()));
        }
        let mut source = fixture();
        let foreign = isolated_task_type(&mut source, "second", kind);
        let key = callback_key(&source, "first", "nested");
        let original = parameter_type(&source, key);
        let mut form = source.types[&original].form.clone();
        let kernel::TypeForm::StructuralRecord { fields } = &mut form else {
            panic!("record")
        };
        assert_eq!(fields.len(), 1);
        fields[0].ty = foreign;
        let nested = intern(&mut source, form);
        set_parameter_type(&mut source, key, nested);
        expect(&source, 17, Some(kind.oracle()));
    }
}

#[test]
fn selfhost_source_effect_scope_checks_new_operation_and_function_result_roots() {
    for kind in [BinderKind::Requirement, BinderKind::Effect] {
        for through_result in [false, true] {
            let mut source = fixture();
            let foreign = isolated_task_type(&mut source, "first", kind);
            let tick = operation(&source, "Clock", "tick");
            if through_result {
                let OwnerRecord::Operation(record) =
                    source.owners.get_mut(&OwnerKey::Operation(tick)).unwrap()
                else {
                    panic!("operation")
                };
                record.result = foreign;
            } else {
                let key = parameter_key(&source, kernel::ParameterParent::Operation(tick), "left");
                set_parameter_type(&mut source, key, foreign);
            }
            expect(&source, 17, Some(kind.oracle()));
        }
        let mut source = fixture();
        let foreign = isolated_task_type(&mut source, "second", kind);
        function_mut(&mut source, "first").result.ty = foreign;
        expect(&source, 17, Some(kind.oracle()));
    }
}

#[test]
fn selfhost_source_effect_scope_preserves_type_parameter_checks_in_task_signatures() {
    let mut source = fixture();
    let generic = callback_key(&source, "generic-task", "callback");
    let original = parameter_type(&source, generic);
    let mut form = source.types[&original].form.clone();
    let kernel::TypeForm::TaskFunction { effect, .. } = &mut form else {
        panic!("task type")
    };
    *effect = kernel::EffectRow::default();
    let foreign = intern(&mut source, form);
    let target = callback_key(&source, "first", "callback");
    set_parameter_type(&mut source, target, foreign);
    expect(&source, 17, Some("kernel_full_type_parameter_scope"));
}

#[test]
fn selfhost_source_effect_scope_allows_unused_well_formed_mixed_binder_descriptors() {
    let mut source = fixture();
    let first = parameter_type(&source, callback_key(&source, "first", "callback"));
    let second = parameter_type(&source, callback_key(&source, "second", "callback"));
    let mut form = source.types[&first].form.clone();
    let kernel::TypeForm::TaskFunction {
        parameters, effect, ..
    } = &mut form
    else {
        panic!("task type")
    };
    *parameters = vec![first, second];
    *effect = kernel::EffectRow::default();
    let unused = intern(&mut source, form);
    assert!(
        !source
            .owners
            .values()
            .flat_map(OwnerRecord::type_roots)
            .any(|ty| ty == unused)
    );
    expect(&source, 0, None);
}

#[test]
fn selfhost_source_effect_types_enforce_depth_on_unused_input_and_result_edges() {
    // The header minimum does not fund every digest-ordered DAG settlement.
    const DEPTH_ALLOWANCE: u32 = 128_000_000;
    for (depth, code) in [(256, 0), (257, 17)] {
        let mut source = fixture();
        let scalar = source
            .types
            .iter()
            .find_map(|(digest, object)| {
                matches!(object.form, kernel::TypeForm::I64).then_some(*digest)
            })
            .unwrap();
        let mut child = scalar;
        for level in 0..depth {
            let through_result = level % 2 == 1;
            child = intern(
                &mut source,
                kernel::TypeForm::TaskFunction {
                    parameters: vec![if through_result { scalar } else { child }],
                    result: if through_result { child } else { scalar },
                    effect: kernel::EffectRow::default(),
                },
            );
        }
        assert!(
            !source
                .owners
                .values()
                .flat_map(OwnerRecord::type_roots)
                .any(|ty| ty == child)
        );
        let mut bytes = graph::packet(&source);
        let minimum = u32::from_le_bytes(bytes[100..104].try_into().unwrap());
        assert!(minimum < DEPTH_ALLOWANCE);
        bytes[100..104].copy_from_slice(&DEPTH_ALLOWANCE.to_le_bytes());
        eprintln!(
            "selfhost TaskFunction raw depth={depth} types={} bytes={} sha256={} minimum_allowance={minimum} selected_allowance={DEPTH_ALLOWANCE}",
            source.types.len(),
            bytes.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&bytes)),
        );
        let start = std::time::Instant::now();
        packet_result("read-namespace", &bytes, 0);
        // Alternation makes either omitted edge route break the over-depth proof.
        packet_result("read-type-structure", &bytes, code);
        eprintln!(
            "selfhost TaskFunction raw depth={depth} code={code} elapsed_ms={}",
            start.elapsed().as_millis(),
        );
    }
}

#[test]
fn selfhost_source_effect_scope_checks_untaken_branch_type_roots() {
    for kind in [BinderKind::Requirement, BinderKind::Effect] {
        let mut source = fixture();
        let foreign = isolated_task_type(&mut source, "second", kind);
        let body = function(&source, "branch-carrier").body;
        let OwnerRecord::Expression(record) = &source.owners[&OwnerKey::Expression(body)] else {
            panic!("branch body")
        };
        let kernel::ExpressionOperation::If {
            condition,
            when_true,
            when_false,
        } = record.operation
        else {
            panic!("branch")
        };
        assert!(matches!(&source.owners[&OwnerKey::Expression(condition)],
            OwnerRecord::Expression(record) if matches!(record.operation,
                kernel::ExpressionOperation::Bool { value: true })));
        assert!(matches!(&source.owners[&OwnerKey::Expression(when_true)],
            OwnerRecord::Expression(record) if matches!(&record.operation,
                kernel::ExpressionOperation::List { items, .. } if items.is_empty())));
        let OwnerRecord::Expression(record) = source
            .owners
            .get_mut(&OwnerKey::Expression(when_false))
            .unwrap()
        else {
            panic!("untaken branch")
        };
        let kernel::ExpressionOperation::List { item_type, items } = &mut record.operation else {
            panic!("untaken empty list")
        };
        assert!(items.is_empty());
        *item_type = foreign;
        expect(&source, 17, Some(kind.oracle()));
    }
}

#[test]
fn selfhost_source_effect_endpoints_bind_package_and_member_identity_together() {
    for kind in [BinderKind::Requirement, BinderKind::Effect] {
        for nested in [false, true] {
            let mut source = fixture();
            let mut row = isolated_row(&source, "first", kind);
            let foreign = kernel::PackageId::from_bytes([250; 16]).unwrap();
            assert_ne!(foreign, source.root.package_id);
            match kind {
                BinderKind::Requirement => {
                    let kernel::RequirementOperand::Parameter(reference) = &mut row.requirements[0]
                    else {
                        panic!("requirement parameter")
                    };
                    reference.package = foreign;
                }
                BinderKind::Effect => row.parameters[0].package = foreign,
            }
            if nested {
                let key = callback_key(&source, "first", "callback");
                let mut form = source.types[&parameter_type(&source, key)].form.clone();
                let kernel::TypeForm::TaskFunction { effect, .. } = &mut form else {
                    panic!("task type")
                };
                *effect = row;
                let digest = intern(&mut source, form);
                set_parameter_type(&mut source, key, digest);
            } else {
                function_mut(&mut source, "first").effect = kernel::FunctionEffect::Task {
                    requirements: row.requirements,
                    effect_parameters: row.parameters,
                };
            }
            expect(&source, 13, None);
        }
    }
}
