use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImageIntegrityError {
    EmptyCode,
    CodeAccountingMismatch,
    MetadataAccountingMismatch,
    StaticBytes,
    EntryRange,
    DuplicateEntry,
    RelocationRange,
    RelocationTarget,
    FrameFacts,
    Safepoint,
    RootRequirement,
    HeapRuntimeSite,
    StructuralRuntimeSite,
    SourceMap,
    TrapMap,
    OutcomeMap,
    RuntimeCallSet,
    ExecutionDomain,
}

impl fmt::Display for ImageIntegrityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyCode => "installable image has no code",
            Self::CodeAccountingMismatch => "installable image code accounting is inconsistent",
            Self::MetadataAccountingMismatch => {
                "installable image metadata accounting is inconsistent"
            }
            Self::StaticBytes => "installable image static bytes data is invalid",
            Self::EntryRange => "installable image entry range is invalid",
            Self::DuplicateEntry => "installable image has duplicate entries",
            Self::RelocationRange => "installable image relocation range is invalid",
            Self::RelocationTarget => "installable image relocation target is invalid",
            Self::FrameFacts => "installable image frame facts are invalid",
            Self::Safepoint => "installable image safepoint is invalid",
            Self::RootRequirement => {
                "installable image stack map disagrees with its verifier requirement"
            }
            Self::HeapRuntimeSite => "installable image heap runtime site is invalid",
            Self::StructuralRuntimeSite => "installable image structural runtime site is invalid",
            Self::SourceMap => "installable image source map is invalid",
            Self::TrapMap => "installable image trap map is invalid",
            Self::OutcomeMap => "installable image outcome map is invalid",
            Self::RuntimeCallSet => "installable image runtime-call set is invalid",
            Self::ExecutionDomain => "installable image execution domain is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ImageIntegrityError {}
