//! Immutable code sharing with exact application-owned call operands.
//! Witness identity and authority remain on application records, never on shared bytes.
use super::super::prepare::{NormalizedCallSite, NormalizedFunction};
use super::*;
use crate::platform::kernel::{DeclarationReference, EffectRow, RequirementOperand};
use std::sync::Arc;

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct CodeIdentity {
    declaration: DeclarationReference,
    types: Arc<[TypeObjectDigest]>,
    effect: EffectRow,
    effects: Arc<[EffectRow]>,
    requirements: Arc<[RequirementOperand]>,
}

pub(super) fn share(
    program: &mut NormalizedProgram,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    // Preserve the allocation-free ordinary preparation path.
    let mut selected = false;
    for function in program.functions.iter() {
        step(work)?;
        selected |= !function.implementation_arguments.is_empty();
    }
    if !selected {
        return Ok(());
    }
    let mut bodies: BTreeMap<CodeIdentity, Vec<NormalizedCode>> = BTreeMap::new();
    work.reserve::<usize>(2)?;
    let empty_types: Arc<[TypeObjectDigest]> = Arc::from([]);
    if Arc::get_mut(&mut program.functions).is_none() {
        work.reserve::<NormalizedFunction>(program.functions.len())?;
        for function in program.functions.iter() {
            step(work)?;
            work.cloned_effect(&function.effect)?;
            if let NormalizedFunctionBody::External(name) = &function.body {
                work.reserve::<u8>(name.as_str().len())?;
            }
        }
    }
    for function in Arc::make_mut(&mut program.functions) {
        step(work)?;
        if function.type_parameters.len() != function.type_arguments.len()
            || function.implementation_parameters.len() != function.implementation_arguments.len()
            || !function.effect_parameters.is_empty()
            || !function.requirement_parameters.is_empty()
        {
            continue;
        }
        let NormalizedFunctionBody::Code(code) = &mut function.body else {
            continue;
        };
        if let crate::platform::kernel::FunctionEffect::Task {
            requirements,
            effect_parameters,
        } = &function.effect
        {
            work.reserve::<RequirementOperand>(requirements.len())?;
            work.reserve::<crate::platform::kernel::EffectParameterReference>(
                effect_parameters.len(),
            )?;
        }
        let key = CodeIdentity {
            declaration: function.declaration,
            types: Arc::clone(&function.type_arguments),
            effect: function.effect.row(),
            effects: Arc::clone(&function.effect_arguments),
            requirements: Arc::clone(&function.requirement_arguments),
        };
        let mut count = 0;
        for instruction in code.instructions.iter() {
            step(work)?;
            if matches!(
                instruction,
                NormalizedInstruction::Call { .. }
                    | NormalizedInstruction::TailCall { .. }
                    | NormalizedInstruction::FunctionValue { .. }
                    | NormalizedInstruction::Parallel { .. }
            ) {
                count += 1;
            }
        }
        work.reserve::<(u32, NormalizedCallSite)>(count)?;
        work.reserve::<usize>(2)?;
        if Arc::get_mut(&mut code.instructions).is_none() {
            work.reserve::<NormalizedInstruction>(code.instructions.len())?;
        }
        let mut callsites = Vec::with_capacity(count);
        let placeholder = FunctionIndex(u32::MAX, program.value_origin);
        for (position, instruction) in Arc::make_mut(&mut code.instructions).iter_mut().enumerate()
        {
            step(work)?;
            let application = match instruction {
                NormalizedInstruction::Call {
                    function,
                    type_arguments,
                    ..
                }
                | NormalizedInstruction::TailCall {
                    function,
                    type_arguments,
                    ..
                }
                | NormalizedInstruction::FunctionValue {
                    function,
                    type_arguments,
                    ..
                } => {
                    let application = NormalizedCallSite::Call {
                        function: *function,
                        type_arguments: Arc::clone(type_arguments),
                    };
                    *function = placeholder;
                    *type_arguments = Arc::clone(&empty_types);
                    Some(application)
                }
                NormalizedInstruction::Parallel {
                    left,
                    left_types,
                    left_implementations,
                    right,
                    right_types,
                    right_implementations,
                    ..
                } => {
                    if !left_implementations.is_empty() || !right_implementations.is_empty() {
                        return Err(missing());
                    }
                    let application = NormalizedCallSite::Parallel {
                        left: *left,
                        left_types: Arc::clone(left_types),
                        right: *right,
                        right_types: Arc::clone(right_types),
                    };
                    *left = placeholder;
                    *right = placeholder;
                    *left_types = Arc::clone(&empty_types);
                    *right_types = Arc::clone(&empty_types);
                    Some(application)
                }
                _ => None,
            };
            if let Some(application) = application {
                callsites.push((u32::try_from(position).map_err(|_| missing())?, application));
            }
        }
        function.callsites = callsites.into();
        if let Some(candidates) = bodies.get_mut(&key) {
            let mut shared = None;
            for candidate in candidates.iter() {
                for _ in code.instructions.iter() {
                    step(work)?;
                }
                if candidate == code {
                    shared = Some(Arc::clone(&candidate.instructions));
                    break;
                }
            }
            if let Some(instructions) = shared {
                code.instructions = instructions;
            } else {
                work.reserve::<NormalizedCode>(1)?;
                candidates.push(code.clone());
            }
        } else {
            work.node::<(CodeIdentity, Vec<NormalizedCode>)>()?;
            work.reserve::<NormalizedCode>(1)?;
            bodies.insert(key, vec![code.clone()]);
        }
    }
    Ok(())
}
