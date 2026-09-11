//! Canonical-source recursive instances, raw ingress, property faults, and independent bytes.
use super::super::{codec, data_codec, data_codec_reference, reference_schema, value_oracle};
use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::{CaseId, ModuleId, ParameterId, TypeParameterId};

fn fixture(
    poison: bool,
) -> (
    KernelSnapshot,
    Vec<TypeObjectDigest>,
    Vec<DeclarationReference>,
) {
    let seed = b"finite-recursive-layout-and-value-oracle";
    let mut snapshot = empty_normalized_snapshot(seed);
    let package = snapshot.root.package_id;
    let module = ModuleId::migrate(seed, 0);
    let tree = DeclarationId::migrate(seed, 0);
    let parameter = TypeParameterId::migrate(seed, 0);
    let reference = DeclarationReference {
        package,
        declaration: tree,
    };
    let mut interner = TypeObjectInterner::default();
    let unit = interner.intern(TypeForm::Unit).unwrap();
    let integer = interner.intern(TypeForm::I64).unwrap();
    let text = interner.intern(TypeForm::Text).unwrap();
    let secret = interner.intern(TypeForm::Secret).unwrap();
    let callable = interner
        .intern(TypeForm::Function {
            parameters: vec![],
            result: secret,
        })
        .unwrap();
    let t = interner
        .intern(TypeForm::TypeParameter { parameter })
        .unwrap();
    let generic = interner
        .intern(TypeForm::Applied {
            declaration: reference,
            arguments: vec![t],
        })
        .unwrap();
    let children = interner.intern(TypeForm::List { item: generic }).unwrap();
    let roots = [integer, text, secret, callable]
        .map(|argument| {
            interner
                .intern(TypeForm::Applied {
                    declaration: reference,
                    arguments: vec![argument],
                })
                .unwrap()
        })
        .to_vec();
    snapshot.owners.insert(
        OwnerKey::Module(module),
        OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name: Name::new("recursive").unwrap(),
        }),
    );
    snapshot.owners.insert(
        OwnerKey::TypeParameter(parameter),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(parameter), OwnerKind::TypeParameter),
            declaration: tree,
            name: Name::new("T").unwrap(),
            constraints: TypeParameterConstraints::None,
        }),
    );
    let mut case_ids: Vec<_> = (0..3).map(|index| CaseId::migrate(seed, index)).collect();
    case_ids.sort();
    let mut cases = Vec::new();
    for (index, (name, payload)) in [("leaf", t), ("branch", children), ("forbidden", secret)]
        .into_iter()
        .enumerate()
    {
        if index == 2 && !poison {
            continue;
        }
        let case = case_ids[index];
        cases.push(case);
        snapshot.owners.insert(
            OwnerKey::Case(case),
            OwnerRecord::Case(CaseRecord {
                header: OwnerHeader::new(OwnerKey::Case(case), OwnerKind::Case),
                declaration: tree,
                name: Name::new(name).unwrap(),
                payload: Some(payload),
            }),
        );
    }
    snapshot.owners.insert(
        OwnerKey::Declaration(tree),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(tree), OwnerKind::Variant),
            module,
            name: Name::new("tree").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Variant {
                type_parameters: vec![parameter],
                cases,
            },
        }),
    );
    let mut entries = Vec::new();
    for (index, ty) in roots.iter().enumerate() {
        let serial = index as u64 + 1;
        let function = DeclarationId::migrate(seed, serial);
        let input = ParameterId::migrate(seed, serial);
        let body = ExpressionId::migrate(seed, serial);
        snapshot.owners.insert(
            OwnerKey::Expression(body),
            OwnerRecord::Expression(
                ExpressionRecord::new(body, ExpressionOperation::Unit {}).unwrap(),
            ),
        );
        snapshot.owners.insert(
            OwnerKey::Parameter(input),
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(input), OwnerKind::Parameter),
                parent: ParameterParent::Function(function),
                name: Name::new("input").unwrap(),
                ty: *ty,
                use_mode: ParameterUse::Unrestricted,
                resource_requirement: None,
            }),
        );
        snapshot.owners.insert(
            OwnerKey::Declaration(function),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(OwnerKey::Declaration(function), OwnerKind::PureFunction),
                module,
                name: Name::new(format!("consume-{index}")).unwrap(),
                visibility: DeclarationVisibility::Private,
                payload: DeclarationPayload::Function(FunctionDeclaration {
                    effect_parameters: Vec::new(),
                    type_parameters: vec![],
                    parameters: vec![input],
                    result: unit,
                    effect: FunctionEffect::Pure,
                    body,
                }),
            }),
        );
        entries.push(DeclarationReference {
            package,
            declaration: function,
        });
    }
    snapshot.types = interner.into_objects();
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    (snapshot, roots, entries)
}

