use std::collections::HashMap;

use crate::normalize::*;
use crate::{CallTarget, EffectSet, InstructionKind, VerifiedProgram};

pub fn direct_call_resolution(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    let function_effects: Vec<EffectSet> = program
        .functions
        .iter()
        .map(|function| function.effects)
        .collect();
    for function in &mut program.functions {
        let mut references = HashMap::new();
        for block in &function.blocks {
            for instruction in &block.instructions {
                match instruction.kind {
                    InstructionKind::FunctionRef(target) => {
                        references.insert(instruction.id, target);
                    }
                    InstructionKind::Copy(source) => {
                        if let Some(target) = references.get(&source).copied() {
                            references.insert(instruction.id, target);
                        }
                    }
                    _ => {}
                }
            }
        }
        for block in &mut function.blocks {
            for instruction in &mut block.instructions {
                let replacement = match &instruction.kind {
                    InstructionKind::Call {
                        target: CallTarget::Indirect(value),
                        arguments,
                        consuming,
                        signature,
                        instantiation,
                    } => references.get(value).map(|target| InstructionKind::Call {
                        target: CallTarget::Direct(*target),
                        arguments: arguments.clone(),
                        consuming: consuming.clone(),
                        signature: signature.clone(),
                        instantiation: instantiation.clone(),
                    }),
                    _ => None,
                };
                if let Some(replacement) = replacement {
                    instruction.kind = replacement;
                    let target = match &instruction.kind {
                        InstructionKind::Call {
                            target: CallTarget::Direct(target),
                            ..
                        } => *target,
                        _ => continue,
                    };
                    if let Some(effects) = function_effects
                        .get(target.index().unwrap_or(usize::MAX))
                        .copied()
                    {
                        instruction.metadata.effects = effects;
                        instruction.metadata.failure = failure_behavior(effects);
                    }
                }
            }
        }
    }
    finish(program)
}
