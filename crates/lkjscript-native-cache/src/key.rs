use lkjscript_native::BackendLimits;

use crate::{ArtifactKey, CacheContext, CacheError, CacheTier};

pub fn artifact_key(
    context: &CacheContext,
    ssa_sha256: [u8; 32],
    tier: CacheTier,
    group_root: u32,
    optimization_policy_sha256: [u8; 32],
    backend: BackendLimits,
) -> Result<ArtifactKey, CacheError> {
    let contracts = lkjscript_contracts::current_contracts()
        .map_err(|error| CacheError::message(format!("contract registry: {error}")))?;
    let mut bytes = Vec::new();
    field(&mut bytes, b"lkjscript.native-image-key")?;
    for name in [
        lkjscript_contracts::NATIVE_IMAGE_CACHE,
        lkjscript_contracts::LANGUAGE,
        lkjscript_contracts::SOURCE,
        lkjscript_contracts::TYPED_HIR,
        lkjscript_contracts::VERIFIED_SSA,
        lkjscript_contracts::BYTECODE,
        lkjscript_contracts::RESOURCE_CATEGORIES,
        lkjscript_contracts::RESOURCE_PROFILES,
        lkjscript_contracts::PACKAGE_MANIFEST,
        lkjscript_contracts::PACKAGE_LOCK,
        lkjscript_contracts::MODULE_INTERFACE,
        lkjscript_contracts::RUNTIME_CALLS,
        lkjscript_contracts::NATIVE_LAYOUT,
    ] {
        let digest = contracts
            .get(name)
            .ok_or_else(|| CacheError::message("cache key contract is absent"))?
            .digest();
        field(&mut bytes, &digest.as_bytes())?;
    }
    field(&mut bytes, context.module_path.as_bytes())?;
    field(&mut bytes, &context.source_sha256)?;
    field(&mut bytes, &context.module_sha256)?;
    field(&mut bytes, &context.package_sha256)?;
    field(&mut bytes, &context.lock_sha256)?;
    field(&mut bytes, &ssa_sha256)?;
    profile(&mut bytes, context.profile)?;
    field(&mut bytes, &[tier_tag(tier)])?;
    field(&mut bytes, &group_root.to_be_bytes())?;
    field(&mut bytes, &optimization_policy_sha256)?;
    field(&mut bytes, b"lkjscript-native-linux-x86_64")?;
    field(
        &mut bytes,
        &lkjscript_contracts::NATIVE_LAYOUT_DIGEST.as_bytes(),
    )?;
    backend_limits(&mut bytes, backend)?;
    for target in [
        b"linux".as_slice(),
        b"x86_64",
        b"sysv",
        b"little-endian",
        b"pointer-width-64",
        b"cpu-features-none",
    ] {
        field(&mut bytes, target)?;
    }
    Ok(ArtifactKey::new(lkjscript_contracts::sha256(&bytes)))
}

fn profile(
    bytes: &mut Vec<u8>,
    value: lkjscript_core::ResourceProfileIdentity,
) -> Result<(), CacheError> {
    field(bytes, value.schema.as_bytes())?;
    field(bytes, &value.contract.as_bytes())?;
    field(bytes, value.name.as_str().as_bytes())?;
    field(bytes, &value.resource_categories.as_bytes())?;
    field(bytes, &value.implementation_maxima_sha256)?;
    field(bytes, &value.ceilings_sha256)?;
    field(
        bytes,
        value
            .host_lowered_ceilings_sha256
            .as_ref()
            .map_or(&[][..], <[u8; 32]>::as_slice),
    )
}

fn backend_limits(bytes: &mut Vec<u8>, value: BackendLimits) -> Result<(), CacheError> {
    for number in [
        to_u64(value.max_functions())?,
        to_u64(value.max_blocks())?,
        to_u64(value.max_values())?,
        to_u64(value.max_locals_per_function())?,
        to_u64(value.max_code_bytes())?,
        value.max_metadata_bytes(),
        value.max_work_units(),
    ] {
        field(bytes, &number.to_be_bytes())?;
    }
    Ok(())
}

fn field(bytes: &mut Vec<u8>, value: &[u8]) -> Result<(), CacheError> {
    let length =
        u64::try_from(value.len()).map_err(|_| CacheError::message("key field overflow"))?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value);
    Ok(())
}

fn to_u64(value: usize) -> Result<u64, CacheError> {
    u64::try_from(value).map_err(|_| CacheError::message("backend limit overflow"))
}

const fn tier_tag(tier: CacheTier) -> u8 {
    match tier {
        CacheTier::Baseline => 0,
        CacheTier::Optimizing => 1,
    }
}
