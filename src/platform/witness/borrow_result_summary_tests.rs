use super::*;
use crate::platform::kernel::{FunctionEffect, RelationKind};
use crate::platform::semantic_id::ParameterId;

#[test]
fn borrowed_result_source_is_an_interface_dependency_not_body_or_presentation() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let mut declaration = snapshot
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Declaration(record)
                if matches!(&record.payload, DeclarationPayload::Function(function)
                    if function.effect == FunctionEffect::Pure) =>
            {
                Some(record.clone())
            }
            _ => None,
        })
        .expect("pure function fixture");
    declaration.header.contract_version = crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
    // Exercise summary and relation construction directly. Full program
    // ownership validity is covered by the independent kernel admission tests.
    let first = ParameterId::from_bytes([201; 16]).unwrap();
    let second = ParameterId::from_bytes([202; 16]).unwrap();
    let DeclarationPayload::Function(function) = &mut declaration.payload else {
        unreachable!()
    };
    function.parameters = vec![first, second];
    function.result_borrow = None;
    let owner = declaration.header.owner;
    let ordinary = local_summary(owner, &OwnerRecord::Declaration(declaration.clone()), None)
        .expect("ordinary summary");
    let DeclarationPayload::Function(function) = &mut declaration.payload else {
        unreachable!()
    };
    function.result_borrow = Some(first);
    let borrowed_first = local_summary(owner, &OwnerRecord::Declaration(declaration.clone()), None)
        .expect("first-source summary");
    let DeclarationPayload::Function(function) = &mut declaration.payload else {
        unreachable!()
    };
    function.result_borrow = Some(second);
    let record = OwnerRecord::Declaration(declaration);
    let borrowed_second = local_summary(owner, &record, None).expect("second-source summary");
    assert_ne!(
        ordinary.semantic_interface,
        borrowed_first.semantic_interface
    );
    assert_ne!(
        borrowed_first.semantic_interface,
        borrowed_second.semantic_interface
    );
    assert_eq!(ordinary.implementation, borrowed_first.implementation);
    assert_eq!(
        borrowed_first.implementation,
        borrowed_second.implementation
    );
    assert_eq!(ordinary.presentation, borrowed_second.presentation);

    let package = snapshot.root.package_id;
    let relations = crate::platform::kernel::extract_owner_relations(
        package,
        owner,
        &record,
        |digest| Ok(snapshot.types.get(&digest).cloned()),
        |_, _| Ok(None),
    )
    .expect("source relation");
    let sources = relations
        .iter()
        .filter(|edge| edge.kind == RelationKind::BorrowResultSource)
        .collect::<Vec<_>>();
    assert_eq!(sources.len(), 1);
    assert_eq!(
        sources[0].source,
        RelationEndpoint::Owner(ExactOwnerKey { package, owner })
    );
    assert_eq!(
        sources[0].target,
        RelationEndpoint::Owner(ExactOwnerKey {
            package,
            owner: OwnerKey::Parameter(second),
        })
    );
    assert_eq!(
        sources[0].kind.propagation(),
        crate::platform::kernel::PropagationClass::Type
    );
}
