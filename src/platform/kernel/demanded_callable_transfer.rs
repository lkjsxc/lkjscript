//! Pull one exact target provenance group through the original call. Complete
//! source/type/operand admission happened during discovery, including dead syntax.
use super::*;

pub(super) fn connect<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    demands: &mut Demands,
    target: &Request,
    call: CallSite,
) -> Result<(), Diagnostic> {
    let owner = analysis.contexts[call.from].key.owner;
    if let Some(expression) = call.expression {
        let Some(OwnerRecord::Expression(record)) = analysis
            .read
            .owner(owner.package, OwnerKey::Expression(expression))?
        else {
            return Err(semantic(
                "kernel_callable_flow_expression",
                "recorded callable syntax is missing",
            ));
        };
        match &record.operation {
            ExpressionOperation::Call {
                function,
                type_arguments,
                ..
            }
            | ExpressionOperation::FunctionValue {
                function,
                type_arguments,
                ..
            } => application(
                analysis,
                demands,
                target,
                call,
                *function,
                type_arguments,
                &[],
            ),
            ExpressionOperation::ImplementationCall {
                function,
                type_arguments,
                implementations,
                ..
            } => application(
                analysis,
                demands,
                target,
                call,
                *function,
                type_arguments,
                implementations,
            ),
            ExpressionOperation::MethodCall {
                witness, method, ..
            } => {
                let selected = analysis.selection(call.from, witness)?;
                let Selection::Scheme {
                    implementation,
                    prerequisites,
                } = analysis.selected_shape(selected)?
                else {
                    return Err(missing_path());
                };
                let key = &analysis.contexts[call.to].key;
                if key.owner != implementation
                    || key.method != Some(*method)
                    || key.implementations != prerequisites
                {
                    return Err(changed_target());
                }
                operand(analysis, demands, target, call, witness, &target.path)
            }
            _ => Err(changed_target()),
        }
    } else {
        let declaration = analysis.declaration(owner)?;
        let DeclarationPayload::OwnedImplementation(scheme) = declaration.payload else {
            return Err(changed_target());
        };
        let mapping = analysis.mapping(&scheme, analysis.contexts[call.from].key.method)?;
        application(
            analysis,
            demands,
            target,
            call,
            mapping.function,
            &mapping.type_arguments,
            &mapping.implementations,
        )
    }
}
fn application<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    demands: &mut Demands,
    target: &Request,
    call: CallSite,
    function: DeclarationReference,
    arguments: &[TypeObjectDigest],
    operands: &[ImplementationOperand],
) -> Result<(), Diagnostic> {
    let context = &analysis.contexts[call.to];
    if context.key.owner != function || context.key.method.is_some() {
        return Err(changed_target());
    }
    if arguments.len() != context.parameters.len()
        || operands.len() != context.key.implementations.len()
    {
        return Err(semantic(
            "kernel_callable_flow_arity",
            "recorded callable application arity changed",
        ));
    }
    match target.path.split_first() {
        None => analysis.arguments(call.from, &target.slots, arguments, call.expression),
        Some((root, suffix)) => operand(
            analysis,
            demands,
            target,
            call,
            operands.get(*root).ok_or_else(missing_path)?,
            suffix,
        ),
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
fn changed_target() -> Diagnostic {
    semantic(
        "kernel_callable_flow_context",
        "recorded exact callable target changed",
    )
}
