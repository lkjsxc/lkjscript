use super::fixtures::{expression, origin, product, product_value};
use crate::hir;

pub(super) fn generic_copy_product_program() -> hir::Program {
    let first = hir::BindingId::new(0);
    let second = hir::BindingId::new(1);
    let function_binding = hir::BindingId::new(2);
    let t = hir::Type::Param("t".into());
    let u = hir::Type::Param("u".into());
    let record = product(0, "transport-record", &[("value", hir::Type::I64)]);
    let record_ty = hir::Type::Product(record.name.clone());
    let function_ty = hir::Type::Forall {
        vars: vec!["u".into(), "t".into()],
        body: Box::new(hir::Type::Fn {
            params: vec![t.clone(), u.clone()],
            ret: Box::new(t.clone()),
        }),
    };
    let binding = |id, name: &str, kind, ty| hir::Binding {
        id,
        name: name.into(),
        kind,
        ty,
        origin: hir::Origin::Source(origin()),
    };
    let bindings = vec![
        binding(first, "first", hir::BindingKind::Parameter, t.clone()),
        binding(second, "second", hir::BindingKind::Parameter, u),
        binding(
            function_binding,
            "copy-product",
            hir::BindingKind::Function,
            function_ty,
        ),
    ];
    let function = hir::Function {
        binding: function_binding,
        origin: origin(),
        params: vec![first, second],
        param_places: vec![hir::PlaceId::new(0), hir::PlaceId::new(1)],
        bounds: Vec::new(),
        arity: 2,
        local_count: 0,
        summary: hir::EffectSet::PURE,
        body: expression(
            t,
            hir::ExprKind::Load(hir::BindingRef {
                binding: first,
                storage: hir::BindingStorage::Local(0),
            }),
        ),
    };
    let body = expression(
        record_ty.clone(),
        hir::ExprKind::Call {
            callee: hir::BindingRef {
                binding: function_binding,
                storage: hir::BindingStorage::Function,
            },
            args: vec![
                product_value(
                    &record,
                    vec![expression(hir::Type::I64, hir::ExprKind::LitI64(7))],
                ),
                expression(hir::Type::Bool, hir::ExprKind::LitBool(true)),
            ],
            instantiation: Some(hir::GenericInstantiation {
                substitutions: vec![
                    hir::TypeSubstitution {
                        parameter: "u".into(),
                        ty: hir::Type::Bool,
                    },
                    hir::TypeSubstitution {
                        parameter: "t".into(),
                        ty: record_ty.clone(),
                    },
                ],
                witnesses: Vec::new(),
            }),
        },
    );
    hir::Program {
        sources: Vec::new(),
        bindings,
        products: vec![record],
        enums: Vec::new(),
        traits: Vec::new(),
        implementations: Vec::new(),
        match_plans: Vec::new(),
        functions: vec![function],
        main: hir::Main {
            origin: origin(),
            params: Vec::new(),
            param_places: Vec::new(),
            param_types: Vec::new(),
            return_type: record_ty,
            arity: 0,
            local_count: 0,
            body,
        },
        global_layout: vec![function_binding],
    }
}
