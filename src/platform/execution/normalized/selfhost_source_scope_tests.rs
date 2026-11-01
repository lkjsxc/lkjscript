//! Independent canonical mutations exercise source ownership and lexical scope.
//! Encoding rebinds the raw objects; it does not validate or normalize candidates.
use super::*;
use crate::platform::semantic_id::{BindingId, ExpressionId};

fn fixture() -> KernelSnapshot {
    byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-lexical.lkjc"
    ))
    .unwrap()
}

/// Reuse the established first-initializer/self-binding negative for generated
/// compiler acceptance. This only constructs and rehashes a complete fixture.
pub(super) fn artifact_rejection_fixture() -> KernelSnapshot {
    let mut source = fixture();
    let first = binder(&source, "first");
    let value = initializer(&source, first);
    expression(&mut source, value).operation = kernel::ExpressionOperation::Local {
        value: kernel::LocalValueReference::LexicalBinding(first),
    };
    rebind_owner_inventory(&mut source);
    source
}

fn binder(source: &KernelSnapshot, name: &str) -> BindingId {
    source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Binding(record) if record.name.as_str() == name => {
                let OwnerKey::Binding(id) = record.header.owner else {
                    panic!("binding kind")
                };
                Some(id)
            }
            _ => None,
        })
        .unwrap()
}

fn initializer(source: &KernelSnapshot, binding: BindingId) -> ExpressionId {
    let OwnerRecord::Binding(record) = &source.owners[&OwnerKey::Binding(binding)] else {
        panic!("binding");
    };
    record.value.unwrap()
}

fn expression(source: &mut KernelSnapshot, id: ExpressionId) -> &mut kernel::ExpressionRecord {
    let OwnerRecord::Expression(record) = source.owners.get_mut(&OwnerKey::Expression(id)).unwrap()
    else {
        panic!("expression");
    };
    record
}

fn emitted_result(bytes: &[u8], code: i64) {
    static IMAGE: OnceLock<Vec<u8>> = OnceLock::new();
    let image = IMAGE.get_or_init(|| {
        let (_, program) = probes("source-scope-probe");
        let target = program
            .function(function("source-scope-probe", "read-syntax"))
            .unwrap()
            .0 as usize;
        let input = input::export(program, target).unwrap();
        let (source, compiler) = compiler();
        let (code, image) =
            compile_with_policy(source, compiler, &input, NormalizedRunPolicy::foreground());
        assert_eq!(code, 0);
        eprintln!(
            "selfhost source syntax self-produced native bytes={} sha256={}",
            image.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&image))
        );
        image
    });
    let output = native_filter_tests::execute_filter(image, bytes, &[], 127);
    assert_eq!(
        output.status.code(),
        Some(if code == 0 { 0 } else { 65 }),
        "self-produced source validator: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        if code == 0 {
            bytes[48..80].to_vec()
        } else {
            vec![]
        }
    );
    assert_eq!(
        output.stderr,
        if code == 0 {
            vec![]
        } else {
            format!("compiler_rejected:{code}\n").into_bytes()
        }
    );
}

fn rebind_owner_inventory(source: &mut KernelSnapshot) {
    // A neutral map commitment keeps the independent oracle's flat inventory
    // coherent. No graph validation, filtering or normalization occurs here.
    let entries = source
        .owners
        .iter()
        .map(|(key, record)| {
            let (object, _) = kernel::encode_owner(record).unwrap();
            (
                kernel::owner_map_key(*key).to_vec(),
                kernel::encode_owner_binding(&kernel::OwnerBinding {
                    kind: record.kind(),
                    object,
                }),
            )
        })
        .collect::<Vec<_>>();
    let content = crate::platform::persistent_map::MapContentRoot::from_sorted(entries).unwrap();
    source.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        source.root.owners.page(),
        source.owners.len() as u64,
        content.digest(),
    );
}

