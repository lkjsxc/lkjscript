use super::*;

#[test]
fn trait_and_impl_metadata_follow_source_closure_and_declaration_order() {
    let dependency = format!(
        "{}{}{}",
        marker_trait("first"),
        POINT_PRODUCT,
        marker_impl("first", "point")
    );
    let root = format!("{}{}", marker_trait("second"), main_source("unit", "unit"));
    let first = analyze_program(
        &parsed_program(&[("dep.lkjscript", &dependency), ("root.lkjscript", &root)])
            .expect("parse closure"),
    )
    .expect("analyze closure");
    let second = analyze_program(
        &parsed_program(&[("dep.lkjscript", &dependency), ("root.lkjscript", &root)])
            .expect("parse closure again"),
    )
    .expect("analyze closure again");
    let facts = |program: &crate::hir::Program| {
        (
            program
                .traits
                .iter()
                .map(|definition| (definition.id.raw(), definition.name.clone()))
                .collect::<Vec<_>>(),
            program
                .implementations
                .iter()
                .map(|implementation| {
                    (
                        implementation.id.raw(),
                        implementation.trait_id.raw(),
                        implementation.product.raw(),
                    )
                })
                .collect::<Vec<_>>(),
        )
    };
    assert_eq!(facts(&first), facts(&second));
    assert_eq!(first.traits[CoreTrait::ALL.len()].name, "first");
    assert_eq!(first.traits[CoreTrait::ALL.len() + 1].name, "second");
}

#[test]
fn generic_and_equality_operation_types_remain_exact() {
    let source = main_source(
        "bool",
        "equal-list/\nlist-prepend/\n1\nempty-list/\ni64\n/empty-list\n/list-prepend\nlist-prepend/\n1\nempty-list/\ni64\n/empty-list\n/list-prepend\n/equal-list",
    );
    let program = analyze_one(&source).expect("analyze exact list equality");
    let ExprKind::Operation {
        operation,
        resolved_signature,
        ..
    } = &program.main.body.kind
    else {
        panic!("expected resolved operation");
    };
    assert_eq!(*operation, Operation::ListEqual);
    assert_eq!(program.main.body.ty, Type::Bool);
    assert_eq!(
        resolved_signature,
        &Type::Fn {
            params: vec![
                Type::List(Box::new(Type::I64)),
                Type::List(Box::new(Type::I64)),
            ],
            ret: Box::new(Type::Bool),
        }
    );
    assert!(program.main.body.effects != EffectSet::PURE);
}
