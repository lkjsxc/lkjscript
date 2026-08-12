mod execution;
mod language;
mod platform;

use crate::{ContractDescriptor, ContractDigest, ContractError, ContractName, ContractSet};

pub const LANGUAGE: &str = "lkjscript.language";
pub const SOURCE: &str = "lkjscript.source";
pub const DIAGNOSTICS: &str = "lkjscript.diagnostics";
pub const TYPED_HIR: &str = "lkjscript.typed-hir";
pub const VERIFIED_SSA: &str = "lkjscript.verified-ssa";
pub const BYTECODE: &str = "lkjscript.bytecode";
pub const RUNTIME_CALLS: &str = "lkjscript.runtime-calls";
pub const NATIVE_LAYOUT: &str = "lkjscript.native-layout";
pub const METRICS: &str = "lkjscript.metrics";
pub const MEMORY_OBLIGATIONS: &str = "lkjscript.memory-obligations";
pub const STRUCTURAL_OWNERSHIP_DOMAINS: &str = "lkjscript.structural-ownership-domains";
pub const PACKAGE_MANIFEST: &str = "lkjscript.package";
pub const PACKAGE_LOCK: &str = "lkjscript.package-lock";
pub const MODULE_INTERFACE: &str = "lkjscript.module-interface";

pub const DIAGNOSTICS_DIGEST: ContractDigest = ContractDigest::from_bytes([
    0x52, 0xb8, 0x7b, 0xe8, 0xa5, 0x27, 0xac, 0xef, 0x73, 0xf7, 0xb9, 0xc5, 0xee, 0xdf, 0x48, 0x9a,
    0x2f, 0x04, 0x26, 0x7b, 0xa9, 0x77, 0x32, 0x96, 0x90, 0xd3, 0xb2, 0x52, 0xf8, 0x99, 0x4b, 0x2b,
]);
pub const METRICS_DIGEST: ContractDigest = ContractDigest::from_bytes([
    0xae, 0x73, 0x7b, 0x57, 0x9e, 0x63, 0xcb, 0xf5, 0x18, 0xed, 0x4d, 0x91, 0x76, 0x98, 0xf4, 0x22,
    0x2e, 0x87, 0x52, 0xe4, 0x47, 0x1d, 0xfb, 0x1b, 0xb1, 0x1d, 0x6e, 0xf3, 0xf3, 0xe6, 0x38, 0xec,
]);
pub const MEMORY_OBLIGATIONS_DIGEST: ContractDigest = ContractDigest::from_bytes([
    0x69, 0xbc, 0xc8, 0xca, 0xe8, 0xf5, 0x63, 0xc2, 0x06, 0xb9, 0x01, 0x09, 0x81, 0xe8, 0xfc, 0xe4,
    0xce, 0x71, 0x04, 0xdf, 0x4d, 0x5d, 0xa9, 0xe5, 0x30, 0x25, 0x8e, 0xd8, 0xb8, 0x50, 0x3c, 0x68,
]);
pub const STRUCTURAL_OWNERSHIP_DOMAINS_DIGEST: ContractDigest = ContractDigest::from_bytes([
    0x2b, 0x9e, 0x14, 0xcb, 0x64, 0x75, 0x01, 0x39, 0xe1, 0x78, 0x9a, 0x46, 0x87, 0x1d, 0x26, 0x03,
    0xb5, 0xd5, 0x54, 0x68, 0xc6, 0xd1, 0x62, 0x02, 0xc1, 0x66, 0x7e, 0x52, 0x89, 0x15, 0x40, 0xad,
]);
pub const SOURCE_DIGEST: ContractDigest = ContractDigest::from_bytes([
    0x30, 0xda, 0x29, 0xa4, 0x0c, 0x3c, 0xf6, 0x29, 0xee, 0x9b, 0x9b, 0xef, 0x8d, 0x8d, 0x1d, 0xa8,
    0xfd, 0x3b, 0xc1, 0x75, 0xc0, 0x27, 0x60, 0xf6, 0xcd, 0xdf, 0xba, 0xf5, 0xc2, 0xaf, 0x74, 0x64,
]);
pub fn current_contracts() -> Result<ContractSet, ContractError> {
    let mut set = ContractSet::new();
    let language = add(&mut set, language::language())?;
    add(&mut set, language::source(language))?;
    add(&mut set, language::diagnostics(language))?;
    let memory = add(&mut set, execution::memory_obligations(language))?;
    add(&mut set, execution::structural_ownership_domains(memory))?;
    let hir = add(&mut set, execution::typed_hir(language))?;
    let ssa = add(&mut set, execution::verified_ssa(hir))?;
    add(&mut set, execution::bytecode(ssa))?;
    let runtime = add(&mut set, execution::runtime_calls())?;
    add(&mut set, execution::native_layout(ssa, runtime))?;
    add(&mut set, execution::metrics())?;
    let manifest = add(&mut set, platform::package_manifest())?;
    let module = add(&mut set, platform::module_interface(language))?;
    add(&mut set, platform::package_lock(manifest, module))?;
    Ok(set)
}

fn add(
    set: &mut ContractSet,
    descriptor: ContractDescriptor,
) -> Result<ContractDigest, ContractError> {
    set.register(descriptor)
}

pub(crate) fn name(value: &str) -> ContractName {
    ContractName::registered(value)
}