fn expect(source: &KernelSnapshot, code: i64) {
    let bytes = graph::packet(source);
    if code == 15 {
        let mut oracle = source.clone();
        rebind_owner_inventory(&mut oracle);
        let errors = kernel::validate_full(&oracle).unwrap_err();
        assert!(
            errors
                .iter()
                .all(|error| error.code != "kernel_full_map_count"),
            "{errors:?}"
        );
        assert!(
            errors
                .iter()
                .any(|error| error.class == crate::platform::DiagnosticClass::Semantic),
            "{errors:?}"
        );
        assert_eq!(
            result("read-graph", &bytes),
            (0, bytes[48..80].to_vec()),
            "adversarial source must remain endpoint-closed"
        );
    }
    assert_eq!(
        result("read-syntax", &bytes),
        (
            code,
            if code == 0 {
                bytes[48..80].to_vec()
            } else {
                vec![]
            }
        )
    );
    emitted_result(&bytes, code);
}

#[test]
fn selfhost_source_scope_admits_complete_compiler_nominal_and_recursive_sources() {
    let lexical = fixture();
    let nominal = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-nominal.lkjc"
    ))
    .unwrap();
    for source in [&compiler().0, &lexical, &nominal] {
        expect(source, 0);
        let bytes = graph::packet(source);
        let word = |at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        for (kind, start, end) in [
            ("owners", word(80), word(84)),
            ("types", word(84), word(88)),
            ("retirements", word(88), word(92)),
        ] {
            eprintln!(
                "selfhost syntax complete-inventory kind={kind} bytes={} sha256={}",
                end - start,
                crate::platform::semantic_id::encode_hex(&Sha256::digest(&bytes[start..end]))
            );
        }
        eprintln!(
            "selfhost syntax carrier owners={} types={} retirements={} bytes={} sha256={} state={}",
            source.owners.len(),
            source.types.len(),
            source.retirements.len(),
            bytes.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&bytes)),
            kernel::semantic_state_digest(source).unwrap()
        );
    }
}

#[test]
fn selfhost_source_scope_preserves_sequential_and_nested_identity_shadowing() {
    let mut source = fixture();
    let first = binder(&source, "first");
    let second = binder(&source, "second");
    let outer = binder(&source, "outer");
    let inner = binder(&source, "inner");
    for id in [first, second, outer, inner] {
        let OwnerRecord::Binding(record) = source.owners.get_mut(&OwnerKey::Binding(id)).unwrap()
        else {
            panic!("binding");
        };
        record.name = kernel::Name::new("shadow").unwrap();
    }
    // The second initializer still addresses the distinct first identity; the
    // inner body explicitly addresses both identically named nested identities.
    kernel::validate_full(&source).unwrap();
    expect(&source, 0);
    let body = source
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(record)
                if matches!(record.operation, kernel::ExpressionOperation::Local {
                value: kernel::LocalValueReference::LexicalBinding(id) } if id == second) =>
            {
                Some(record)
            }
            _ => None,
        })
        .unwrap();
    body.operation = kernel::ExpressionOperation::Local {
        value: kernel::LocalValueReference::LexicalBinding(first),
    };
    kernel::validate_full(&source).unwrap();
    expect(&source, 0);
}

#[test]
fn selfhost_source_scope_rejects_self_forward_foreign_and_escaped_locals() {
    let base = fixture();
    let first = binder(&base, "first");
    let second = binder(&base, "second");
    let inner = binder(&base, "inner");
    let value = initializer(&base, first);
    for target in [first, second, inner] {
        let mut changed = base.clone();
        expression(&mut changed, value).operation = kernel::ExpressionOperation::Local {
            value: kernel::LocalValueReference::LexicalBinding(target),
        };
        expect(&changed, 15);
    }
    let foreign = base
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Parameter(record) if record.name.as_str() == "foreign" => {
                let OwnerKey::Parameter(id) = record.header.owner else {
                    panic!("parameter")
                };
                Some(id)
            }
            _ => None,
        })
        .unwrap();
    let mut changed = base.clone();
    expression(&mut changed, value).operation = kernel::ExpressionOperation::Local {
        value: kernel::LocalValueReference::FunctionParameter(foreign),
    };
    expect(&changed, 15);
    // The final list item belongs to an otherwise unreferenced function.
    let last = base
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(kernel::ExpressionRecord {
                operation: kernel::ExpressionOperation::List { items, .. },
                ..
            }) => items.last().copied(),
            _ => None,
        })
        .unwrap();
    let mut changed = base;
    expression(&mut changed, last).operation = kernel::ExpressionOperation::Local {
        value: kernel::LocalValueReference::FunctionParameter(foreign),
    };
    expect(&changed, 15);
}

