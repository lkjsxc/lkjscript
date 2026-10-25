//! Raw nominal member mutations reach parameter-slot flow after earlier admission.
//! Expectations follow finite parameter flow, independently of the emitted walk.
use super::*;
use crate::platform::semantic_id::{DeclarationId, TypeParameterId};

fn fixture() -> KernelSnapshot {
    byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-nominal-flow.lkjc"
    ))
    .unwrap()
}

fn declaration(source: &KernelSnapshot, name: &str) -> DeclarationId {
    source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Declaration(record) if record.name.as_str() == name => {
                let OwnerKey::Declaration(id) = record.header.owner else {
                    panic!("declaration kind")
                };
                Some(id)
            }
            _ => None,
        })
        .unwrap()
}

fn intern(source: &mut KernelSnapshot, form: kernel::TypeForm) -> TypeObjectDigest {
    let object = kernel::TypeObject::new(form).unwrap();
    let digest = kernel::encode_type_object(&object).unwrap().0;
    source.types.insert(digest, object);
    digest
}

fn parameter(source: &mut KernelSnapshot, owner: &str, name: &str) -> TypeObjectDigest {
    let parent = declaration(source, owner);
    let parameter: TypeParameterId = source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::TypeParameter(record)
                if record.declaration == parent && record.name.as_str() == name =>
            {
                let OwnerKey::TypeParameter(id) = record.header.owner else {
                    panic!("type parameter kind")
                };
                Some(id)
            }
            _ => None,
        })
        .unwrap();
    intern(source, kernel::TypeForm::TypeParameter { parameter })
}

fn applied(
    source: &mut KernelSnapshot,
    name: &str,
    arguments: Vec<TypeObjectDigest>,
) -> TypeObjectDigest {
    let declaration = DeclarationReference {
        package: source.root.package_id,
        declaration: declaration(source, name),
    };
    intern(
        source,
        if arguments.is_empty() {
            kernel::TypeForm::Named { declaration }
        } else {
            kernel::TypeForm::Applied {
                declaration,
                arguments,
            }
        },
    )
}

fn member(source: &mut KernelSnapshot, owner: &str, name: &str, ty: TypeObjectDigest) {
    let parent = declaration(source, owner);
    let mut changed = 0;
    for value in source.owners.values_mut() {
        match value {
            OwnerRecord::Field(record)
                if record.declaration == parent && record.name.as_str() == name =>
            {
                record.ty = ty;
                changed += 1;
            }
            OwnerRecord::Case(record)
                if record.declaration == parent && record.name.as_str() == name =>
            {
                record.payload = Some(ty);
                changed += 1;
            }
            _ => {}
        }
    }
    assert_eq!(changed, 1);
}

fn rebind_owner_inventory(source: &mut KernelSnapshot) {
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

// LKJSG002 binds every supplied descriptor, including unused ones. The Rust
// snapshot oracle instead requires exactly the types reachable from live owners.
// Project only that neutral inventory boundary; preserve all owner meaning and
// leave the raw carrier unchanged so both validators inspect the same mutation.
fn live_type_oracle(source: &KernelSnapshot) -> KernelSnapshot {
    let mut oracle = source.clone();
    rebind_owner_inventory(&mut oracle);
    let mut pending: Vec<_> = oracle
        .owners
        .values()
        .flat_map(OwnerRecord::type_roots)
        .collect();
    let state = kernel::semantic_state_digest(&oracle).unwrap();
    let mut reachable = std::collections::BTreeSet::new();
    while let Some(digest) = pending.pop() {
        if reachable.insert(digest) {
            pending.extend(oracle.types[&digest].child_types());
        }
    }
    oracle.types.retain(|digest, _| reachable.contains(digest));
    assert_eq!(oracle.owners, source.owners);
    assert_eq!(kernel::semantic_state_digest(&oracle).unwrap(), state);
    oracle
}

#[test]
fn selfhost_source_nominal_flow_separates_raw_and_live_type_inventories() {
    let base = fixture();
    let mut source = base.clone();
    let bytes = intern(&mut source, kernel::TypeForm::Bytes);
    let list = intern(&mut source, kernel::TypeForm::List { item: bytes });
    assert!(!base.types.contains_key(&list));
    let raw = graph::packet(&source);
    let original = graph::packet(&base);
    assert_ne!(raw, original);
    assert_eq!(&raw[48..80], &original[48..80]);
    let errors = kernel::validate_full(&source).unwrap_err();
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|error| {
        error.code == "kernel_full_type_unreachable"
            && error.class == crate::platform::DiagnosticClass::Semantic
    }));
    let oracle = live_type_oracle(&source);
    kernel::validate_full(&oracle).unwrap();
    assert_eq!(oracle.owners, source.owners);
    assert_eq!(oracle.types, base.types);
    assert_eq!(graph::packet(&source), raw);
    expect(&source, 0, None);
}