fn branch(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    children: Vec<NormalizedValue>,
) -> NormalizedValue {
    NormalizedValue::Variant {
        layout: program.variant_instances[&ty],
        case: 1,
        payload: Some(Box::new(NormalizedValue::list(children).unwrap())),
    }
}
fn leaf(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    value: NormalizedValue,
) -> NormalizedValue {
    NormalizedValue::Variant {
        layout: program.variant_instances[&ty],
        case: 0,
        payload: Some(Box::new(value)),
    }
}

#[test]
fn recursive_instances_properties_ingress_and_codec_limits_are_independent() {
    let (snapshot, roots, entries) = fixture(false);
    let program = prepare_snapshot(&snapshot);
    let reference = reference_schema::NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
    let bound = super::super::reference::BoundReferenceSchema {
        canonical: Arc::new(reference.clone()),
        value_origin: program.value_origin,
    };
    // Exactly four closed applications; open generic metadata is not a runtime instance.
    // Payload length cannot create types. The template has its separate selector index.
    assert_eq!(program.variant_instances.len(), 4);
    assert_eq!(reference.variant_instances.len(), 4);
    for (index, ty) in roots.iter().enumerate() {
        assert!(program.ordinary_types.contains(ty));
        assert!(reference.ordinary_types.contains(ty));
        assert_eq!(program.capture_safe_types.contains(ty), index != 2);
        assert_eq!(reference.capture_safe_types.contains(ty), index != 2);
        assert_eq!(program.comparable_types.contains(ty), index < 2);
        assert_eq!(reference.comparable_types.contains(ty), index < 2);
        let empty = branch(&program, *ty, vec![]);
        assert_eq!(
            codec::encode_value(&program, &empty, *ty, JsonLimits::default()).is_ok(),
            index < 2
        );
        assert_eq!(
            data_codec::encode_typed(&program, &empty, *ty).is_ok(),
            index < 2
        );
        assert_eq!(
            data_codec_reference::encode_typed(&bound, &empty, *ty).is_ok(),
            index < 2
        );
    }
    let value = branch(
        &program,
        roots[0],
        vec![
            leaf(&program, roots[0], NormalizedValue::I64(1)),
            branch(
                &program,
                roots[0],
                vec![
                    leaf(&program, roots[0], NormalizedValue::I64(2)),
                    leaf(&program, roots[0], NormalizedValue::I64(4)),
                ],
            ),
            branch(&program, roots[0], vec![]),
        ],
    );
    let bytes = data_codec::encode_typed(&program, &value, roots[0]).unwrap();
    assert_eq!(
        bytes,
        data_codec_reference::encode_typed(&bound, &value, roots[0]).unwrap()
    );
    assert_eq!(bytes, independently_encoded_tree(&snapshot, roots[0]));
    // Cancellation crosses layout metadata and separate nested values; it cannot return
    // encoded prefixes or a decoder fallback. Neither codec shares these traversal helpers.
    for after in [1, 3, 7, 14] {
        for result in [
            data_codec::encode_typed_with_control(
                &program,
                &value,
                roots[0],
                &ExecutionControl::cancel_after_checks(after),
            )
            .map(|_| ()),
            data_codec_reference::encode_typed_with_control(
                &bound,
                &value,
                roots[0],
                &ExecutionControl::cancel_after_checks(after),
            )
            .map(|_| ()),
            data_codec::decode_typed_with_control(
                &program,
                &bytes,
                roots[0],
                &ExecutionControl::cancel_after_checks(after),
            )
            .map(|_| ()),
            data_codec_reference::decode_typed_with_control(
                &bound,
                &bytes,
                roots[0],
                &ExecutionControl::cancel_after_checks(after),
            )
            .map(|_| ()),
        ] {
            assert_eq!(
                result.unwrap_err().class,
                crate::platform::DiagnosticClass::Cancelled
            );
        }
    }
    let decoded = data_codec::decode_typed(&program, &bytes, roots[0]).unwrap();
    let expected = serde_json::json!({"case":"branch","value":[{"case":"leaf","value":1},{"case":"branch","value":[{"case":"leaf","value":2},{"case":"leaf","value":4}]},{"case":"branch","value":[]}]});
    assert_eq!(
        codec::encode_value(&program, &decoded, roots[0], JsonLimits::default()).unwrap(),
        expected
    );
    for after in [1, 3, 7, 14] {
        for result in [
            codec::decode_value_with_control(
                &program,
                &expected,
                roots[0],
                JsonLimits::default(),
                &ExecutionControl::cancel_after_checks(after),
            )
            .map(|_| ()),
            codec::encode_value_with_control(
                &program,
                &value,
                roots[0],
                JsonLimits::default(),
                &ExecutionControl::cancel_after_checks(after),
            )
            .map(|_| ()),
        ] {
            assert_eq!(
                result.unwrap_err().class,
                crate::platform::DiagnosticClass::Cancelled
            );
        }
    }
    assert!(data_codec::decode_typed(&program, &bytes, roots[1]).is_err());
    let control = ExecutionControl::uncancelled();
    for bad in [
        branch(
            &program,
            roots[0],
            vec![
                leaf(&program, roots[0], NormalizedValue::I64(1)),
                leaf(
                    &program,
                    roots[0],
                    NormalizedValue::text("wrong second value"),
                ),
            ],
        ),
        branch(&program, roots[0], vec![branch(&program, roots[1], vec![])]),
    ] {
        let observed = Mutex::new(None);
        assert!(
            NormalizedVm::new(&program, Default::default())
                .observing(&observed, &super::super::vm::CoreNormalizedHost)
                .invoke(entries[0], vec![bad.clone()], None, &control)
                .is_err()
        );
        assert_eq!(observed.into_inner().unwrap().unwrap().calls, 0);
        assert!(
            NormalizedReferenceInterpreter::new(&snapshot, &program, Default::default())
                .invoke(entries[0], vec![bad.clone()], None, &control)
                .is_err()
        );
        assert!(
            value_oracle::inspect(&reference, program.value_origin, &bad, roots[0], &control)
                .is_err()
        );
    }
    let foreign = prepare_snapshot(&snapshot);
    assert!(
        NormalizedVm::new(&foreign, Default::default())
            .invoke(entries[0], vec![value], None, &control)
            .is_err()
    );
    let mut deep = leaf(&program, roots[0], NormalizedValue::I64(1));
    for _ in 0..63 {
        deep = branch(&program, roots[0], vec![deep]);
    }
    data_codec::encode_typed(&program, &deep, roots[0]).unwrap();
    let excessive = branch(&program, roots[0], vec![deep]);
    assert_eq!(
        data_codec::encode_typed(&program, &excessive, roots[0])
            .unwrap_err()
            .class,
        crate::platform::DiagnosticClass::Resource
    );
    assert!(data_codec_reference::encode_typed(&bound, &excessive, roots[0]).is_err());
    for after in [1, program.work.type_derivation_steps / 2] {
        assert_eq!(
            NormalizedProgram::prepare_with_control(
                program.artifact().clone(),
                &ExecutionControl::cancel_after_checks(after)
            )
            .unwrap_err()
            .class,
            crate::platform::DiagnosticClass::Cancelled
        );
        assert_eq!(
            reference_schema::NormalizedReferenceSchema::reconstruct_with_control(
                [&snapshot],
                &ExecutionControl::cancel_after_checks(after)
            )
            .unwrap_err()
            .class,
            ExecutionFailureClass::Cancelled
        );
    }
    let (poisoned, poisoned_roots, _) = fixture(true);
    let poisoned_program = prepare_snapshot(&poisoned);
    let poisoned_reference =
        reference_schema::NormalizedReferenceSchema::reconstruct([&poisoned]).unwrap();
    assert!(
        !poisoned_program
            .capture_safe_types
            .contains(&poisoned_roots[0])
    );
    assert!(
        !poisoned_reference
            .capture_safe_types
            .contains(&poisoned_roots[0])
    );
    assert!(
        !poisoned_program
            .comparable_types
            .contains(&poisoned_roots[0])
    );
    assert!(
        data_codec::encode_typed(
            &poisoned_program,
            &branch(&poisoned_program, poisoned_roots[0], vec![]),
            poisoned_roots[0]
        )
        .is_err()
    );
    println!(
        "recursive instances={} production-steps={} production-bytes={} reference-steps={} reference-bytes={} raw-second-value=checked forbidden-sibling=checked codec-depth=127/129",
        program.variant_instances.len(),
        program.work.type_derivation_steps,
        program.work.type_metadata_bytes,
        reference.type_derivation_steps,
        reference.type_metadata_bytes
    );
}

