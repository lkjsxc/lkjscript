use super::{encoder::Encoder, instruction, memory, metadata, types};
use crate::*;

pub(super) fn function(out: &mut Encoder, value: &Function) {
    let Function {
        id,
        name,
        signature,
        places,
        failure_cleanups,
        effects,
        entry,
        blocks,
        origin,
    } = value;
    out.u32(id.raw());
    out.string(name);
    types::signature_value(out, signature);
    out.sequence(places, |out, value| {
        let PlaceMetadata {
            id,
            binding,
            ty,
            drop_glue,
        } = value;
        out.u32(id.raw());
        out.u32(binding.raw());
        types::ty(out, ty);
        out.option(drop_glue.as_ref(), |out, value| {
            memory::drop_glue(out, *value)
        });
    });
    out.sequence(failure_cleanups, failure_cleanup);
    metadata::effects(out, *effects);
    out.u32(entry.raw());
    out.sequence(blocks, block);
    metadata::origin(out, origin);
}

fn failure_cleanup(out: &mut Encoder, value: &FailureCleanupNode) {
    let FailureCleanupNode { action, next } = value;
    match action {
        FailureCleanupAction::EndBorrow {
            place,
            loan,
            kind,
            value,
        } => {
            out.tag(0);
            out.u32(place.raw());
            out.u32(loan.raw());
            borrow_kind(out, *kind);
            out.u32(value.raw());
        }
        FailureCleanupAction::DropOwner { place, value, glue } => {
            out.tag(1);
            out.option(place.as_ref(), |out, value| out.u32(value.raw()));
            out.u32(value.raw());
            memory::drop_glue(out, *glue);
        }
    }
    out.option(next.as_ref(), |out, value| out.u64(value.raw()));
}

fn block(out: &mut Encoder, value: &Block) {
    let Block {
        id,
        parameters,
        instructions,
        terminator,
        metadata: block_metadata,
    } = value;
    out.u32(id.raw());
    out.sequence(parameters, |out, value| {
        let BlockParameter {
            id,
            ty,
            owner_place,
            origin,
        } = value;
        out.u32(id.raw());
        types::ty(out, ty);
        out.option(owner_place.as_ref(), |out, value| out.u32(value.raw()));
        metadata::origin(out, origin);
    });
    out.sequence(instructions, instruction::instruction);
    terminator_value(out, terminator);
    let BlockMetadata {
        loop_header,
        origin,
        failure_cleanup,
        frame_state,
    } = block_metadata;
    out.bool(*loop_header);
    metadata::origin(out, origin);
    out.option(failure_cleanup.as_ref(), |out, roots| {
        out.option(roots.loans.as_ref(), |out, value| out.u64(value.raw()));
        out.option(roots.unplaced.as_ref(), |out, value| out.u64(value.raw()));
        out.option(roots.places.as_ref(), |out, value| out.u64(value.raw()));
    });
    out.option(frame_state.as_ref(), instruction::frame_state);
}

fn terminator_value(out: &mut Encoder, value: &Terminator) {
    match value {
        Terminator::Branch { target, arguments } => {
            out.tag(0);
            out.u32(target.raw());
            ids(out, arguments);
        }
        Terminator::ConditionalBranch {
            condition,
            true_target,
            true_arguments,
            false_target,
            false_arguments,
        } => {
            out.tag(1);
            out.u32(condition.raw());
            out.u32(true_target.raw());
            ids(out, true_arguments);
            out.u32(false_target.raw());
            ids(out, false_arguments);
        }
        Terminator::Return(value) => {
            out.tag(2);
            out.u32(value.raw());
        }
        Terminator::Trap { value } => {
            out.tag(3);
            out.u32(value.raw());
        }
        Terminator::Exit { code } => {
            out.tag(4);
            out.u32(code.raw());
        }
        Terminator::Outcome { outcome, detail } => {
            out.tag(5);
            out.tag(match outcome {
                StructuredOutcome::DeadlineExceeded => 0,
                StructuredOutcome::ResourceLimitExceeded => 1,
                StructuredOutcome::HostFailure => 2,
            });
            out.option(detail.as_ref(), |out, value| out.u32(value.raw()));
        }
    }
}

pub(super) fn borrow_kind(out: &mut Encoder, value: BorrowKind) {
    out.tag(match value {
        BorrowKind::Shared => 0,
        BorrowKind::Mutable => 1,
    });
}
fn ids(out: &mut Encoder, values: &[ValueId]) {
    out.sequence(values, |out, value| out.u32(value.raw()));
}
