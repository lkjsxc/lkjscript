use super::{decode::instruction_error, Kind, State};
use crate::{DecodedInstruction, FunctionProto, Result};

pub(super) fn merge_state(
    target: &mut Option<State>,
    incoming: &State,
    proto: &FunctionProto,
    predecessor: DecodedInstruction,
) -> Result<bool> {
    let Some(current) = target else {
        *target = Some(incoming.clone());
        return Ok(true);
    };
    if current.stack.len() != incoming.stack.len() {
        return Err(instruction_error(
            proto,
            predecessor.op(),
            predecessor.offset(),
            "incompatible operand stack depth at CFG join",
        ));
    }
    let mut changed = false;
    for (existing, incoming) in current.stack.iter_mut().zip(&incoming.stack) {
        let merged = merge_kind(*existing, *incoming).ok_or_else(|| {
            instruction_error(
                proto,
                predecessor.op(),
                predecessor.offset(),
                "incompatible operand stack categories at CFG join",
            )
        })?;
        if *existing != merged {
            *existing = merged;
            changed = true;
        }
    }
    for (existing, incoming) in current.locals.iter_mut().zip(&incoming.locals) {
        changed |= merge_slot(existing, *incoming, proto, predecessor, "local")?;
    }
    for (existing, incoming) in current.globals.iter_mut().zip(&incoming.globals) {
        changed |= merge_slot(existing, *incoming, proto, predecessor, "global")?;
    }
    Ok(changed)
}

fn merge_slot(
    existing: &mut Option<Kind>,
    incoming: Option<Kind>,
    proto: &FunctionProto,
    predecessor: DecodedInstruction,
    category: &str,
) -> Result<bool> {
    match (*existing, incoming) {
        (None, _) => Ok(false),
        (Some(_), None) => {
            *existing = None;
            Ok(true)
        }
        (Some(left), Some(right)) => {
            let merged = merge_kind(left, right).ok_or_else(|| {
                instruction_error(
                    proto,
                    predecessor.op(),
                    predecessor.offset(),
                    &format!("incompatible {category} categories at CFG join"),
                )
            })?;
            if Some(merged) == *existing {
                Ok(false)
            } else {
                *existing = Some(merged);
                Ok(true)
            }
        }
    }
}

fn merge_kind(left: Kind, right: Kind) -> Option<Kind> {
    if left == right {
        Some(left)
    } else if left == Kind::Any || right == Kind::Any {
        Some(Kind::Any)
    } else {
        None
    }
}
