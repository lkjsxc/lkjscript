use super::*;

pub(super) fn metadata_bytes(parts: MetadataSlices<'_>) -> Option<u64> {
    let mut bytes = 64_u64;
    bytes = add_records(bytes, parts.entries.len(), 32)?;
    for entry in parts.entries {
        bytes = add_records(bytes, entry.signature.parameters().len(), 1)?;
    }
    bytes = add_records(bytes, parts.relocations.len(), 24)?;
    bytes = add_records(bytes, parts.runtime_calls.len(), 8)?;
    bytes = add_records(bytes, parts.frames.len(), 32)?;
    for frame in parts.frames {
        bytes = add_records(bytes, frame.homes.len(), 16)?;
    }
    bytes = add_records(bytes, parts.safepoints.len(), 24)?;
    for safepoint in parts.safepoints {
        bytes = add_records(bytes, safepoint.stack_map.roots.len(), 16)?;
    }
    bytes = add_records(bytes, parts.root_requirements.len(), 16)?;
    for requirement in parts.root_requirements {
        bytes = add_records(bytes, requirement.roots.len(), 16)?;
    }
    bytes = add_records(bytes, parts.heap_runtime_sites.len(), 48)?;
    for site in parts.heap_runtime_sites {
        bytes = add_records(bytes, site.arguments.len(), 16)?;
        bytes = add_records(bytes, site.descriptor.input_types().len(), 1)?;
        if let crate::HeapOperation::ConstantStr(text) = site.descriptor.operation() {
            bytes = add_records(bytes, text.len(), 1)?;
        }
    }
    bytes = add_records(bytes, parts.source_map.len(), 24)?;
    bytes = add_records(bytes, parts.trap_map.len(), 16)?;
    add_records(bytes, parts.outcome_map.len(), 16)
}

fn add_records(bytes: u64, count: usize, record_bytes: u64) -> Option<u64> {
    let count = u64::try_from(count).ok()?;
    bytes.checked_add(count.checked_mul(record_bytes)?)
}
