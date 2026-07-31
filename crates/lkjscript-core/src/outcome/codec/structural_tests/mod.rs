#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use super::*;
use crate::{
    InlineStructuralValue, LayoutIdentity, OwnedValue, SemanticPayload, SemanticTypeIdentity,
    SemanticValue, StaticStructuralLeaf, StructuralKind, StructuralSnapshotLimits, StructuralType,
};

mod byte_vector;
mod decoder_rejection;
mod owned_views;
mod round_trip;
mod symbols;

fn value(id: u64, kind: StructuralKind, payload: SemanticPayload) -> SemanticValue {
    let layout = NonZeroU64::new(id).expect("nonzero layout");
    let semantic = NonZeroU64::new(id + 100).expect("nonzero semantic type");
    SemanticValue::new(
        StructuralType::new(
            LayoutIdentity::new(layout),
            SemanticTypeIdentity::new(semantic),
            kind,
        ),
        payload,
    )
}

fn encoded(value: SemanticValue) -> Vec<u8> {
    let outcome = ExecutionOutcome::Returned(
        OwnedValue::from_structural(value, StructuralSnapshotLimits::DEFAULT)
            .expect("structural snapshot"),
    );
    encode_execution_outcome(&outcome, 2 * 1024 * 1024).expect("encode")
}