#[test]
fn recursive_decoder_operational_failures_cannot_select_fallback_in_either_tier() {
    let (mut snapshot, roots, _) = fixture(false);
    let module = snapshot
        .owners
        .keys()
        .find_map(|owner| {
            if let OwnerKey::Module(id) = owner {
                Some(*id)
            } else {
                None
            }
        })
        .unwrap();
    let mut intern = |form| {
        let object = TypeObject::new(form).unwrap();
        let (ty, _) = encode_type_object(&object).unwrap();
        snapshot.types.insert(ty, object);
        ty
    };
    let bytes = intern(TypeForm::Bytes);
    let text = intern(TypeForm::Text);
    let boolean = intern(TypeForm::Bool);
    let json_result = intern(TypeForm::StructuralRecord {
        fields: vec![
            StructuralTypeField {
                name: Name::new("error").unwrap(),
                ty: text,
            },
            StructuralTypeField {
                name: Name::new("valid").unwrap(),
                ty: boolean,
            },
            StructuralTypeField {
                name: Name::new("value").unwrap(),
                ty: roots[0],
            },
        ],
    });
    let mut functions = Vec::new();
    for (index, (implementation, result)) in [
        ("core.data.decode-or", roots[0]),
        ("core.json.decode-or", json_result),
    ]
    .into_iter()
    .enumerate()
    {
        let declaration =
            DeclarationId::migrate(b"recursive-codec-failure-boundaries", index as u64);
        let mut parameters = Vec::new();
        for (position, (name, ty)) in [("bytes", bytes), ("fallback", roots[0])]
            .into_iter()
            .enumerate()
        {
            let id = ParameterId::migrate(
                b"recursive-codec-failure-boundaries",
                (index * 2 + position) as u64,
            );
            parameters.push(id);
            snapshot.owners.insert(
                OwnerKey::Parameter(id),
                OwnerRecord::Parameter(ParameterRecord {
                    header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
                    parent: ParameterParent::Function(declaration),
                    name: Name::new(name).unwrap(),
                    ty,
                    use_mode: ParameterUse::Unrestricted,
                    resource_requirement: None,
                }),
            );
        }
        snapshot.owners.insert(
            OwnerKey::Declaration(declaration),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::External),
                module,
                name: Name::new(format!("decode-{index}")).unwrap(),
                visibility: DeclarationVisibility::Private,
                payload: DeclarationPayload::External(ExternalDeclaration {
                    type_parameters: vec![],
                    parameters,
                    result,
                    implementation: ImplementationName::new(implementation).unwrap(),
                }),
            }),
        );
        functions.push(DeclarationReference {
            package: snapshot.root.package_id,
            declaration,
        });
    }
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    let program = prepare_snapshot(&snapshot);
    let value = branch(
        &program,
        roots[0],
        (0..1000)
            .map(|n| leaf(&program, roots[0], NormalizedValue::I64(n)))
            .collect(),
    );
    let fallback = branch(&program, roots[0], vec![]);
    let encoded = [
        data_codec::encode_typed(&program, &value, roots[0]).unwrap(),
        codec::encode_typed(&program, &value, roots[0], Default::default()).unwrap(),
    ];
    for (index, function) in functions.into_iter().enumerate() {
        for raw_host in [false, true] {
            for exhausted in [false, true] {
                // The JSON parser classifies oversized source as rejected input, represented
                // by valid=false. Typed-data byte admission is an operational Resource error.
                if exhausted && index == 1 {
                    continue;
                }
                let bytes = if exhausted {
                    vec![0; 4 * 1_048_576 + 1]
                } else {
                    encoded[index].clone()
                };
                let control = if exhausted {
                    ExecutionControl::uncancelled()
                } else {
                    ExecutionControl::cancel_after_checks(100)
                };
                let expected = if exhausted {
                    ExecutionFailureClass::Resource
                } else {
                    ExecutionFailureClass::Cancelled
                };
                let observed = Mutex::new(None);
                let evaluator = NormalizedVm::new(&program, Default::default());
                let evaluator = if raw_host {
                    evaluator.observing(&observed, &super::super::vm::CoreNormalizedHost)
                } else {
                    evaluator.observing_checked(&observed)
                };
                let error = evaluator
                    .invoke(
                        function,
                        vec![NormalizedValue::bytes(bytes.clone()), fallback.clone()],
                        None,
                        &control,
                    )
                    .unwrap_err();
                assert_eq!(
                    error.class, expected,
                    "production {index} raw={raw_host}: {error:?}"
                );
                assert_eq!(
                    observed.into_inner().unwrap().unwrap().external_calls,
                    1,
                    "failure must occur inside the decoder"
                );
                let control = if exhausted {
                    ExecutionControl::uncancelled()
                } else {
                    ExecutionControl::cancel_after_checks(100)
                };
                let observed = Mutex::new(None);
                let evaluator =
                    NormalizedReferenceInterpreter::new(&snapshot, &program, Default::default());
                let evaluator = if raw_host {
                    evaluator.observing(
                        &observed,
                        &super::super::reference::CoreNormalizedReferenceHost,
                    )
                } else {
                    evaluator.observing_checked(&observed)
                };
                let error = evaluator
                    .invoke(
                        function,
                        vec![NormalizedValue::bytes(bytes), fallback.clone()],
                        None,
                        &control,
                    )
                    .unwrap_err();
                assert_eq!(
                    error.class, expected,
                    "reference {index} raw={raw_host}: {error:?}"
                );
                assert_eq!(
                    observed.into_inner().unwrap().unwrap().external_calls,
                    1,
                    "reference failure must occur inside the decoder"
                );
            }
        }
    }
}

