use super::super::*;
use super::call_fixture::*;
use super::fixtures::*;
use crate::hir;
use lkjscript_core::{ResourceKind, Result};

#[test]
fn function_signatures_and_direct_immutable_borrows_are_exact() -> Result<()> {
    let aggregate = product(0, "borrowed-record", &[("name", hir::Type::Str)]);
    let cases = vec![
        (hir::Type::Str, text("string"), Vec::new()),
        (hir::Type::Path, fake(hir::Type::Path), Vec::new()),
        (
            hir::Type::Product(aggregate.name.clone()),
            product_value(&aggregate, vec![text("aggregate")]),
            vec![aggregate.clone()],
        ),
    ];
    for (ty, value, products) in cases {
        let program = direct_call_program(ty, value, products);
        let plan = derive(&program)?;
        assert_eq!(
            plan.functions[0].signature.parameters,
            vec![MemoryParameterMode::BorrowShared]
        );
        let call = plan
            .calls
            .iter()
            .find(|call| matches!(call.target, MemoryCallTarget::Direct(_)))
            .ok_or_else(|| lkjscript_core::Error::msg("direct call is missing"))?;
        let scope_id = call.borrow_scopes[0]
            .ok_or_else(|| lkjscript_core::Error::msg("direct borrow scope is missing"))?;
        let scope = scope_id
            .index()
            .and_then(|index| plan.borrow_scopes.get(index))
            .ok_or_else(|| lkjscript_core::Error::msg("direct borrow scope record is missing"))?;
        assert_eq!(scope.kind, MemoryBorrowKind::Shared);
        assert_eq!(scope.semantic_uses, 1);
        assert_eq!(scope.end_after, call.expression);
    }
    Ok(())
}

#[test]
fn all_parameter_modes_are_closed() -> Result<()> {
    for (ty, expected) in [
        (hir::Type::Unit, MemoryParameterMode::Copy),
        (hir::Type::ByteSlice, MemoryParameterMode::BorrowShared),
        (
            hir::Type::ByteSliceMut,
            MemoryParameterMode::BorrowExclusive,
        ),
        (hir::Type::ByteVector, MemoryParameterMode::Consume),
    ] {
        let program = direct_call_program(ty.clone(), fake(ty), Vec::new());
        let plan = derive(&program)?;
        assert_eq!(plan.functions[0].signature.parameters, vec![expected]);
    }
    let mut resource = direct_call_program(
        hir::Type::Resource(ResourceKind::FileReader),
        fake(hir::Type::Resource(ResourceKind::FileReader)),
        Vec::new(),
    );
    resource.functions[0].body = expression(
        hir::Type::Resource(ResourceKind::FileReader),
        hir::ExprKind::Move {
            place: hir::PlaceId::new(0),
            binding: hir::BindingRef {
                binding: hir::BindingId::new(0),
                storage: hir::BindingStorage::Local(0),
            },
        },
    );
    let plan = derive(&resource)?;
    assert_eq!(
        plan.functions[0].signature.parameters,
        vec![MemoryParameterMode::Consume]
    );
    Ok(())
}

#[test]
fn result_modes_are_closed_and_borrowed_results_are_rejected() -> Result<()> {
    for (ty, expected) in [
        (hir::Type::Unit, MemoryResultMode::Trivial),
        (hir::Type::Str, MemoryResultMode::Owned),
        (
            hir::Type::Resource(ResourceKind::FileReader),
            MemoryResultMode::External,
        ),
    ] {
        let plan = derive(&program(ty.clone(), fake(ty), Vec::new(), Vec::new()))?;
        assert_eq!(plan.functions[0].signature.result, expected);
        assert_ne!(
            plan.functions[0].signature.result,
            MemoryResultMode::SealedShared
        );
    }
    for ty in [hir::Type::ByteSlice, hir::Type::ByteSliceMut] {
        let error = producer::derive(&program(ty.clone(), fake(ty), Vec::new(), Vec::new()))
            .err()
            .map(|item| item.to_string())
            .unwrap_or_default();
        assert!(error.contains("LKJ-MEM-BORROWED-RESULT") && error.contains("escape"));
    }
    Ok(())
}

#[test]
fn destinations_record_field_order_active_payload_and_reverse_abort() -> Result<()> {
    let record = product(
        0,
        "destination-record",
        &[
            ("first", hir::Type::I64),
            ("second", hir::Type::Str),
            ("third", hir::Type::Bytes),
        ],
    );
    let body = product_value(
        &record,
        vec![
            expression(hir::Type::I64, hir::ExprKind::LitI64(1)),
            text("second"),
            bytes(),
        ],
    );
    let ty = hir::Type::Product(record.name.clone());
    let plan = derive(&program(ty, body, vec![record.clone()], Vec::new()))?;
    let destination = plan
        .destinations
        .first()
        .ok_or_else(|| lkjscript_core::Error::msg("product destination is missing"))?;
    assert_eq!(destination.field_count, 3);
    assert_eq!(destination.initialized_order, vec![0, 1, 2]);
    assert_eq!(destination.reverse_abort_cleanup, vec![2, 1, 0]);
    assert_eq!(destination.kind, MemoryDestinationKind::CutoverRequired);
    assert_eq!(
        destination.execution_cutover,
        Some(MemoryExecutionCutover::Product(record.name))
    );
    assert_eq!(
        destination
            .fields
            .iter()
            .map(|field| field.index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    Ok(())
}
