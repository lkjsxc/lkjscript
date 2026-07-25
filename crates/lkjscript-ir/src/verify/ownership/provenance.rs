use std::collections::{BTreeMap, BTreeSet};

use crate::verify::*;
use crate::{Function, InstructionKind, IrError};

pub(crate) fn collect_ownership_provenance(
    function: &Function,
) -> crate::Result<(usize, &crate::Block)> {
    let mut work = 0usize;
    charge_ownership_work(&mut work, function.blocks.len())?;
    charge_ownership_work(&mut work, function.places.len())?;
    for block in &function.blocks {
        charge_ownership_work(&mut work, block.parameters.len())?;
        charge_ownership_work(&mut work, block.instructions.len())?;
        charge_ownership_work(&mut work, successors(&block.terminator).len())?;
    }
    let entry = block_by_id(function, function.entry)?;
    let mut proven_places = BTreeSet::new();
    let mut loan_ids = BTreeSet::new();
    let mut borrows = BTreeMap::new();

    for parameter in &entry.parameters {
        charge_ownership_work(&mut work, 1)?;
        if is_owned_buf(&parameter.ty) {
            let place = parameter.owner_place.ok_or_else(|| {
                IrError::new("SSA Owned entry parameter is missing exact place provenance")
            })?;
            let declared = place_by_id(function, place)?;
            if declared.ty != parameter.ty || !proven_places.insert(place) {
                return fail("SSA Owned entry parameter has duplicate or mismatched provenance");
            }
        }
    }

    for block in &function.blocks {
        for instruction in &block.instructions {
            charge_ownership_work(&mut work, 1)?;
            match instruction.kind {
                InstructionKind::PlaceInit { place, .. } => {
                    proven_places.insert(place);
                }
                InstructionKind::Borrow { loan, .. } => {
                    if !loan_ids.insert(loan) {
                        return fail("SSA has duplicate LoanId ownership facts");
                    }
                    borrows.insert(
                        instruction.id,
                        BorrowDefinition {
                            block: block.id,
                            loan,
                        },
                    );
                }
                _ => {}
            }
        }
    }
    for place in &function.places {
        if is_owned_buf(&place.ty) && !proven_places.contains(&place.id) {
            return fail("SSA Owned local place is missing exact initialization provenance");
        }
    }
    verify_borrow_uses(function, &borrows, &mut work)?;
    Ok((work, entry))
}
