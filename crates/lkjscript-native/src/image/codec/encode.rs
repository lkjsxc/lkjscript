use super::{heap_encode, values, writer::Writer, ImageCodecError, ImageCodecLimits};
use crate::image::*;

pub(super) fn payload(
    image: &InstallableImage,
    limits: ImageCodecLimits,
) -> Result<Vec<u8>, ImageCodecError> {
    image
        .validate_integrity()
        .map_err(|_| ImageCodecError::new("cannot encode invalid image"))?;
    let mut out = Writer::new(limits.max_encoded_bytes);
    contracts(&mut out, image.contracts())?;
    out.u64(image.accounting().work_units())?;
    out.bytes(image.bytes())?;
    out.sequence(image.entries(), entry)?;
    out.sequence(image.relocations(), relocation)?;
    out.sequence(image.runtime_calls(), |out, item| {
        values::runtime_call(out, *item)
    })?;
    out.sequence(image.frames(), frame)?;
    out.sequence(image.safepoints(), safepoint)?;
    out.sequence(&image.root_requirements, root_requirement)?;
    out.sequence(image.heap_runtime_sites(), heap_site)?;
    out.sequence(image.source_map(), source_map)?;
    out.sequence(image.trap_map(), trap_map)?;
    out.sequence(image.outcome_map(), outcome_map)?;
    Ok(out.finish())
}

fn contracts(out: &mut Writer, value: ImageContracts) -> Result<(), ImageCodecError> {
    out.fixed(&value.language().as_bytes())?;
    out.fixed(&value.verified_ssa().as_bytes())?;
    out.fixed(&value.runtime_calls().as_bytes())?;
    out.fixed(&value.native_layout().as_bytes())
}

fn entry(out: &mut Writer, value: &EntryMetadata) -> Result<(), ImageCodecError> {
    values::function(out, value.function())?;
    out.u32(value.source_function().get())?;
    values::signature(out, value.signature())?;
    out.u32(value.offset())?;
    out.u32(value.end())
}

fn relocation(out: &mut Writer, value: &Relocation) -> Result<(), ImageCodecError> {
    out.u32(value.offset())?;
    out.u8(match value.kind() {
        RelocationKind::Absolute64 => 0,
    })?;
    match value.target() {
        RelocationTarget::Function(function) => {
            out.u8(0)?;
            values::function(out, function)
        }
        RelocationTarget::Runtime(slot) => {
            out.u8(1)?;
            values::runtime_call(out, slot)
        }
    }
}

fn frame(out: &mut Writer, value: &FrameFacts) -> Result<(), ImageCodecError> {
    values::function(out, value.function())?;
    out.u32(value.frame_bytes())?;
    out.u32(value.value_slots())?;
    out.u32(value.local_slots())?;
    out.u8(value.outgoing_machine_arguments())?;
    out.bool(value.uses_red_zone())?;
    out.bool(value.call_site_aligned_16())?;
    out.sequence(value.homes(), |out, item| values::frame_home(out, *item))
}

fn safepoint(out: &mut Writer, value: &Safepoint) -> Result<(), ImageCodecError> {
    out.u32(value.id())?;
    values::function(out, value.function())?;
    out.u32(value.code_offset())?;
    out.sequence(value.stack_map().roots(), |out, item| root(out, *item))
}

fn root_requirement(out: &mut Writer, value: &RootMapRequirement) -> Result<(), ImageCodecError> {
    out.u32(value.id)?;
    values::function(out, value.function)?;
    out.sequence(&value.roots, |out, item| root(out, *item))
}

fn root(out: &mut Writer, value: RootLocation) -> Result<(), ImageCodecError> {
    out.i32(value.rbp_displacement())?;
    match value.kind() {
        FrameHomeKind::Local(index) => {
            out.u8(0)?;
            out.u32(index)?;
        }
        FrameHomeKind::Value(index) => {
            out.u8(1)?;
            out.u32(index)?;
        }
    }
    values::value_type(out, ValueType::Reference(value.reference_type()))
}

fn heap_site(out: &mut Writer, value: &HeapRuntimeSite) -> Result<(), ImageCodecError> {
    out.u32(value.id())?;
    values::function(out, value.function())?;
    out.u32(value.safepoint())?;
    heap_encode::descriptor(out, value.descriptor())?;
    out.sequence(value.arguments(), |out, item| {
        values::frame_home(out, *item)
    })?;
    values::frame_home(out, value.result())?;
    values::source(out, value.source())
}

fn source_map(out: &mut Writer, value: &SourceMapEntry) -> Result<(), ImageCodecError> {
    values::function(out, value.function())?;
    out.u32(value.code_start())?;
    out.u32(value.code_end())?;
    values::source(out, value.source())
}

fn trap_map(out: &mut Writer, value: &TrapMapEntry) -> Result<(), ImageCodecError> {
    values::function(out, value.function())?;
    out.u32(value.code_offset())?;
    out.u32(value.trap().as_u32())?;
    match value.site() {
        Some(site) => {
            out.u8(1)?;
            out.u32(site)
        }
        None => out.u8(0),
    }
}

fn outcome_map(out: &mut Writer, value: &OutcomeMapEntry) -> Result<(), ImageCodecError> {
    values::function(out, value.function())?;
    out.u32(value.code_offset())?;
    match value.outcome() {
        OutcomeKind::Return => out.u8(0),
        OutcomeKind::Trap(code) => {
            out.u8(1)?;
            out.u32(code.as_u32())
        }
        OutcomeKind::Exit => out.u8(2),
        OutcomeKind::DeadlineExceeded => out.u8(3),
        OutcomeKind::ResourceLimitExceeded => out.u8(4),
        OutcomeKind::HostFailure => out.u8(5),
    }
}
