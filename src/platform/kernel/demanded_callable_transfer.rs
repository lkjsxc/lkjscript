//! Pull one exact target provenance group through its admitted call inputs.
//! Sharing those inputs never merges target paths, source scopes or type slots.
use super::*;

pub(super) fn connect<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    demands: &mut Demands,
    target: &Request,
    call: CallSite,
    inputs: &Inputs,
) -> Result<(), Diagnostic> {
    match inputs {
        Inputs::Application {
            arguments,
            operands,
        } => match target.path.split_first() {
            None => analysis.arguments(call.from, &target.slots, arguments, call.expression),
            Some((root, suffix)) => operand(
                analysis,
                demands,
                target,
                call,
                operands.get(*root).ok_or_else(missing_path)?,
                suffix,
            ),
        },
        Inputs::Method(witness) => operand(analysis, demands, target, call, witness, &target.path),
    }
}

fn operand<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    demands: &mut Demands,
    target: &Request,
    call: CallSite,
    mut operand: &ImplementationOperand,
    mut suffix: &[usize],
) -> Result<(), Diagnostic> {
    loop {
        analysis.tick()?;
        match operand {
            ImplementationOperand::Concrete {
                type_arguments,
                implementations,
                ..
            } => {
                if let Some((child, rest)) = suffix.split_first() {
                    operand = implementations.get(*child).ok_or_else(missing_path)?;
                    suffix = rest;
                } else {
                    return analysis.arguments(
                        call.from,
                        &target.slots,
                        type_arguments,
                        call.expression,
                    );
                }
            }
            ImplementationOperand::Parameter { scope, parameter } => {
                let owner = analysis.contexts[call.from].key.owner;
                if *scope != owner {
                    return Err(semantic(
                        "kernel_callable_flow_witness_scope",
                        "witness parameter escapes its exact callable scope",
                    ));
                }
                let declaration = analysis.declaration(owner)?;
                let ordinal = analysis.parameter_ordinal(&declaration.payload, *parameter)?;
                return demands.forward(analysis, call, target, ordinal, suffix);
            }
        }
    }
}
