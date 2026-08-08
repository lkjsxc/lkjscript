use super::fixtures::{derive, expression, origin, product, product_value, program, unit};
use crate::hir;
use lkjscript_core::Result;

#[test]
fn declaration_graph_crosses_former_type_node_and_scc_work_boundaries() -> Result<()> {
    const DECLARATIONS: u64 = 32_769;
    const FORMER_TYPE_NODES: u64 = 16_384;
    const FORMER_SCC_WORK: u64 = 65_536;
    const _: () = assert!(DECLARATIONS > FORMER_TYPE_NODES);

    let products = (0..DECLARATIONS)
        .map(|index| product(index, &format!("wide-{index}"), &[]))
        .collect();
    let plan = derive(&program(hir::Type::Unit, unit(), products, Vec::new()))?;
    assert!(plan.work.scc_work > FORMER_SCC_WORK);
    assert_eq!(plan.work.type_edges, 0);
    Ok(())
}

fn generated_temporary_borrow_program(count: u64) -> hir::Program {
    let binding = hir::BindingId::new(0);
    let source = hir::BindingRef {
        binding,
        storage: hir::BindingStorage::Local(0),
    };
    let mut expressions: Vec<_> = (0..count)
        .map(|loan| {
            expression(
                hir::Type::ByteSlice,
                hir::ExprKind::BorrowBytes {
                    place: hir::PlaceId::new(0),
                    loan: hir::LoanId::new(loan),
                    binding: source,
                },
            )
        })
        .collect();
    expressions.push(unit());
    let mut program = program(
        hir::Type::Unit,
        expression(hir::Type::Unit, hir::ExprKind::Do(expressions)),
        Vec::new(),
        Vec::new(),
    );
    program.bindings.push(hir::Binding {
        id: binding,
        name: "borrow-source".into(),
        kind: hir::BindingKind::Parameter,
        ty: hir::Type::Bytes,
        origin: hir::Origin::Source(origin()),
    });
    program.main.params.push(binding);
    program.main.param_places.push(hir::PlaceId::new(0));
    program.main.param_types.push(hir::Type::Bytes);
    program.main.arity = 1;
    program
}

fn generated_structural_destination_program(
    count: u64,
    expression_count: u64,
) -> Result<hir::Program> {
    let products: Vec<_> = (0..count)
        .map(|index| {
            product(
                index,
                &format!("wide-destination-{index}"),
                &[("value", hir::Type::I64)],
            )
        })
        .collect();
    let mut expressions: Vec<_> = products
        .iter()
        .take(usize::try_from(expression_count).map_err(|_| {
            lkjscript_core::Error::msg("test structural expression count exceeds host usize")
        })?)
        .map(|definition| {
            product_value(
                definition,
                vec![expression(hir::Type::I64, hir::ExprKind::LitI64(0))],
            )
        })
        .collect();
    expressions.push(unit());
    let mut program = program(
        hir::Type::Unit,
        expression(hir::Type::Unit, hir::ExprKind::Do(expressions)),
        products,
        Vec::new(),
    );
    program.traits = (0_u64..)
        .zip(hir::CoreTrait::ALL)
        .map(|(raw, core)| hir::TraitDefinition {
            id: hir::TraitId::new(raw),
            name: core.name().into(),
            origin: hir::Origin::Builtin,
            core: Some(core),
        })
        .collect();
    Ok(program)
}

fn generated_structural_parameter_program(count: u64) -> Result<hir::Program> {
    let mut program = generated_structural_destination_program(count, 1)?;
    let parameter_types: Vec<_> = program
        .products
        .iter()
        .map(|product| hir::Type::Product(product.name.clone()))
        .collect();
    let parameter_ids: Vec<_> = (0_u64..)
        .take(parameter_types.len())
        .map(hir::BindingId::new)
        .collect();
    let function_binding =
        hir::BindingId::new(u64::try_from(parameter_ids.len()).map_err(|_| {
            lkjscript_core::Error::msg("test parameter count exceeds HIR BindingId")
        })?);
    program.bindings = parameter_ids
        .iter()
        .zip(&parameter_types)
        .enumerate()
        .map(|(index, (id, ty))| hir::Binding {
            id: *id,
            name: format!("destination-parameter-{index}"),
            kind: hir::BindingKind::Parameter,
            ty: ty.clone(),
            origin: hir::Origin::Source(origin()),
        })
        .chain(std::iter::once(hir::Binding {
            id: function_binding,
            name: "destination-parameters".into(),
            kind: hir::BindingKind::Function,
            ty: hir::Type::Fn {
                params: parameter_types.clone(),
                ret: Box::new(hir::Type::Unit),
            },
            origin: hir::Origin::Source(origin()),
        }))
        .collect();
    program.functions.push(hir::Function {
        binding: function_binding,
        origin: hir::Origin::Source(origin()),
        params: parameter_ids.clone(),
        param_places: (0_u64..)
            .take(parameter_ids.len())
            .map(hir::PlaceId::new)
            .collect(),
        bounds: Vec::new(),
        arity: parameter_ids.len(),
        local_count: 0,
        summary: hir::EffectSet::PURE,
        body: unit(),
    });
    program.global_layout.push(function_binding);
    Ok(program)
}

#[test]
#[ignore = "opt-in release HIR memory-plan quota-removal stress geometry"]
fn generated_hir_crosses_use_loan_obligation_destination_and_drop_path_boundaries() -> Result<()> {
    const BORROWS: u64 = 65_537;
    const DESTINATIONS_AND_DROP_PATHS: u64 = 32_769;

    let borrow_plan = derive(&generated_temporary_borrow_program(BORROWS))?;
    assert_eq!(borrow_plan.work.uses, BORROWS);
    assert_eq!(borrow_plan.work.loans, BORROWS);
    assert!(borrow_plan.work.obligations > 32_768);

    let destination_hir = generated_structural_destination_program(
        DESTINATIONS_AND_DROP_PATHS,
        DESTINATIONS_AND_DROP_PATHS,
    )?;
    let destination_verified = super::super::verify_hir_memory(&destination_hir)?;
    let destination_plan = destination_verified.plan();
    assert_eq!(
        destination_plan.work.destinations,
        DESTINATIONS_AND_DROP_PATHS
    );
    assert_eq!(
        destination_plan.work.drop_paths,
        DESTINATIONS_AND_DROP_PATHS
    );
    assert_eq!(destination_plan.destinations.len(), 32_769);
    assert_eq!(destination_plan.drop_paths.len(), 32_769);
    Ok(())
}

#[test]
#[ignore = "opt-in release structural destination pipeline stress geometry"]
fn structural_destinations_cross_the_former_limit_in_validated_bytecode() -> Result<()> {
    const DESTINATIONS: u64 = 16_385;

    let destination_hir = generated_structural_parameter_program(DESTINATIONS)?;
    let destination_verified = super::super::verify_hir_memory(&destination_hir)?;
    let (ssa, _) = crate::ssa::lower_program_with_metrics(&destination_verified)?;
    let (chunk, _) = crate::codegen::compile_program(&ssa)?;
    let bytecode =
        lkjscript_core::validate_chunk(chunk, lkjscript_core::ValidationPolicy::Unrestricted)?;
    assert_eq!(bytecode.structural_destinations().len(), 16_385);
    assert_eq!(bytecode.structural_destination_fields().len(), 1);
    Ok(())
}