// Handwritten grammar for this exact two-case fixture, with its active recursive reference.
// It does not traverse a prepared layout or invoke either codec's layout/payload helpers.
fn independently_encoded_tree(snapshot: &KernelSnapshot, tree: TypeObjectDigest) -> Vec<u8> {
    fn blob(out: &mut Vec<u8>, text: String) {
        out.extend_from_slice(&(text.len() as u32).to_be_bytes());
        out.extend_from_slice(text.as_bytes());
    }
    fn hash(domain: &str, bytes: &[u8]) -> [u8; 32] {
        let mut h = blake3::Hasher::new_derive_key(domain);
        h.update(&(bytes.len() as u64).to_be_bytes());
        h.update(bytes);
        *h.finalize().as_bytes()
    }
    let TypeForm::Applied {
        declaration,
        arguments,
    } = &snapshot.types[&tree].form
    else {
        panic!("application");
    };
    let integer = arguments[0];
    let list = encode_type_object(&TypeObject::new(TypeForm::List { item: tree }).unwrap())
        .unwrap()
        .0;
    let mut header = tree.bytes().to_vec();
    header.push(9);
    header.extend_from_slice(&1_u32.to_be_bytes());
    header.extend_from_slice(&integer.bytes());
    header.push(2);
    blob(&mut header, declaration.package.to_string());
    blob(&mut header, declaration.declaration.to_string());
    let mut description = header.clone();
    description.extend_from_slice(&[1, 1]);
    description.extend_from_slice(&2_u32.to_be_bytes());
    let OwnerRecord::Declaration(record) =
        &snapshot.owners[&OwnerKey::Declaration(declaration.declaration)]
    else {
        panic!("owner");
    };
    let DeclarationPayload::Variant { cases, .. } = &record.payload else {
        panic!("variant");
    };
    for (index, name) in ["leaf", "branch"].into_iter().enumerate() {
        blob(&mut description, declaration.package.to_string());
        blob(&mut description, cases[index].to_string());
        blob(&mut description, name.into());
        description.push(1);
        if index == 0 {
            description.extend_from_slice(&integer.bytes());
            description.push(2);
        } else {
            description.extend_from_slice(&list.bytes());
            description.push(7);
            description.extend_from_slice(&header);
            description.push(0);
        }
    }
    let mut output = b"LKJDVAL1".to_vec();
    output.extend_from_slice(&1_u16.to_be_bytes());
    output.extend_from_slice(&hash("lkjscript.data.typed-layout.v1", &description));
    for (case, count_or_value) in [(1, 3_i64), (0, 1), (1, 2), (0, 2), (0, 4), (1, 0)] {
        output.extend_from_slice(&(case as u32).to_be_bytes());
        if case == 1 {
            output.extend_from_slice(&(count_or_value as u32).to_be_bytes());
        } else {
            output.extend_from_slice(&count_or_value.to_be_bytes());
        }
    }
    output.extend_from_slice(&hash("lkjscript.data.typed-value-envelope.v1", &output));
    output
}
