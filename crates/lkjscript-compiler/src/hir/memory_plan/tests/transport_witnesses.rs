use super::super::*;
use super::fixtures::{derive, fact, verify_forged};
use super::transport_fixture::generic_copy_product_program;
use crate::hir;
use lkjscript_core::{Error, Result};

fn direct_call(plan: &mut HirMemoryPlan) -> Result<&mut MemoryCallPlan> {
    plan.calls
        .iter_mut()
        .find(|item| matches!(item.target, MemoryCallTarget::Direct(_)))
        .ok_or_else(|| Error::msg("generic direct call is missing"))
}

fn substitutions(program: &mut hir::Program) -> Result<&mut Vec<hir::TypeSubstitution>> {
    let hir::ExprKind::Call {
        instantiation: Some(instantiation),
        ..
    } = &mut program.main.body.kind
    else {
        return Err(Error::msg("generic fixture instantiation is missing"));
    };
    Ok(&mut instantiation.substitutions)
}

#[test]
fn generic_copy_product_binds_exact_transport_witnesses() -> Result<()> {
    let program = generic_copy_product_program();
    let plan = derive(&program)?;
    let signature = &plan.functions[0].signature;
    assert_eq!(
        signature.witness_parameters,
        vec![
            MemoryWitnessParameter {
                parameter: "u".into(),
                operations: vec![MemoryWitnessOperation::Transport],
            },
            MemoryWitnessParameter {
                parameter: "t".into(),
                operations: vec![MemoryWitnessOperation::Transport],
            },
        ]
    );
    assert!(plan.functions[1].signature.witness_parameters.is_empty());
    let call = plan
        .calls
        .iter()
        .find(|item| matches!(item.target, MemoryCallTarget::Direct(_)))
        .ok_or_else(|| Error::msg("generic direct call is missing"))?;
    let product = fact(&plan, &MemoryType::Product("transport-record".into()))?;
    let boolean = fact(&plan, &MemoryType::Bool)?;
    assert_eq!(
        call.witness_arguments,
        vec![
            MemoryWitnessArgument {
                parameter: "u".into(),
                witness: boolean.witness,
            },
            MemoryWitnessArgument {
                parameter: "t".into(),
                witness: product.witness,
            },
        ]
    );
    let parameter = fact(&plan, &MemoryType::TypeParameter("t".into()))?;
    assert_eq!(
        plan.witness(parameter.witness)
            .ok_or_else(|| Error::msg("type parameter witness is missing"))?
            .facts
            .requirement,
        MemoryWitnessRequirement::SpecializationRequired
    );
    Ok(())
}

#[test]
fn forged_transport_witness_records_are_independently_rejected() -> Result<()> {
    let program = generic_copy_product_program();
    let plan = derive(&program)?;
    let mut forged = Vec::new();

    let mut missing_parameter = plan.clone();
    missing_parameter.functions[0]
        .signature
        .witness_parameters
        .pop();
    forged.push(missing_parameter);
    let mut wrong_operations = plan.clone();
    wrong_operations.functions[0].signature.witness_parameters[0]
        .operations
        .clear();
    forged.push(wrong_operations);
    let mut reordered_parameters = plan.clone();
    reordered_parameters.functions[0]
        .signature
        .witness_parameters
        .swap(0, 1);
    forged.push(reordered_parameters);
    let mut extra_parameter = plan.clone();
    extra_parameter.functions[0]
        .signature
        .witness_parameters
        .push(plan.functions[0].signature.witness_parameters[0].clone());
    forged.push(extra_parameter);

    let mut missing_argument = plan.clone();
    direct_call(&mut missing_argument)?.witness_arguments.pop();
    forged.push(missing_argument);
    let mut wrong_argument = plan.clone();
    direct_call(&mut wrong_argument)?.witness_arguments[0].witness =
        MemoryWitnessId::from_bytes([7; 32]);
    forged.push(wrong_argument);
    let mut reordered_arguments = plan.clone();
    direct_call(&mut reordered_arguments)?
        .witness_arguments
        .swap(0, 1);
    forged.push(reordered_arguments);
    let mut extra_argument = plan.clone();
    let duplicate = direct_call(&mut extra_argument)?.witness_arguments[0].clone();
    direct_call(&mut extra_argument)?
        .witness_arguments
        .push(duplicate);
    forged.push(extra_argument);

    for candidate in &mut forged {
        assert_ne!(compute_plan_id(candidate)?, plan.id);
        assert!(verify_forged(&program, candidate).is_err());
    }
    Ok(())
}

#[test]
fn malformed_transport_substitutions_and_signatures_use_exact_errors() -> Result<()> {
    let error = |program: &hir::Program| {
        producer::derive(program)
            .err()
            .map(|item| item.to_string())
            .unwrap_or_default()
    };
    let mut missing = generic_copy_product_program();
    substitutions(&mut missing)?.pop();
    assert_eq!(
        error(&missing),
        "HIR direct generic call is missing witness substitutions"
    );
    let mut reordered = generic_copy_product_program();
    substitutions(&mut reordered)?.swap(0, 1);
    assert_eq!(
        error(&reordered),
        "HIR direct generic call witness substitutions are reordered"
    );
    let mut duplicate = generic_copy_product_program();
    substitutions(&mut duplicate)?[1].parameter = "u".into();
    assert_eq!(
        error(&duplicate),
        "HIR direct generic call has duplicate witness substitution"
    );
    let mut unresolved = generic_copy_product_program();
    substitutions(&mut unresolved)?[0].ty = hir::Type::Param("forwarded".into());
    assert_eq!(
        error(&unresolved),
        "HIR direct generic call witness substitution is unresolved"
    );

    let mut indirect = generic_copy_product_program();
    if let hir::ExprKind::Call { callee, .. } = &mut indirect.main.body.kind {
        callee.storage = hir::BindingStorage::Local(0);
    }
    assert_eq!(
        error(&indirect),
        "HIR indirect generic call has no residual transport witness signature"
    );
    let mut nested = generic_copy_product_program();
    nested.bindings[2].ty = hir::Type::Forall {
        vars: vec!["t".into(), "u".into()],
        body: Box::new(hir::Type::Fn {
            params: vec![
                hir::Type::List(Box::new(hir::Type::Param("t".into()))),
                hir::Type::Param("u".into()),
            ],
            ret: Box::new(hir::Type::Param("t".into())),
        }),
    };
    assert_eq!(
        error(&nested),
        "HIR memory witness parameter has a nested operational use"
    );
    let mut excessive = generic_copy_product_program();
    let vars: Vec<_> = (0..17).map(|index| format!("t-{index}")).collect();
    excessive.bindings[2].ty = hir::Type::Forall {
        vars: vars.clone(),
        body: Box::new(hir::Type::Fn {
            params: vars.iter().cloned().map(hir::Type::Param).collect(),
            ret: Box::new(hir::Type::Unit),
        }),
    };
    assert_eq!(error(&excessive), "HIR memory witness parameters exceed 16");
    Ok(())
}
