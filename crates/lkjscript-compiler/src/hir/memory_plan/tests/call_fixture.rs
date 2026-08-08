use super::fixtures::{expression, origin, unit};
use crate::hir;

pub(super) fn direct_call_program(
    parameter_ty: hir::Type,
    value: hir::Expr,
    products: Vec<hir::ProductDefinition>,
) -> hir::Program {
    let parameter = hir::BindingId::new(0);
    let function_binding = hir::BindingId::new(1);
    let local = hir::BindingId::new(2);
    let binding = |id, name: &str, kind, ty| hir::Binding {
        id,
        name: name.into(),
        kind,
        ty,
        origin: hir::Origin::Source(origin()),
    };
    let bindings = vec![
        binding(
            parameter,
            "value",
            hir::BindingKind::Parameter,
            parameter_ty.clone(),
        ),
        binding(
            function_binding,
            "observe",
            hir::BindingKind::Function,
            hir::Type::Fn {
                params: vec![parameter_ty.clone()],
                ret: Box::new(hir::Type::Unit),
            },
        ),
        binding(
            local,
            "local",
            hir::BindingKind::ImmutableLocal,
            parameter_ty.clone(),
        ),
    ];
    let function = hir::Function {
        binding: function_binding,
        origin: hir::Origin::Source(origin()),
        params: vec![parameter],
        param_places: vec![hir::PlaceId::new(0)],
        bounds: Vec::new(),
        arity: 1,
        local_count: 0,
        summary: hir::EffectSet::PURE,
        body: unit(),
    };
    let call = expression(
        hir::Type::Unit,
        hir::ExprKind::Call {
            callee: hir::BindingRef {
                binding: function_binding,
                storage: hir::BindingStorage::Function,
            },
            args: vec![expression(
                parameter_ty,
                hir::ExprKind::Load(hir::BindingRef {
                    binding: local,
                    storage: hir::BindingStorage::Local(0),
                }),
            )],
            instantiation: None,
        },
    );
    let body = expression(
        hir::Type::Unit,
        hir::ExprKind::Let {
            bindings: vec![hir::LocalDefinition {
                binding: local,
                place: hir::PlaceId::new(0),
                static_bytes: false,
                slot: 0,
                value,
            }],
            body: Box::new(call),
        },
    );
    hir::Program {
        sources: Vec::new(),
        bindings,
        products,
        enums: Vec::new(),
        traits: Vec::new(),
        implementations: Vec::new(),
        match_plans: Vec::new(),
        functions: vec![function],
        main: hir::Main {
            origin: hir::Origin::Source(origin()),
            params: Vec::new(),
            param_places: Vec::new(),
            param_types: Vec::new(),
            return_type: hir::Type::Unit,
            arity: 0,
            local_count: 1,
            body,
        },
        global_layout: vec![function_binding],
    }
}
