//! Preserve authentic historical inputs independently of the maintained application.
//! The fixture was exported by the predecessor executable from clean source
//! 7f90cd47e2837b3ece4f278bdc9b7a734e8640aa; no historical inventory is regenerated.
use super::*;
use crate::platform::kernel::{
    DeclarationRecord, DeclarationVisibility, FunctionDeclaration, encode_owner,
};

pub(crate) fn lkjournal_before_scoped_borrow() -> KernelSnapshot {
    let bytes = include_bytes!("../../../tests/fixtures/lkjournal-before-scoped-borrow.lkjp");
    let transport =
        "package_transport_58cdcc1ba0d872cff359b9f9db75d15d5f5006ac5b50cb5572217f14b03fae0d"
            .parse()
            .unwrap();
    let mut closure =
        crate::platform::package_transport::source::PackageContainer::decode(bytes, transport)
            .unwrap()
            .admit()
            .unwrap();
    let snapshot = closure
        .packages
        .remove(&closure.container.root.package_revision)
        .unwrap()
        .snapshot;
    assert_eq!(snapshot.owners.len(), 2_040);
    snapshot
}

fn body_owners(
    snapshot: &KernelSnapshot,
    root: crate::platform::ExpressionId,
) -> BTreeSet<OwnerKey> {
    let mut result = BTreeSet::new();
    let mut pending = vec![OwnerKey::Expression(root)];
    while let Some(key) = pending.pop() {
        assert!(result.insert(key), "owned body must be a tree");
        match &snapshot.owners[&key] {
            OwnerRecord::Expression(expression) => {
                pending.extend(
                    expression
                        .children()
                        .into_iter()
                        .map(|child| OwnerKey::Expression(child.expression)),
                );
                match &expression.operation {
                    ExpressionOperation::Let { bindings, .. } => {
                        pending.extend(bindings.iter().copied().map(OwnerKey::Binding));
                    }
                    ExpressionOperation::Match { arms, .. } => {
                        pending.extend(
                            arms.iter()
                                .filter_map(|arm| arm.payload_binding)
                                .map(OwnerKey::Binding),
                        );
                    }
                    _ => {}
                }
            }
            OwnerRecord::Binding(binding) => {
                pending.extend(binding.value.map(OwnerKey::Expression));
            }
            _ => panic!("only expressions and bindings belong to a function body"),
        }
    }
    result
}

fn declaration(
    snapshot: &KernelSnapshot,
    id: crate::platform::DeclarationId,
) -> (&DeclarationRecord, &FunctionDeclaration) {
    let OwnerRecord::Declaration(record) = &snapshot.owners[&OwnerKey::Declaration(id)] else {
        panic!("declaration");
    };
    let DeclarationPayload::Function(function) = &record.payload else {
        panic!("function");
    };
    (record, function)
}

#[test]
fn scoped_borrow_adoption_preserves_every_unrelated_owner_type_and_retirement() {
    use crate::platform::kernel::ParameterUse;
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal");
    let before_head = std::fs::read(path.join("HEAD")).unwrap();
    let old = lkjournal_before_scoped_borrow();
    let current = GraphRepository::open(&path)
        .unwrap()
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    assert_eq!(current.root.package_id, old.root.package_id);
    assert_eq!(current.root.repository_id, old.root.repository_id);
    assert_eq!(current.types, old.types);
    assert_eq!(current.dependency_types, old.dependency_types);
    let owner = "decl_7f443401f4946c55fa239c5430e8ad93".parse().unwrap();
    let borrower = "decl_08eec4f6b013dea79cfed578f85b7db9".parse().unwrap();
    let (old_record, old_function) = declaration(&old, owner);
    let (new_record, new_function) = declaration(&current, owner);
    let mut restored = new_record.clone();
    let DeclarationPayload::Function(ref mut function) = restored.payload else {
        panic!("function");
    };
    function.body = old_function.body;
    assert_eq!(
        restored, *old_record,
        "owner signature and identities must remain exact"
    );
    let old_body = body_owners(&old, old_function.body);
    let new_body = body_owners(&current, new_function.body);
    assert_eq!(old_body.len(), 36);
    assert_eq!(new_body.len(), 35);
    for (key, record) in &old.owners {
        if *key != OwnerKey::Declaration(owner) && !old_body.contains(key) {
            assert_eq!(
                current.owners.get(key),
                Some(record),
                "unrelated owner {key}"
            );
            assert_eq!(
                encode_owner(&current.owners[key]).unwrap(),
                encode_owner(record).unwrap()
            );
        }
    }
    let (borrow_record, borrow_function) = declaration(&current, borrower);
    assert_eq!(borrow_record.visibility, DeclarationVisibility::Private);
    assert_eq!(borrow_function.parameters.len(), 1);
    assert_eq!(borrow_function.effect, old_function.effect);
    let OwnerRecord::Parameter(parameter) =
        &current.owners[&OwnerKey::Parameter(borrow_function.parameters[0])]
    else {
        panic!("borrow parameter");
    };
    assert_eq!(parameter.use_mode, ParameterUse::Borrow);
    let OwnerRecord::Parameter(owned_parameter) =
        &old.owners[&OwnerKey::Parameter(*old_function.parameters.last().unwrap())]
    else {
        panic!("owner parameter");
    };
    assert_eq!(owned_parameter.use_mode, ParameterUse::Consume);
    assert_eq!(parameter.ty, owned_parameter.ty);
    assert_eq!(
        parameter.resource_requirement,
        owned_parameter.resource_requirement
    );
    let mut expected_added = new_body;
    expected_added.extend(body_owners(&current, borrow_function.body));
    expected_added.insert(OwnerKey::Declaration(borrower));
    expected_added.insert(OwnerKey::Parameter(borrow_function.parameters[0]));
    let added = current
        .owners
        .keys()
        .filter(|key| !old.owners.contains_key(key))
        .copied()
        .collect::<BTreeSet<_>>();
    let retired = old
        .owners
        .keys()
        .filter(|key| !current.owners.contains_key(key))
        .copied()
        .collect::<BTreeSet<_>>();
    assert_eq!(added, expected_added);
    assert_eq!(added.len(), 40);
    assert_eq!(retired, old_body);
    for (key, retirement) in &old.retirements {
        assert_eq!(
            current.retirements.get(key),
            Some(retirement),
            "historical retirement {key}"
        );
    }
    assert_eq!(
        current
            .retirements
            .keys()
            .filter(|key| !old.retirements.contains_key(key))
            .copied()
            .collect::<BTreeSet<_>>(),
        retired
    );
    assert_eq!(current.owners.len(), 2_044);
    assert_eq!(std::fs::read(path.join("HEAD")).unwrap(), before_head);
}
