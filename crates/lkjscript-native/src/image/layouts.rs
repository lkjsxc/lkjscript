use super::*;

pub(super) fn valid_frame_homes(frame: &FrameFacts, entry: &EntryMetadata) -> bool {
    let expected = match usize::try_from(frame.value_slots).ok().and_then(|values| {
        usize::try_from(frame.local_slots)
            .ok()
            .and_then(|locals| values.checked_add(locals))
    }) {
        Some(expected) => expected,
        None => return false,
    };
    if frame.homes.len() != expected {
        return false;
    }
    let mut kinds = HashSet::new();
    for home in &frame.homes {
        let displacement = match u32::try_from(home.rbp_displacement.checked_neg().unwrap_or(0)) {
            Ok(displacement) => displacement,
            Err(_) => return false,
        };
        if home.rbp_displacement > -16
            || home.rbp_displacement % 8 != 0
            || displacement > frame.frame_bytes
            || canonical_home_displacement(frame, home.kind) != Some(home.rbp_displacement)
            || !kinds.insert(home.kind)
        {
            return false;
        }
        match home.kind {
            FrameHomeKind::Local(index) if index < frame.local_slots => {}
            FrameHomeKind::Value(index) if index < frame.value_slots => {
                if entry
                    .signature
                    .parameters()
                    .get(index as usize)
                    .is_some_and(|parameter| *parameter != home.value_type)
                {
                    return false;
                }
            }
            FrameHomeKind::Local(_) | FrameHomeKind::Value(_) => return false,
        }
    }
    true
}

pub(super) fn canonical_home_displacement(frame: &FrameFacts, kind: FrameHomeKind) -> Option<i32> {
    let slot = match kind {
        FrameHomeKind::Local(index) => u64::from(index).checked_add(1)?,
        FrameHomeKind::Value(index) => u64::from(frame.local_slots)
            .checked_add(u64::from(index))?
            .checked_add(1)?,
    };
    let bytes = slot.checked_add(1)?.checked_mul(8)?;
    i32::try_from(bytes).ok()?.checked_neg()
}

pub(super) fn valid_stack_map(frame: &FrameFacts, map: &ExactStackMap) -> bool {
    if map.roots.windows(2).any(|pair| pair[0] >= pair[1]) {
        return false;
    }
    map.roots.iter().all(|root| {
        frame.homes.iter().any(|home| {
            home.kind == root.kind
                && home.rbp_displacement == root.rbp_displacement
                && home.value_type == ValueType::Reference(root.reference_type)
        })
    })
}

pub(super) fn offset_in_function(
    entries: &[EntryMetadata],
    function: FunctionId,
    offset: u32,
) -> bool {
    entries
        .iter()
        .any(|entry| entry.function == function && entry.offset <= offset && offset < entry.end)
}

pub(super) fn range_in_function(
    entries: &[EntryMetadata],
    function: FunctionId,
    start: u32,
    end: u32,
) -> bool {
    entries
        .iter()
        .any(|entry| entry.function == function && entry.offset <= start && end <= entry.end)
}

pub(super) struct MetadataSlices<'a> {
    pub(super) entries: &'a [EntryMetadata],
    pub(super) relocations: &'a [Relocation],
    pub(super) runtime_calls: &'a [RuntimeCallSlot],
    pub(super) frames: &'a [FrameFacts],
    pub(super) safepoints: &'a [Safepoint],
    pub(super) root_requirements: &'a [RootMapRequirement],
    pub(super) heap_runtime_sites: &'a [HeapRuntimeSite],
    pub(super) source_map: &'a [SourceMapEntry],
    pub(super) trap_map: &'a [TrapMapEntry],
    pub(super) outcome_map: &'a [OutcomeMapEntry],
}