#[test]
fn selfhost_source_scope_rejects_reference_kind_confusion_and_sibling_escape() {
    let base = fixture();
    let first = binder(&base, "first");
    let second = binder(&base, "second");
    let input = base
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Parameter(record) if record.name.as_str() == "input" => {
                let OwnerKey::Parameter(id) = record.header.owner else {
                    panic!("parameter")
                };
                Some(id)
            }
            _ => None,
        })
        .unwrap();
    // Endpoint admission accepts these exact IDs. Their reference variants
    // disagree with the admitted binder category, independently of visibility.
    for (site, value) in [
        (
            initializer(&base, first),
            kernel::LocalValueReference::OperationParameter(input),
        ),
        (
            initializer(&base, second),
            kernel::LocalValueReference::MatchPayload(first),
        ),
        (
            initializer(&base, second),
            kernel::LocalValueReference::TransactionBinding(first),
        ),
    ] {
        let mut changed = base.clone();
        expression(&mut changed, site).operation = kernel::ExpressionOperation::Local { value };
        expect(&changed, 15);
    }
    let outer = binder(&base, "outer");
    let inner = binder(&base, "inner");
    let (parent, body) = base
        .owners
        .iter()
        .find_map(|(key, owner)| match owner {
            OwnerRecord::Expression(kernel::ExpressionRecord {
                operation: kernel::ExpressionOperation::Let { bindings, body },
                ..
            }) if bindings.contains(&outer) => {
                let OwnerKey::Expression(id) = key else {
                    panic!("expression")
                };
                Some((*id, *body))
            }
            _ => None,
        })
        .unwrap();
    let sequence = ExpressionId::migrate(b"selfhost-source-sibling-scope", 1);
    let escaped = ExpressionId::migrate(b"selfhost-source-sibling-scope", 2);
    let mut changed = base;
    for (id, operation) in [
        (
            sequence,
            kernel::ExpressionOperation::Sequence {
                items: vec![body, escaped],
            },
        ),
        (
            escaped,
            kernel::ExpressionOperation::Local {
                value: kernel::LocalValueReference::LexicalBinding(inner),
            },
        ),
    ] {
        changed.owners.insert(
            OwnerKey::Expression(id),
            OwnerRecord::Expression(kernel::ExpressionRecord::new(id, operation).unwrap()),
        );
    }
    let kernel::ExpressionOperation::Let { body, .. } =
        &mut expression(&mut changed, parent).operation
    else {
        panic!("let")
    };
    *body = sequence;
    // The binding belongs to the same function, but its sibling Let has ended.
    expect(&changed, 15);
}

#[test]
fn selfhost_source_scope_checks_reverse_membership_and_unlisted_parameters() {
    let base = fixture();
    let other = base
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Parameter(record) if record.name.as_str() == "foreign" => {
                Some(record.parent)
            }
            _ => None,
        })
        .unwrap();
    let input = base
        .owners
        .iter()
        .find_map(|(key, owner)| match owner {
            OwnerRecord::Parameter(record) if record.name.as_str() == "input" => Some(*key),
            _ => None,
        })
        .unwrap();
    let mut changed = base.clone();
    let OwnerRecord::Parameter(record) = changed.owners.get_mut(&input).unwrap() else {
        panic!("parameter")
    };
    record.parent = other;
    expect(&changed, 15);
    let mut changed = base;
    let owner = changed
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Declaration(record) if record.name.as_str() == "chain" => Some(record),
            _ => None,
        })
        .unwrap();
    let kernel::DeclarationPayload::Function(function) = &mut owner.payload else {
        panic!("function")
    };
    function.parameters.clear();
    expect(&changed, 15);
}

