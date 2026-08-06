use super::{encoder::Encoder, types, witness};
use crate::*;

pub(super) fn function(out: &mut Encoder, value: &FunctionProto) {
    let FunctionProto {
        name,
        arity,
        locals,
        memory_plan,
        memory_witness_parameters,
        call_witnesses,
        parameter_structurals,
        parameter_structural_places,
        parameter_type_variables,
        parameter_copy_kinds,
        return_copy_kind,
        parameter_region_products,
        return_region_product,
        return_structural,
        return_type_variable,
        parameter_resources,
        parameter_resource_places,
        return_resource,
        parameter_uniques,
        parameter_unique_places,
        return_unique,
        unique_places,
        failure_cleanups,
        failure_cleanup_ranges,
        code,
    } = value;
    out.string(name);
    out.len(*arity);
    out.len(*locals);
    out.option(memory_plan.as_ref(), |out, value| out.fixed(&value.bytes()));
    out.sequence(memory_witness_parameters, witness::parameter);
    out.sequence(call_witnesses, witness::call_site);
    out.sequence(parameter_structurals, |out, value| {
        out.option(value.as_ref(), |out, value| out.u64(value.raw()))
    });
    options_usize(out, parameter_structural_places);
    options_u64(out, parameter_type_variables);
    out.sequence(parameter_copy_kinds, |out, value| {
        out.option(value.as_ref(), |out, value| {
            types::structural_kind(out, *value)
        })
    });
    out.option(return_copy_kind.as_ref(), |out, value| {
        types::structural_kind(out, *value)
    });
    out.sequence(parameter_region_products, |out, value| {
        out.option(value.as_ref(), |out, value| out.u64(value.raw()))
    });
    out.option(return_region_product.as_ref(), |out, value| {
        out.u64(value.raw())
    });
    out.option(return_structural.as_ref(), |out, value| {
        out.u64(value.raw())
    });
    out.option(return_type_variable.as_ref(), |out, value| out.u64(*value));
    out.sequence(parameter_resources, |out, value| {
        out.option(value.as_ref(), |out, value| types::resource(out, *value))
    });
    options_usize(out, parameter_resource_places);
    out.option(return_resource.as_ref(), resource_return);
    out.sequence(parameter_uniques, |out, value| {
        out.option(value.as_ref(), |out, value| unique(out, *value))
    });
    options_usize(out, parameter_unique_places);
    out.option(return_unique.as_ref(), |out, value| unique(out, *value));
    out.len(*unique_places);
    out.sequence(failure_cleanups, cleanup);
    out.sequence(failure_cleanup_ranges, cleanup_range);
    out.bytes(code);
}
fn cleanup(out: &mut Encoder, value: &FailureCleanupNode) {
    let FailureCleanupNode { action, next } = value;
    cleanup_action(out, action);
    out.option(next.as_ref(), |out, value| out.u64(value.raw()));
}
fn cleanup_action(out: &mut Encoder, value: &FailureCleanupAction) {
    match value {
        FailureCleanupAction::EndBorrow { local, place, kind } => {
            out.tag(0);
            out.len(*local);
            out.len(*place);
            unique(out, *kind);
        }
        FailureCleanupAction::DropUnique { local, place, kind } => {
            out.tag(1);
            out.len(*local);
            option_usize(out, place.as_ref());
            unique(out, *kind);
        }
        FailureCleanupAction::DropResource { local, place, kind } => {
            out.tag(2);
            out.len(*local);
            option_usize(out, place.as_ref());
            types::resource(out, *kind);
        }
        FailureCleanupAction::EndStructuralBorrow {
            local,
            place,
            representation,
        } => {
            out.tag(3);
            out.len(*local);
            out.len(*place);
            out.u64(representation.raw());
        }
        FailureCleanupAction::DropStructural {
            local,
            place,
            representation,
        } => {
            out.tag(4);
            out.len(*local);
            option_usize(out, place.as_ref());
            out.u64(representation.raw());
        }
        FailureCleanupAction::AbortStructuralDestination { local, destination } => {
            out.tag(5);
            out.len(*local);
            out.u64(destination.raw());
        }
    }
}
fn cleanup_range(out: &mut Encoder, value: &FailureCleanupRange) {
    let FailureCleanupRange {
        start,
        end,
        plan,
        unentered_plan,
    } = value;
    out.u64(*start);
    out.u64(*end);
    out.option(plan.as_ref(), |out, roots| {
        out.option(roots.loans.as_ref(), |out, value| out.u64(value.raw()));
        out.option(roots.unplaced.as_ref(), |out, value| out.u64(value.raw()));
        out.option(roots.places.as_ref(), |out, value| out.u64(value.raw()));
    });
    out.option(unentered_plan.as_ref(), |out, value| out.u64(value.raw()));
}
fn resource_return(out: &mut Encoder, value: &ResourceReturnKind) {
    match value {
        ResourceReturnKind::Resource(kind) => {
            out.tag(0);
            types::resource(out, *kind);
        }
        ResourceReturnKind::Result(kind) => {
            out.tag(1);
            types::resource(out, *kind);
        }
    }
}
fn unique(out: &mut Encoder, value: UniqueValueKind) {
    out.tag(match value {
        UniqueValueKind::Bytes => 0,
        UniqueValueKind::ByteVector => 1,
        UniqueValueKind::ByteSlice => 2,
        UniqueValueKind::ByteSliceMut => 3,
    });
}
fn options_usize(out: &mut Encoder, values: &[Option<usize>]) {
    out.sequence(values, |out, value| option_usize(out, value.as_ref()));
}
fn option_usize(out: &mut Encoder, value: Option<&usize>) {
    out.option(value, |out, value| out.len(*value));
}
fn options_u64(out: &mut Encoder, values: &[Option<u64>]) {
    out.sequence(values, |out, value| {
        out.option(value.as_ref(), |out, value| out.u64(*value))
    });
}
