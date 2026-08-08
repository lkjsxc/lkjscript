use super::super::*;
use super::fixtures::*;
use crate::hir;
use lkjscript_core::{Error, Result};

fn shared_program() -> hir::Program {
    let payload = product(
        0,
        "sealed-payload",
        &[
            ("a", hir::Type::Str),
            ("b", hir::Type::Str),
            ("c", hir::Type::Str),
            ("d", hir::Type::Str),
            ("e", hir::Type::Str),
            ("f", hir::Type::Str),
            ("g", hir::Type::Str),
            ("h", hir::Type::Str),
        ],
    );
    let ty = hir::Type::Product(payload.name.clone());
    let left = hir::BindingId::new(0);
    let right = hir::BindingId::new(1);
    let function = hir::BindingId::new(2);
    let local = hir::BindingId::new(3);
    let bindings = vec![
        binding(left, "left", hir::BindingKind::Parameter, ty.clone()),
        binding(right, "right", hir::BindingKind::Parameter, ty.clone()),
        binding(
            function,
            "select",
            hir::BindingKind::Function,
            hir::Type::Fn {
                params: vec![ty.clone(), ty.clone()],
                ret: Box::new(hir::Type::Unit),
            },
        ),
        binding(
            local,
            "payload",
            hir::BindingKind::ImmutableLocal,
            ty.clone(),
        ),
    ];
    let function_definition = hir::Function {
        binding: function,
        origin: hir::Origin::Source(origin()),
        params: vec![left, right],
        param_places: vec![hir::PlaceId::new(0), hir::PlaceId::new(1)],
        bounds: Vec::new(),
        arity: 2,
        local_count: 0,
        summary: hir::EffectSet::PURE,
        body: unit(),
    };
    let load = || {
        expression(
            ty.clone(),
            hir::ExprKind::Load(hir::BindingRef {
                binding: local,
                storage: hir::BindingStorage::Local(0),
            }),
        )
    };
    let call = expression(
        hir::Type::Unit,
        hir::ExprKind::Call {
            callee: hir::BindingRef {
                binding: function,
                storage: hir::BindingStorage::Function,
            },
            args: vec![load(), load()],
            instantiation: None,
        },
    );
    let value = product_value(&payload, (0..8).map(|_| text("payload")).collect());
    let main = expression(
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
        products: vec![payload],
        enums: Vec::new(),
        traits: Vec::new(),
        implementations: Vec::new(),
        match_plans: Vec::new(),
        functions: vec![function_definition],
        main: hir::Main {
            origin: hir::Origin::Source(origin()),
            params: Vec::new(),
            param_places: Vec::new(),
            param_types: Vec::new(),
            return_type: hir::Type::Unit,
            arity: 0,
            local_count: 1,
            body: main,
        },
        global_layout: vec![function],
    }
}

fn binding(id: hir::BindingId, name: &str, kind: hir::BindingKind, ty: hir::Type) -> hir::Binding {
    hir::Binding {
        id,
        name: name.into(),
        kind,
        ty,
        origin: hir::Origin::Source(origin()),
    }
}

#[test]
fn compiler_selects_sealed_per_value_and_verifier_rejects_mutation() -> Result<()> {
    let program = shared_program();
    let plan = derive(&program)?;
    let sealed: Vec<_> = plan
        .value_placements
        .iter()
        .filter(|placement| placement.storage == MemoryDomain::SealedRegion)
        .collect();
    assert!(sealed.iter().any(|placement| {
        placement.route == MemoryValueRoute::SealedShare
            && placement.independently_live_owners == 2
            && placement.structural_nodes >= 8
    }));
    assert!(sealed
        .iter()
        .any(|placement| placement.route == MemoryValueRoute::LastUseMove));
    let source =
        include_str!("../../../../../lkjscript-app/tests/fixtures/sealed-placement.lkjscript");
    let compiled = crate::compile_source(source, "sealed-placement.lkjscript")?;
    let representations = &compiled.ssa().program().memory.representations;
    assert!(representations.iter().any(|item| {
        item.category == lkjscript_ir::StructuralValueCategory::Owner
            && item.storage == lkjscript_ir::StructuralStorage::UniqueStructural
    }));
    assert!(representations.iter().any(|item| {
        item.category == lkjscript_ir::StructuralValueCategory::Owner
            && item.storage == lkjscript_ir::StructuralStorage::SealedRegion
    }));
    let mut forged = plan;
    forged
        .value_placements
        .first_mut()
        .ok_or_else(|| Error::msg("placement fixture is empty"))?
        .use_count = 99;
    assert!(verify_forged(&program, &mut forged).is_err());
    Ok(())
}
