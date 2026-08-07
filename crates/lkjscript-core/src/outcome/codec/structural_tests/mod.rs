#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use super::*;
use crate::{
    InlineStructuralValue, LayoutIdentity, OwnedValue, SemanticPayload, SemanticTypeIdentity,
    SemanticValue, StaticStructuralLeaf, StructuralKind, StructuralType,
};

mod byte_vector;
mod decoder_rejection;
mod owned_views;
mod round_trip;
mod symbols;

fn structural_type(id: u64, kind: StructuralKind) -> StructuralType {
    let layout = NonZeroU64::new(id).expect("nonzero layout");
    let semantic = NonZeroU64::new(id + 100).expect("nonzero semantic type");
    StructuralType::new(
        LayoutIdentity::new(layout),
        SemanticTypeIdentity::new(semantic),
        kind,
    )
}

fn value(id: u64, kind: StructuralKind, payload: SemanticPayload) -> SemanticValue {
    SemanticValue::new(structural_type(id, kind), payload)
}

fn encoded(value: SemanticValue) -> Vec<u8> {
    let outcome = ExecutionOutcome::Returned(
        OwnedValue::from_structural(value).expect("structural snapshot"),
    );
    encode_execution_outcome(&outcome, 2 * 1024 * 1024).expect("encode")
}
