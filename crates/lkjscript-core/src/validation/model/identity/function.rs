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
    out.u8(*arity);
    out.u8(*locals);
    out.option(memory_plan.as_ref(), |out, value| out.fixed(&value.bytes()));
    out.sequence(memory_witness_parameters, witness::parameter);
    out.sequence(call_witnesses, witness::call_site);
    out.sequence(parameter_structurals, |out, value| {
        out.option(value.as_ref(), |out, value| out.u16(value.raw()))
    });
    options_u8(out, parameter_structural_places);
    options_u16(out, parameter_type_variables);
    out.sequence(parameter_copy_kinds, |out, value| {
        out.option(value.as_ref(), |out, value| {
            types::structural_kind(out, *value)
        })
    });
    out.option(return_copy_kind.as_ref(), |out, value| {
        types::structural_kind(out, *value)
    });
    out.sequence(parameter_region_products, |out, value| {
        out.option(value.as_ref(), |out, value| out.u16(value.raw()))
    });
    out.option(return_region_product.as_ref(), |out, value| {
        out.u16(value.raw())
    });
    out.option(return_structural.as_ref(), |out, value| {
        out.u16(value.raw())
    });
    out.option(return_type_variable.as_ref(), |out, value| out.u16(*value));
    out.sequence(parameter_resources, |out, value| {
        out.option(value.as_ref(), |out, value| types::resource(out, *value))
    });
    options_u8(out, parameter_resource_places);
    out.option(return_resource.as_ref(), resource_return);
    out.sequence(parameter_uniques, |out, value| {
        out.option(value.as_ref(), |out, value| unique(out, *value))
    });
    options_u8(out, parameter_unique_places);
    out.option(return_unique.as_ref(), |out, value| unique(out, *value));
    out.u8(*unique_places);
    out.sequence(failure_cleanups, cleanup);
    out.sequence(failure_cleanup_ranges, cleanup_range);
    out.bytes(code);
}
fn cleanup(out: &mut Encoder, value: &FailureCleanupPlan) {
    let FailureCleanupPlan { actions } = value;
    out.sequence(actions, cleanup_action);
}
fn cleanup_action(out: &mut Encoder, value: &FailureCleanupAction) {
    match value {
        FailureCleanupAction::EndBorrow { local, place, kind } => {
            out.tag(0);
            out.u8(*local);
            out.u8(*place);
            unique(out, *kind);
        }
        FailureCleanupAction::DropUnique { local, place, kind } => {
            out.tag(1);
            out.u8(*local);
            option_u8(out, place.as_ref());
            unique(out, *kind);
        }
        FailureCleanupAction::DropResource { local, place, kind } => {
            out.tag(2);
            out.u8(*local);
            option_u8(out, place.as_ref());
            types::resource(out, *kind);
        }
        FailureCleanupAction::EndStructuralBorrow {
            local,
            place,
            representation,
        } => {
            out.tag(3);
            out.u8(*local);
            out.u8(*place);
            out.u16(representation.raw());
        }
        FailureCleanupAction::DropStructural {
            local,
            place,
            representation,
        } => {
            out.tag(4);
            out.u8(*local);
            option_u8(out, place.as_ref());
            out.u16(representation.raw());
        }
        FailureCleanupAction::AbortStructuralDestination { local, destination } => {
            out.tag(5);
            out.u8(*local);
            out.u16(destination.raw());
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
    out.u16(*start);
    out.u16(*end);
    out.option(plan.as_ref(), |out, value| out.u16(*value));
    out.option(unentered_plan.as_ref(), |out, value| out.u16(*value));
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
fn options_u8(out: &mut Encoder, values: &[Option<u8>]) {
    out.sequence(values, |out, value| option_u8(out, value.as_ref()));
}
fn option_u8(out: &mut Encoder, value: Option<&u8>) {
    out.option(value, |out, value| out.u8(*value));
}
fn options_u16(out: &mut Encoder, values: &[Option<u16>]) {
    out.sequence(values, |out, value| {
        out.option(value.as_ref(), |out, value| out.u16(*value))
    });
}
