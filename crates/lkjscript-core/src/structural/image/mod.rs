mod access;
mod builder;
mod children;
mod facts;
mod merge;
mod model;
mod observation;
mod semantic;
mod validation;

pub use children::SemanticChildren;
pub use model::{
    CheckedU64Range, LocalNodeId, StructuralImage, StructuralNode, StructuralNodePayload,
    StructuralNodeRecord, StructuralNodeView,
};

pub(crate) use builder::{discard_semantic, prepare_discard};
pub(crate) use facts::TreeFacts;
pub(crate) use semantic::{require_kind, semantic_facts};

use super::value_runtime::{SemanticValue, StructuralValueError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralImageConversionFailure {
    pub error: StructuralValueError,
    pub value: SemanticValue,
}

impl StructuralImage {
    #[allow(clippy::result_large_err)]
    pub fn from_owned(value: SemanticValue) -> Result<Self, StructuralImageConversionFailure> {
        let facts = match semantic_facts(&value) {
            Ok(facts) => facts,
            Err(error) => return Err(StructuralImageConversionFailure { error, value }),
        };
        let mut discard = match prepare_discard(facts) {
            Ok(stack) => stack,
            Err(error) => return Err(StructuralImageConversionFailure { error, value }),
        };
        let image = match Self::build(&value, facts) {
            Ok(image) => image,
            Err(error) => return Err(StructuralImageConversionFailure { error, value }),
        };
        discard_semantic(value, &mut discard);
        Ok(image)
    }
}
