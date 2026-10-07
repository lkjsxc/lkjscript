//! One immutable input projection per exact incoming call, within one proof only.
//! This shares source operands, never requests, selected identities or proof results.
use super::*;
use std::rc::Rc;

#[path = "demanded_callable_input_storage.rs"]
mod storage;

pub(super) enum Inputs {
    Application {
        arguments: Vec<TypeObjectDigest>,
        operands: Vec<ImplementationOperand>,
    },
    Method(ImplementationOperand),
}

pub(super) struct Incoming {
    pub(super) call: CallSite,
    inputs: Option<Rc<Inputs>>,
}

impl Incoming {
    pub(super) fn new(call: CallSite) -> Self {
        Self { call, inputs: None }
    }

    pub(super) fn resolve(
        &mut self,
        load: impl FnOnce(CallSite) -> Result<Inputs, Diagnostic>,
    ) -> Result<Rc<Inputs>, Diagnostic> {
        if let Some(inputs) = &self.inputs {
            return Ok(Rc::clone(inputs));
        }
        // Failed loading, target checks or reservation cannot publish a partial entry.
        let inputs = Rc::new(load(self.call)?);
        self.inputs = Some(Rc::clone(&inputs));
        Ok(inputs)
    }
}

pub(super) fn load<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    call: CallSite,
) -> Result<Inputs, Diagnostic> {
    analysis.reserve::<Inputs>(1)?;
    analysis.reserve::<usize>(2)?; // Rc's strong and weak counters.
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
        match record.operation {
            ExpressionOperation::Call {
                function,
                type_arguments,
                ..
            }
            | ExpressionOperation::FunctionValue {
                function,
                type_arguments,
                ..
            } => {
                admit_application(analysis, call, function, &type_arguments, &[])?;
                Ok(Inputs::Application {
                    arguments: type_arguments,
                    operands: Vec::new(),
                })
            }
            ExpressionOperation::ImplementationCall {
                function,
                type_arguments,
                implementations,
                ..
            } => {
                admit_application(analysis, call, function, &type_arguments, &implementations)?;
                Ok(Inputs::Application {
                    arguments: type_arguments,
                    operands: implementations,
                })
            }
            ExpressionOperation::MethodCall {
                witness, method, ..
            } => {
                let selected = analysis.selection(call.from, &witness)?;
                let Selection::Scheme {
                    implementation,
                    prerequisites,
                } = analysis.selected_shape(selected)?
                else {
                    return Err(missing_path());
                };
                let key = &analysis.contexts[call.to].key;
                if key.owner != implementation
                    || key.method != Some(method)
                    || key.implementations != prerequisites
                {
                    return Err(changed_target());
                }
                storage::reserve(analysis, &[], std::slice::from_ref(&witness))?;
                Ok(Inputs::Method(witness))
            }
            _ => Err(changed_target()),
        }
    } else {
        let declaration = analysis.declaration(owner)?;
        let DeclarationPayload::OwnedImplementation(scheme) = declaration.payload else {
            return Err(changed_target());
        };
        let mapping = analysis.mapping(&scheme, analysis.contexts[call.from].key.method)?;
        admit_application(
            analysis,
            call,
            mapping.function,
            &mapping.type_arguments,
            &mapping.implementations,
        )?;
        // The declaration remains the read owner's temporary. Reserve before copying
        // only its selected mapping, rather than retaining every unrelated method.
        Ok(Inputs::Application {
            arguments: mapping.type_arguments.clone(),
            operands: mapping.implementations.clone(),
        })
    }
}

fn admit_application<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
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
    storage::reserve(analysis, arguments, operands)
}

fn changed_target() -> Diagnostic {
    semantic(
        "kernel_callable_flow_context",
        "recorded exact callable target changed",
    )
}

#[cfg(test)]
#[path = "demanded_callable_input_tests.rs"]
mod tests;
