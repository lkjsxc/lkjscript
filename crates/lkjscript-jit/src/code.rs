use crate::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NumericConversionSiteCounts {
    pub f64_from_i64_exact: usize,
    pub i64_from_f64_exact: usize,
    pub i64_from_f64_trunc: usize,
}

pub(crate) struct CodeObject {
    pub(crate) functions: Vec<FunctionId>,
    pub(crate) contracts: ImageContracts,
    pub(crate) entries: Vec<EntryMetadata>,
    pub(crate) accounting: CodeAccounting,
    pub(crate) accounted_allocation_bytes: u64,
    pub(crate) relocations: Vec<Relocation>,
    pub(crate) runtime_calls: Vec<RuntimeCallSlot>,
    pub(crate) numeric_conversion_sites: NumericConversionSiteCounts,
    pub(crate) entry_stack_requirements: Vec<(FunctionId, usize)>,
    pub(crate) compile_stats: CompileStats,
    pub(crate) optimization_certificate: Option<OptimizationCertificate>,
    pub(crate) optimization_stats: Option<OptimizationStats>,
    pub(crate) explicit_traps: Vec<(u64, String)>,
    pub(crate) diagnostic_machine_code: Option<Vec<u8>>,
    pub(crate) native_entry_count: u64,
    pub(crate) installed: InstalledImage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeObjectRecord {
    pub functions: Vec<FunctionId>,
    pub contracts: ImageContracts,
    pub code_bytes: u64,
    pub metadata_bytes: u64,
    pub accounted_allocation_bytes: u64,
    pub relocation_count: usize,
    pub runtime_calls: Vec<RuntimeCallSlot>,
    pub numeric_conversion_sites: NumericConversionSiteCounts,
    pub diagnostic_machine_code: Option<Vec<u8>>,
    pub compile_stats: CompileStats,
    pub optimization_certificate: Option<OptimizationCertificate>,
    pub optimization_stats: Option<OptimizationStats>,
    pub optimization_metadata_bytes_estimate: u64,
    pub native_entry_count: u64,
    pub wx_transition_verified: bool,
}