#[test]
fn selfhost_source_scope_rejects_orphan_shared_and_disconnected_cyclic_syntax() {
    let base = fixture();
    let a = ExpressionId::migrate(b"selfhost-source-syntax-adversary", 1);
    let b = ExpressionId::migrate(b"selfhost-source-syntax-adversary", 2);
    let mut orphan = base.clone();
    orphan.owners.insert(
        OwnerKey::Expression(a),
        OwnerRecord::Expression(
            kernel::ExpressionRecord::new(a, kernel::ExpressionOperation::Unit {}).unwrap(),
        ),
    );
    expect(&orphan, 15);
    for mutual in [false, true] {
        let mut cycle = base.clone();
        cycle.owners.insert(
            OwnerKey::Expression(a),
            OwnerRecord::Expression(
                kernel::ExpressionRecord::new(
                    a,
                    kernel::ExpressionOperation::Sequence {
                        items: vec![if mutual { b } else { a }],
                    },
                )
                .unwrap(),
            ),
        );
        if mutual {
            cycle.owners.insert(
                OwnerKey::Expression(b),
                OwnerRecord::Expression(
                    kernel::ExpressionRecord::new(
                        b,
                        kernel::ExpressionOperation::Sequence { items: vec![a] },
                    )
                    .unwrap(),
                ),
            );
        }
        expect(&cycle, 15);
    }
    let mut cross_parent = base.clone();
    let branches = base
        .owners
        .iter()
        .filter_map(|(key, owner)| match owner {
            OwnerRecord::Expression(kernel::ExpressionRecord {
                operation: kernel::ExpressionOperation::If { when_true, .. },
                ..
            }) => {
                let OwnerKey::Expression(id) = key else {
                    panic!("if")
                };
                Some((*id, *when_true))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let kernel::ExpressionOperation::If { when_false, .. } =
        &mut expression(&mut cross_parent, branches[1].0).operation
    else {
        panic!("if")
    };
    *when_false = branches[0].1;
    expect(&cross_parent, 15);
    let mut shared = base;
    let record = shared
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(kernel::ExpressionRecord {
                operation:
                    kernel::ExpressionOperation::If {
                        when_true,
                        when_false,
                        ..
                    },
                ..
            }) => Some((when_true, when_false)),
            _ => None,
        })
        .unwrap();
    let replaced = *record.1;
    *record.1 = *record.0;
    let OwnerRecord::Expression(removed) = shared
        .owners
        .remove(&OwnerKey::Expression(replaced))
        .unwrap()
    else {
        panic!("replaced expression")
    };
    assert!(removed.children().is_empty());
    // No orphan remains: two occurrences under one parent must still reject.
    expect(&shared, 15);
}

#[test]
fn selfhost_source_scope_work_refusal_is_distinct_and_exact_bound_passes() {
    let mut source = fixture();
    let (key, record) = source
        .owners
        .iter()
        .find(|(key, _)| matches!(key, OwnerKey::Module(_)))
        .unwrap();
    source.owners = BTreeMap::from([(*key, record.clone())]);
    source.types.clear();
    source.retirements.clear();
    let bytes = graph::packet(&source);
    for (work, code) in [(0_u32, 6), (1, 6), (2, 0), (3, 0)] {
        let mut input = work.to_le_bytes().to_vec();
        input.extend_from_slice(&bytes);
        assert_eq!(
            result("read-syntax-budget", &input),
            (
                code,
                if code == 0 {
                    bytes[48..80].to_vec()
                } else {
                    vec![]
                }
            )
        );
    }
    source.owners.clear();
    let empty = graph::packet(&source);
    let mut input = 0_u32.to_le_bytes().to_vec();
    input.extend_from_slice(&empty);
    assert_eq!(
        result("read-syntax-budget", &input),
        (0, empty[48..80].to_vec())
    );
    let mut deep = byte_buffer_tests::author_only(
        "declarations.begin\n(units (module create deep-scope (function create read (visibility public) (effect pure) (parameter create input (type I64)) (returns I64) (body (local input)))))\ndeclarations.end\n",
    ).unwrap();
    let (declaration, mut body) = deep
        .owners
        .iter()
        .find_map(|(key, owner)| match owner {
            OwnerRecord::Declaration(kernel::DeclarationRecord {
                payload: kernel::DeclarationPayload::Function(function),
                ..
            }) => Some((*key, function.body)),
            _ => None,
        })
        .unwrap();
    for ordinal in 0..6 {
        let id = ExpressionId::migrate(b"selfhost-source-deep-scope", ordinal);
        deep.owners.insert(
            OwnerKey::Expression(id),
            OwnerRecord::Expression(
                kernel::ExpressionRecord::new(
                    id,
                    kernel::ExpressionOperation::Sequence { items: vec![body] },
                )
                .unwrap(),
            ),
        );
        body = id;
    }
    let OwnerRecord::Declaration(record) = deep.owners.get_mut(&declaration).unwrap() else {
        panic!("declaration")
    };
    let kernel::DeclarationPayload::Function(function) = &mut record.payload else {
        panic!("function")
    };
    function.body = body;
    rebind_owner_inventory(&mut deep);
    kernel::validate_full(&deep).unwrap();
    assert_eq!(deep.owners.len(), 10);
    let deep_bytes = graph::packet(&deep);
    // Four 10-row jumps + a 10-row rooted scan + a 10-owner scan +
    // leaf/six Sequence ancestors/callable = 68, independently enumerated.
    for (work, code) in [(67_u32, 6), (68, 0)] {
        let mut input = work.to_le_bytes().to_vec();
        input.extend_from_slice(&deep_bytes);
        assert_eq!(
            result("read-syntax-budget", &input),
            (
                code,
                if code == 0 {
                    deep_bytes[48..80].to_vec()
                } else {
                    vec![]
                }
            )
        );
    }
    expect(&deep, 0);
    let mut unsupported = fixture();
    let cell = kernel::TypeObject::new(kernel::TypeForm::OwnedI64Cell).unwrap();
    let digest = kernel::encode_type_object(&cell).unwrap().0;
    unsupported.types.insert(digest, cell);
    expect(&unsupported, 13);
    let mut malformed = bytes;
    malformed[0] ^= 1;
    assert_eq!(result("read-syntax", &malformed), (12, vec![]));
    emitted_result(&malformed, 12);
}

#[test]
fn selfhost_source_scope_does_not_claim_type_admission_or_type_inventory_identity() {
    let mut source = fixture();
    let third = binder(&source, "third");
    let value = initializer(&source, third);
    expression(&mut source, value).operation = kernel::ExpressionOperation::Bool { value: true };
    assert!(kernel::validate_full(&source).is_err());
    expect(&source, 0);
    let before = graph::packet(&source);
    let unit = kernel::TypeObject::new(kernel::TypeForm::Unit).unwrap();
    let digest = kernel::encode_type_object(&unit).unwrap().0;
    source.types.insert(digest, unit);
    let list = kernel::TypeObject::new(kernel::TypeForm::List { item: digest }).unwrap();
    let digest = kernel::encode_type_object(&list).unwrap().0;
    source.types.insert(digest, list);
    let after = graph::packet(&source);
    assert_eq!(&before[48..80], &after[48..80]);
    assert_ne!(before, after);
    assert_ne!(Sha256::digest(&before), Sha256::digest(&after));
    expect(&source, 0);
}