fn expect(source: &KernelSnapshot, code: i64, oracle_code: Option<&str>) {
    let bytes = graph::packet(source);
    if let Some(expected) = oracle_code {
        let errors = kernel::validate_full(&live_type_oracle(source)).unwrap_err();
        assert!(
            errors.iter().any(|error| {
                error.code == expected && error.class == crate::platform::DiagnosticClass::Semantic
            }),
            "{errors:?}"
        );
        assert!(
            errors.iter().all(|error| {
                error.code != "kernel_full_map_count"
                    && error.code != "kernel_full_type_unreachable"
                    && error.class != crate::platform::DiagnosticClass::Resource
            }),
            "{errors:?}"
        );
        assert_eq!(
            result("read-type-scope", &bytes),
            (0, bytes[48..80].to_vec()),
            "nominal negative must reach the slot-flow layer"
        );
    }
    assert_eq!(
        result("read-nominal", &bytes),
        (
            code,
            if code == 0 {
                bytes[48..80].to_vec()
            } else {
                vec![]
            }
        )
    );
    emitted_result("read-nominal", &bytes, code);
}

#[test]
fn selfhost_source_nominal_flow_admits_complete_compiler_and_finite_schemas() {
    let independent = fixture();
    // The independent fixture covers tree wrapping, parameter permutation,
    // duplication/selection, closed reset, and expanding acyclic slot paths.
    for source in [&compiler().0, &independent] {
        expect(source, 0, None);
        let bytes = graph::packet(source);
        eprintln!(
            "selfhost E1 nominal carrier owners={} types={} retirements={} bytes={} sha256={} state={}",
            source.owners.len(),
            source.types.len(),
            source.retirements.len(),
            bytes.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&bytes)),
            kernel::semantic_state_digest(source).unwrap()
        );
    }
    compiler_source_name_mutation(&compiler().0);
}

#[test]
fn selfhost_source_nominal_flow_rejects_expanding_self_and_mutual_slots() {
    let base = fixture();
    let mut source = base.clone();
    let t = parameter(&mut source, "Tree", "T");
    let nested = intern(&mut source, kernel::TypeForm::List { item: t });
    let grow = applied(&mut source, "Tree", vec![nested]);
    member(&mut source, "Tree", "branch", grow);
    expect(&source, 18, Some("kernel_type_nominal_expansion"));

    let mut source = base.clone();
    let a = parameter(&mut source, "Swap", "A");
    let b = parameter(&mut source, "Swap", "B");
    let nested = intern(&mut source, kernel::TypeForm::List { item: b });
    let grow = applied(&mut source, "Swap", vec![nested, a]);
    member(&mut source, "Swap", "next", grow);
    expect(&source, 18, Some("kernel_type_nominal_expansion"));

    let mut source = base.clone();
    let a = parameter(&mut source, "Dup", "A");
    let nested = intern(&mut source, kernel::TypeForm::List { item: a });
    let grow = applied(&mut source, "Select", vec![a, nested]);
    member(&mut source, "Dup", "next", grow);
    expect(&source, 18, Some("kernel_type_nominal_expansion"));

    let mut source = base;
    let a = parameter(&mut source, "Drop", "A");
    let grow = applied(&mut source, "Reset", vec![a]);
    member(&mut source, "Drop", "next", grow);
    expect(&source, 18, Some("kernel_type_nominal_expansion"));
}

