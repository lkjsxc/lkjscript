//! Share a frame slot across one proven-dead aggregate transfer. No IR rewrite.
//! Borrowed sources and every transfer crossing a control boundary stay apart.
use super::*;

pub(super) fn coalesce(
    function: &Function, layout: &Layouts, first: &[usize], last: &[usize],
    borrowed: &[bool],
) -> Vec<usize> {
    let mut storage = Vec::new();
    for local in 0..function.locals.len() { storage.push(local); }
    let mut block = 0;
    for (index, instruction) in function.body.iter().enumerate() {
        let transfer = match &instruction.op {
            Op::Read { dst, place, .. } if place.projections.is_empty() => Some((*dst, place.local)),
            Op::Assign { place, src } if place.projections.is_empty() => Some((place.local, *src)),
            _ => None,
        };
        if let Some((destination, source)) = transfer {
            let ty = function.locals[source].ty;
            // A Copy read is eligible too, but only when every future use and
            // definition of its source is absent. Earlier projected accesses
            // are harmless; taking any view of the source forbids sharing.
            // The destination starts its lifetime here, so later views of it
            // observe the same storage for its entire lifetime.
            if source != destination && ty == function.locals[destination].ty
                && !layout.storage(ty).scalar && layout.storage(ty).bytes != 0
                && !borrowed[source]
                && first[destination] == index * 2 + 1 && last[source] == index * 2
                && first[source] >= block * 2 + 1 && first[source] < index * 2
                && first[source] % 2 == 1
            {
                storage[destination] = storage[source];
            }
        }
        match instruction.op {
            Op::If { .. } | Op::Else | Op::End | Op::Loop | Op::Break | Op::Continue
                | Op::BreakTo { .. } | Op::ContinueTo { .. } | Op::Return { .. }
                | Op::Trap { .. } => block = index + 1,
            _ => {}
        }
    }
    storage
}
