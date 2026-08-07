//! Typed authority for canonical source-visible semantic names.

mod names;
mod operation;
mod removed;

pub use names::{
    is_identifier, BUILTIN_ERROR_NAMES, BYTE_TEXT_FOUNDATION_TYPE_NAMES, COMPILER_TRAIT_NAMES,
    CONTEXTUAL_FORM_NAMES, PRELUDE_TYPE_NAMES, PRELUDE_VARIANT_NAMES, RESERVED_WORDS,
    SIMPLE_TYPE_NAMES, TYPE_CONSTRUCTOR_NAMES,
};
pub use operation::{
    operation_by_id, operation_by_source_name, operation_semantics_by_id, OperationCategory,
    OperationEffects, OperationIdentity, OperationOwnership, OperationSemanticsRecord,
    OperationVocabularyRecord, RuntimeLowering, SemanticConstructor, OPERATION_COUNT,
};
pub use removed::{removed_spelling, RemovedSpelling, REMOVED_SPELLINGS};

#[cfg(test)]
mod tests;
