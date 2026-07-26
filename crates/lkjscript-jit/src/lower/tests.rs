use super::{lower_type, LayoutInterner, LoweringError, LoweringFailureCode, SsaType};

#[test]
fn nested_layout_interner_is_injective_for_previous_result_tag_collision() {
    let first = SsaType::Result(
        Box::new(SsaType::Product(lkjscript_ir::ProductId::new(11))),
        Box::new(SsaType::Product(lkjscript_ir::ProductId::new(0))),
    );
    let second = SsaType::Result(
        Box::new(SsaType::Product(lkjscript_ir::ProductId::new(19))),
        Box::new(SsaType::Unit),
    );
    let mut layouts = LayoutInterner {
        identities: std::collections::HashMap::new(),
        next: LayoutInterner::FIRST_NESTED_IDENTITY,
    };
    layouts.intern(&first).expect("first exact layout");
    layouts.intern(&second).expect("second exact layout");
    assert_ne!(layouts.identity(&first), layouts.identity(&second));
}

#[test]
fn layout_identity_exhaustion_is_structured() {
    let ty = SsaType::List(Box::new(SsaType::Unit));
    let mut layouts = LayoutInterner {
        identities: std::collections::HashMap::new(),
        next: u32::MAX,
    };
    assert!(matches!(layouts.intern(&ty), Err(LoweringError { .. })));
}

#[test]
fn concrete_enum_layouts_are_injective_and_host_substitutions_reject() {
    let id = lkjscript_ir::EnumId::new([1; 32]);
    let scalar = SsaType::Enum {
        id,
        arguments: vec![SsaType::I64],
    };
    let host = SsaType::Enum {
        id,
        arguments: vec![SsaType::Handle],
    };
    let mut layouts = LayoutInterner {
        identities: std::collections::HashMap::new(),
        next: LayoutInterner::FIRST_NESTED_IDENTITY,
    };
    layouts.intern(&scalar).expect("scalar enum layout");
    layouts.intern(&host).expect("host enum layout identity");
    assert_ne!(layouts.identity(&scalar), layouts.identity(&host));
    let error = lower_type(lkjscript_ir::FunctionId::new(0), &host, &layouts)
        .expect_err("host enum substitution rejects");
    assert_eq!(error.code(), LoweringFailureCode::UnsupportedType);
}
