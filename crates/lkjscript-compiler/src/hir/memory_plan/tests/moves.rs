use super::super::*;
use super::fixtures::*;
use crate::hir;
use lkjscript_core::Result;

#[test]
fn affine_aggregate_copy_and_partial_field_move_are_rejected() -> Result<()> {
    let record = product(0, "affine-record", &[("payload", hir::Type::Bytes)]);
    let binding = hir::BindingId::new(0);
    let binding_record = hir::Binding {
        id: binding,
        name: "record".into(),
        kind: hir::BindingKind::ImmutableLocal,
        ty: hir::Type::Product(record.name.clone()),
        origin: hir::Origin::Source(origin()),
    };
    let value = product_value(&record, vec![bytes()]);
    for body in [
        expression(
            hir::Type::Product(record.name.clone()),
            hir::ExprKind::Load(hir::BindingRef {
                binding,
                storage: hir::BindingStorage::Local(0),
            }),
        ),
        expression(
            hir::Type::Bytes,
            hir::ExprKind::ProductField {
                product: record.id,
                field: 0,
                value: Box::new(expression(
                    hir::Type::Product(record.name.clone()),
                    hir::ExprKind::Load(hir::BindingRef {
                        binding,
                        storage: hir::BindingStorage::Local(0),
                    }),
                )),
            },
        ),
    ] {
        let let_body = expression(
            body.ty.clone(),
            hir::ExprKind::Let {
                bindings: vec![hir::LocalDefinition {
                    binding,
                    place: hir::PlaceId::new(0),
                    static_bytes: false,
                    slot: 0,
                    value: value.clone(),
                }],
                body: Box::new(body),
            },
        );
        let mut program = program(
            let_body.ty.clone(),
            let_body,
            vec![record.clone()],
            Vec::new(),
        );
        program.bindings.push(binding_record.clone());
        program.main.local_count = 1;
        let error = producer::derive(&program)
            .err()
            .map(|item| item.to_string())
            .unwrap_or_default();
        assert!(error.contains("AFFINE-AGGREGATE-COPY") || error.contains("PARTIAL-MOVE"));
    }
    Ok(())
}
