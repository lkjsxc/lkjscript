use super::{heap_decode, reader::Reader, values, ImageCodecError, ImageCodecLimits};
use crate::image::*;

pub(super) fn payload(
    bytes: &[u8],
    limits: ImageCodecLimits,
) -> Result<InstallableImage, ImageCodecError> {
    let mut input = Reader::new(bytes, limits.max_records);
    let contracts = contracts(&mut input)?;
    let work_units = input.u64()?;
    let code = input.bytes()?.to_vec();
    let plan = crate::plan::fresh_plan_id();
    let entries = sequence(&mut input, |input| entry(input, plan))?;
    let relocations = sequence(&mut input, |input| decode_relocation(input, plan))?;
    let runtime_calls = sequence(&mut input, values::read_runtime_call)?;
    let frames = sequence(&mut input, |input| frame(input, plan))?;
    let safepoints = sequence(&mut input, |input| safepoint(input, plan))?;
    let root_requirements = sequence(&mut input, |input| root_requirement(input, plan))?;
    let heap_runtime_sites = sequence(&mut input, |input| heap_site(input, plan))?;
    let source_map = sequence(&mut input, |input| super::decode_maps::source(input, plan))?;
    let trap_map = sequence(&mut input, |input| super::decode_maps::trap(input, plan))?;
    let outcome_map = sequence(&mut input, |input| super::decode_maps::outcome(input, plan))?;
    if !input.done() {
        return Err(ImageCodecError::new("trailing image payload bytes"));
    }
    if contracts != ImageContracts::current() {
        return Err(ImageCodecError::new("decoded image contract mismatch"));
    }
    InstallableImage::new(ImageParts {
        bytes: code,
        entries,
        relocations,
        runtime_calls,
        frames,
        safepoints,
        root_requirements,
        heap_runtime_sites,
        source_map,
        trap_map,
        outcome_map,
        work_units,
        contracts,
    })
    .map_err(|_| ImageCodecError::new("decoded image integrity failure"))
}

fn contracts(input: &mut Reader<'_>) -> Result<ImageContracts, ImageCodecError> {
    Ok(ImageContracts::new(
        lkjscript_contracts::ContractDigest::from_bytes(input.fixed()?),
        lkjscript_contracts::ContractDigest::from_bytes(input.fixed()?),
        lkjscript_contracts::ContractDigest::from_bytes(input.fixed()?),
        lkjscript_contracts::ContractDigest::from_bytes(input.fixed()?),
    ))
}

fn entry(input: &mut Reader<'_>, plan: u64) -> Result<EntryMetadata, ImageCodecError> {
    Ok(entry_metadata(
        values::read_function(input, plan)?,
        SourceFunctionId::new(input.u32()?),
        values::read_signature(input)?,
        input.u32()?,
        input.u32()?,
    ))
}

fn decode_relocation(input: &mut Reader<'_>, plan: u64) -> Result<Relocation, ImageCodecError> {
    let offset = input.u32()?;
    let kind = match input.u8()? {
        0 => RelocationKind::Absolute64,
        _ => return Err(ImageCodecError::new("unknown relocation kind")),
    };
    let target = match input.u8()? {
        0 => RelocationTarget::Function(values::read_function(input, plan)?),
        1 => RelocationTarget::Runtime(values::read_runtime_call(input)?),
        _ => return Err(ImageCodecError::new("unknown relocation target")),
    };
    Ok(super::super::relocation(offset, kind, target))
}

fn frame(input: &mut Reader<'_>, plan: u64) -> Result<FrameFacts, ImageCodecError> {
    let function = values::read_function(input, plan)?;
    let frame_bytes = input.u32()?;
    let value_slots = input.u32()?;
    let local_slots = input.u32()?;
    let outgoing = input.u8()?;
    if input.bool()? || !input.bool()? {
        return Err(ImageCodecError::new("noncanonical frame ABI facts"));
    }
    let homes = sequence(input, values::read_frame_home)?;
    Ok(frame_facts(
        function,
        frame_bytes,
        value_slots,
        local_slots,
        outgoing,
        homes,
    ))
}

fn safepoint(input: &mut Reader<'_>, plan: u64) -> Result<Safepoint, ImageCodecError> {
    let id = input.u32()?;
    let function = values::read_function(input, plan)?;
    let offset = input.u32()?;
    let roots = sequence(input, read_root)?;
    Ok(exact_safepoint(id, function, offset, roots))
}

fn root_requirement(
    input: &mut Reader<'_>,
    plan: u64,
) -> Result<RootMapRequirement, ImageCodecError> {
    let id = input.u32()?;
    let function = values::read_function(input, plan)?;
    let roots = sequence(input, read_root)?;
    Ok(root_map_requirement(id, function, roots))
}

fn read_root(input: &mut Reader<'_>) -> Result<RootLocation, ImageCodecError> {
    let displacement = input.i32()?;
    let kind = match input.u8()? {
        0 => FrameHomeKind::Local(input.u32()?),
        1 => FrameHomeKind::Value(input.u32()?),
        _ => return Err(ImageCodecError::new("unknown root location kind")),
    };
    let reference = match values::read_value_type(input)? {
        ValueType::Reference(value) => value,
        _ => return Err(ImageCodecError::new("root is not a reference")),
    };
    Ok(root_location(displacement, kind, reference))
}

fn heap_site(input: &mut Reader<'_>, plan: u64) -> Result<HeapRuntimeSite, ImageCodecError> {
    let id = input.u32()?;
    let function = values::read_function(input, plan)?;
    let safepoint = input.u32()?;
    let descriptor = heap_decode::descriptor(input)?;
    let arguments = sequence(input, values::read_frame_home)?;
    let result = values::read_frame_home(input)?;
    let source = values::read_source(input)?;
    Ok(heap_runtime_site(
        id, function, safepoint, descriptor, arguments, result, source,
    ))
}

fn sequence<T>(
    input: &mut Reader<'_>,
    decode: impl Fn(&mut Reader<'_>) -> Result<T, ImageCodecError>,
) -> Result<Vec<T>, ImageCodecError> {
    let count = input.count()?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(decode(input)?);
    }
    Ok(values)
}
