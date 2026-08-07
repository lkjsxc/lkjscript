use crate::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NumericConversionSiteCounts {
    pub f64_from_i64_exact: usize,
    pub i64_from_f64_exact: usize,
    pub i64_from_f64_trunc: usize,
}

pub struct CodeObject {
    pub(crate) identity: u64,
    pub(crate) functions: Vec<FunctionId>,
    pub(crate) tier: Tier,
    pub(crate) contracts: ImageContracts,
    pub(crate) entries: Vec<EntryMetadata>,
    pub(crate) accounting: CodeAccounting,
    pub(crate) accounted_allocation_bytes: u64,
    pub(crate) relocations: Vec<Relocation>,
    pub(crate) runtime_calls: Vec<RuntimeCallSlot>,
    pub(crate) numeric_conversion_sites: NumericConversionSiteCounts,
    pub(crate) frames: Vec<FrameFacts>,
    pub(crate) automatic_stack_requirements: Vec<(FunctionId, usize)>,
    pub(crate) source_map: Vec<SourceMapEntry>,
    pub(crate) trap_map: Vec<TrapMapEntry>,
    pub(crate) outcome_map: Vec<OutcomeMapEntry>,
    pub(crate) compile_stats: CompileStats,
    pub(crate) optimization_certificate: Option<OptimizationCertificate>,
    pub(crate) optimization_stats: Option<OptimizationStats>,
    pub(crate) invalidated: bool,
    pub(crate) explicit_traps: Vec<(u64, String)>,
    pub(crate) diagnostic_machine_code: Option<Vec<u8>>,
    pub(crate) native_entry_count: u64,
    pub(crate) installed: InstalledImage,
}

impl CodeObject {
    pub const fn identity(&self) -> u64 {
        self.identity
    }

    pub fn functions(&self) -> &[FunctionId] {
        &self.functions
    }

    pub const fn tier(&self) -> Tier {
        self.tier
    }

    pub const fn contracts(&self) -> ImageContracts {
        self.contracts
    }

    pub fn entries(&self) -> &[EntryMetadata] {
        &self.entries
    }

    pub const fn accounting(&self) -> CodeAccounting {
        self.accounting
    }

    pub const fn accounted_allocation_bytes(&self) -> u64 {
        self.accounted_allocation_bytes
    }

    pub fn relocations(&self) -> &[Relocation] {
        &self.relocations
    }

    pub fn runtime_calls(&self) -> &[RuntimeCallSlot] {
        &self.runtime_calls
    }

    pub fn frames(&self) -> &[FrameFacts] {
        &self.frames
    }

    pub fn source_map(&self) -> &[SourceMapEntry] {
        &self.source_map
    }

    pub fn trap_map(&self) -> &[TrapMapEntry] {
        &self.trap_map
    }

    pub fn outcome_map(&self) -> &[OutcomeMapEntry] {
        &self.outcome_map
    }

    pub const fn compile_stats(&self) -> &CompileStats {
        &self.compile_stats
    }

    pub fn optimization_certificate(&self) -> Option<&OptimizationCertificate> {
        self.optimization_certificate.as_ref()
    }

    pub const fn optimization_stats(&self) -> Option<&OptimizationStats> {
        self.optimization_stats.as_ref()
    }

    pub const fn invalidated(&self) -> bool {
        self.invalidated
    }

    pub fn diagnostic_machine_code(&self) -> Option<&[u8]> {
        self.diagnostic_machine_code.as_deref()
    }

    pub const fn native_entry_count(&self) -> u64 {
        self.native_entry_count
    }

    pub fn wx_transition_verified(&self) -> bool {
        self.installed.wx_transition_verified()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeObjectRecord {
    pub identity: u64,
    pub functions: Vec<FunctionId>,
    pub tier: Tier,
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
    pub invalidated: bool,
    pub native_entry_count: u64,
    pub wx_transition_verified: bool,
}
