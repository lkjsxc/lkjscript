use super::*;

pub(crate) fn entry_metadata(
    function: FunctionId,
    source_function: SourceFunctionId,
    signature: Signature,
    offset: u32,
    end: u32,
) -> EntryMetadata {
    EntryMetadata {
        function,
        source_function,
        signature,
        offset,
        end,
    }
}

pub(crate) fn relocation(
    offset: u32,
    kind: RelocationKind,
    target: RelocationTarget,
) -> Relocation {
    Relocation {
        offset,
        kind,
        target,
    }
}

pub(crate) fn frame_facts(
    function: FunctionId,
    frame_bytes: u32,
    value_slots: u64,
    local_slots: u64,
    outgoing_machine_arguments: u8,
    homes: Vec<FrameHome>,
    returned_structural_owners: Vec<FrameHomeKind>,
) -> FrameFacts {
    FrameFacts {
        function,
        frame_bytes,
        value_slots,
        local_slots,
        outgoing_machine_arguments,
        uses_red_zone: false,
        call_site_aligned_16: true,
        homes,
        returned_structural_owners,
    }
}

pub(crate) const fn frame_home(
    kind: FrameHomeKind,
    value_type: ValueType,
    rbp_displacement: i32,
) -> FrameHome {
    FrameHome {
        kind,
        value_type,
        rbp_displacement,
    }
}

pub(crate) fn heap_runtime_site(
    id: u64,
    function: FunctionId,
    descriptor: HeapCallDescriptor,
    arguments: Vec<FrameHome>,
    result: FrameHome,
    source: Option<SourceOrigin>,
) -> HeapRuntimeSite {
    HeapRuntimeSite {
        id,
        function,
        descriptor,
        arguments,
        result,
        source,
    }
}

pub(crate) fn structural_runtime_site(
    id: u64,
    function: FunctionId,
    descriptor: StructuralCallDescriptor,
    source: Option<SourceOrigin>,
) -> StructuralRuntimeSite {
    StructuralRuntimeSite {
        id,
        function,
        descriptor,
        source,
    }
}

pub(crate) fn source_map_entry(
    function: FunctionId,
    code_start: u32,
    code_end: u32,
    source: Option<SourceOrigin>,
) -> SourceMapEntry {
    SourceMapEntry {
        function,
        code_start,
        code_end,
        source,
    }
}

pub(crate) fn trap_map_entry(
    function: FunctionId,
    code_offset: u32,
    trap: TrapCode,
    site: Option<u64>,
) -> TrapMapEntry {
    TrapMapEntry {
        function,
        code_offset,
        trap,
        site,
    }
}

pub(crate) fn outcome_map_entry(
    function: FunctionId,
    code_offset: u32,
    outcome: OutcomeKind,
) -> OutcomeMapEntry {
    OutcomeMapEntry {
        function,
        code_offset,
        outcome,
    }
}
