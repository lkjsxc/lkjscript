use super::*;

pub(super) fn check_object_limit(
    actual: u64,
    maximum: u64,
    kind: ExecutableLimitKind,
) -> Result<(), InstallError> {
    if actual > maximum {
        return Err(InstallError::LimitExceeded(kind));
    }
    Ok(())
}

pub(super) fn checked_usage(
    current: ExecutableUsage,
    accounting: lkjscript_native::CodeAccounting,
    limits: ExecutableLimits,
) -> Result<ExecutableUsage, InstallError> {
    let next = ExecutableUsage {
        code_bytes: current
            .code_bytes
            .checked_add(accounting.code_bytes())
            .ok_or(InstallError::LimitExceeded(
                ExecutableLimitKind::TotalCodeBytes,
            ))?,
        metadata_bytes: current
            .metadata_bytes
            .checked_add(accounting.metadata_bytes())
            .ok_or(InstallError::LimitExceeded(
                ExecutableLimitKind::TotalMetadataBytes,
            ))?,
        work_units: current
            .work_units
            .checked_add(accounting.work_units())
            .ok_or(InstallError::LimitExceeded(
                ExecutableLimitKind::TotalWorkUnits,
            ))?,
        objects: current
            .objects
            .checked_add(1)
            .ok_or(InstallError::LimitExceeded(
                ExecutableLimitKind::ObjectCount,
            ))?,
    };
    check_object_limit(
        next.code_bytes,
        limits.max_total_code_bytes,
        ExecutableLimitKind::TotalCodeBytes,
    )?;
    check_object_limit(
        next.metadata_bytes,
        limits.max_total_metadata_bytes,
        ExecutableLimitKind::TotalMetadataBytes,
    )?;
    check_object_limit(
        next.work_units,
        limits.max_total_work_units,
        ExecutableLimitKind::TotalWorkUnits,
    )?;
    check_object_limit(
        next.objects,
        limits.max_objects,
        ExecutableLimitKind::ObjectCount,
    )?;
    Ok(next)
}
