use super::*;

pub(super) fn metadata_bytes(parts: MetadataSlices<'_>) -> Option<u64> {
    let mut bytes = 64_u64;
    bytes = add_records(bytes, parts.static_bytes.len(), 8)?;
    for value in parts.static_bytes {
        bytes = add_records(bytes, value.len(), 1)?;
    }
    bytes = add_records(bytes, parts.entries.len(), 32)?;
    for entry in parts.entries {
        bytes = add_records(bytes, entry.signature.parameters().len(), 1)?;
    }
    bytes = add_records(bytes, parts.relocations.len(), 24)?;
    bytes = add_records(bytes, parts.runtime_calls.len(), 8)?;
    bytes = add_records(bytes, parts.frames.len(), 32)?;
    for frame in parts.frames {
        bytes = add_records(bytes, frame.homes.len(), 16)?;
        bytes = add_records(bytes, frame.returned_structural_owners.len(), 8)?;
    }
    bytes = add_records(bytes, parts.heap_runtime_sites.len(), 48)?;
    for site in parts.heap_runtime_sites {
        bytes = add_records(bytes, site.arguments.len(), 16)?;
        bytes = add_records(bytes, site.descriptor.input_types().len(), 1)?;
    }
    bytes = add_records(bytes, parts.structural_runtime_sites.len(), 96)?;
    for site in parts.structural_runtime_sites {
        use crate::StructuralOperation as Op;
        match site.descriptor.operation() {
            Op::Borrow { projection } => {
                bytes = add_records(bytes, projection.path().len(), 2)?;
            }
            Op::DestinationCreate { aggregate, .. }
            | Op::DestinationFinish { aggregate, .. }
            | Op::DestinationInitialize { aggregate, .. }
            | Op::DestinationAbort { aggregate, .. } => {
                bytes = add_records(bytes, aggregate.fields().len(), 24)?;
            }
            _ => {}
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