#[test]
fn selfhost_source_nominal_flow_checks_signatures_phantoms_and_inactive_members() {
    let base = fixture();
    let mut source = base.clone();
    let a = parameter(&mut source, "Signature", "A");
    let nested = intern(&mut source, kernel::TypeForm::List { item: a });
    let grow = applied(&mut source, "Signature", vec![nested]);
    let atom = intern(&mut source, kernel::TypeForm::I64);
    let callback = intern(
        &mut source,
        kernel::TypeForm::Function {
            parameters: vec![grow],
            result: atom,
        },
    );
    member(&mut source, "Signature", "callback", callback);
    expect(&source, 18, Some("kernel_type_nominal_expansion"));

    let mut source = base.clone();
    let t = parameter(&mut source, "Tree", "T");
    let phantom = applied(&mut source, "Phantom", vec![t]);
    let grow = applied(&mut source, "Tree", vec![phantom]);
    member(&mut source, "Tree", "branch", grow);
    expect(&source, 18, Some("kernel_type_nominal_expansion"));

    let mut source = base;
    let t = parameter(&mut source, "Tree", "T");
    let nested = intern(&mut source, kernel::TypeForm::List { item: t });
    let grow = applied(&mut source, "Tree", vec![nested]);
    let phantom = applied(&mut source, "Phantom", vec![grow]);
    member(&mut source, "Tree", "branch", phantom);
    expect(&source, 18, Some("kernel_type_nominal_expansion"));
}

#[test]
fn selfhost_source_nominal_flow_requires_exact_ordered_argument_arity() {
    let base = fixture();
    let mut source = base.clone();
    let missing = applied(&mut source, "Tree", vec![]);
    member(&mut source, "Tree", "branch", missing);
    expect(&source, 18, Some("kernel_type_nominal_arity"));

    let mut source = base;
    let t = parameter(&mut source, "Tree", "T");
    let extra = applied(&mut source, "Tree", vec![t, t]);
    member(&mut source, "Tree", "branch", extra);
    expect(&source, 18, Some("kernel_type_nominal_arity"));
}

#[test]
fn selfhost_source_nominal_flow_preserves_resource_refusal() {
    let source = fixture();
    let bytes = graph::packet(&source);
    for work in [0_u32, 1] {
        let packet = [work.to_le_bytes().as_slice(), bytes.as_slice()].concat();
        assert_eq!(result("read-nominal-budget", &packet), (6, vec![]));
        emitted_result("read-nominal-budget", &packet, 6);
    }
    let large = [u32::MAX.to_le_bytes().as_slice(), bytes.as_slice()].concat();
    assert_eq!(result("read-nominal-budget", &large), (0, bytes[48..80].to_vec()));
    emitted_result("read-nominal-budget", &large, 0);
    expect(&source, 0, None);
    exact_budget_boundary("read-nominal-budget", &source);
}

fn compiler_source_name_mutation(source: &KernelSnapshot) {
    let mut modules = std::collections::BTreeMap::<_, (OwnerKey, kernel::Name)>::new();
    let (first, second, duplicate_name) = source
        .owners
        .iter()
        .find_map(|(key, owner)| {
            let OwnerRecord::Declaration(record) = owner else {
                return None;
            };
            if let Some((first, name)) = modules.get(&record.module) {
                Some((*first, *key, name.clone()))
            } else {
                modules.insert(record.module, (*key, record.name.clone()));
                None
            }
        })
        .unwrap();
    assert_ne!(first, second);
    let mut changed = source.clone();
    let OwnerRecord::Declaration(record) = changed.owners.get_mut(&second).unwrap() else {
        panic!("declaration kind")
    };
    assert_ne!(record.name, duplicate_name);
    record.name = duplicate_name;
    rebind_owner_inventory(&mut changed);
    let errors = kernel::validate_full(&changed).unwrap_err();
    assert!(
        errors.iter().any(|error| {
            error.code == "kernel_full_namespace_duplicate"
                && error.class == crate::platform::DiagnosticClass::Semantic
        }),
        "{errors:?}"
    );
    assert!(
        errors.iter().all(|error| {
            error.code != "kernel_full_map_count"
                && error.code != "kernel_full_type_unreachable"
                && error.class != crate::platform::DiagnosticClass::Resource
        }),
        "{errors:?}"
    );
    assert_ne!(
        kernel::semantic_state_digest(source).unwrap(),
        kernel::semantic_state_digest(&changed).unwrap()
    );
    let bytes = graph::packet(&changed);
    // Exact diagnostic 16 in the composed validator entails successful raw,
    // endpoint, containment, and value-scope admission before the namespace pass.
    // This is compiler-carrier input sensitivity, not source-to-normalized
    // compilation or an emitted compiler-stage mutation proof.
    assert_eq!(result("read-nominal", &bytes), (16, vec![]));
    emitted_result("read-nominal", &bytes, 16);
    eprintln!(
        "selfhost compiler source mutation diagnostic=16 owners={} types={} bytes={} sha256={} state={}",
        changed.owners.len(),
        changed.types.len(),
        bytes.len(),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&bytes)),
        kernel::semantic_state_digest(&changed).unwrap()
    );
}
