use super::*;

pub(super) fn verify_entries(plan: &HirMemoryPlan) -> Result<()> {
    for entry in &plan.entries {
        if entry.mode.escape == MemoryEscape::Returned
            && matches!(entry.ty, MemoryType::ByteSlice | MemoryType::ByteSliceMut)
        {
            return Err(Error::msg("borrowed HIR result escaped its function"));
        }
        if matches!(entry.ty, MemoryType::ByteVector)
            && (entry.drop_glue != Some(MemoryDropGlueId::new(0)) || !is_unique_byte_vector(entry))
        {
            return Err(Error::msg(
                "byte-vector memory plan is not deterministic unique storage",
            ));
        }
        if matches!(
            entry.ty,
            MemoryType::Symbol | MemoryType::Function { .. } | MemoryType::ForAll { .. }
        ) && !is_static_artifact_value(entry)
        {
            return Err(Error::msg(
                "artifact value memory plan is not static and trivial",
            ));
        }
        if matches!(entry.ty, MemoryType::Bytes) {
            let expected = if entry.mode.domain == MemoryDomain::Static {
                None
            } else {
                Some(MemoryDropGlueId::new(1 + ResourceKind::ALL.len() as u32))
            };
            if entry.drop_glue != expected {
                return Err(Error::msg("bytes memory plan has wrong drop glue"));
            }
        } else if let MemoryType::Resource(kind) = entry.ty {
            let expected = MemoryDropGlueId::new(1 + u32::from(kind as u8));
            if entry.drop_glue != Some(expected) {
                return Err(Error::msg("resource memory plan has wrong drop glue"));
            }
        }
    }
    Ok(())
}

fn is_unique_byte_vector(entry: &MemoryPlanEntry) -> bool {
    entry.mode.multiplicity == MemoryMultiplicity::Affine
        && entry.mode.aliasing == MemoryAliasing::Unique
        && entry.mode.domain == MemoryDomain::UniqueStructural
        && entry.mode.destruction == MemoryDestruction::DropGlue
        && entry.mode.identity == MemoryIdentity::Value
        && entry.mode.portability == MemoryPortability::WorkerLocal
        && entry.mode.contention == MemoryContention::SingleOwner
        && entry.legacy_family.is_none()
}

fn is_static_artifact_value(entry: &MemoryPlanEntry) -> bool {
    entry.mode.multiplicity == MemoryMultiplicity::Copy
        && entry.mode.aliasing == MemoryAliasing::StaticShared
        && entry.mode.domain == MemoryDomain::Static
        && entry.mode.destruction == MemoryDestruction::Trivial
        && entry.mode.identity == MemoryIdentity::Value
        && entry.mode.portability == MemoryPortability::WorkerLocal
        && entry.mode.contention == MemoryContention::ImmutableShared
        && entry.drop_glue.is_none()
        && entry.legacy_family.is_none()
}
