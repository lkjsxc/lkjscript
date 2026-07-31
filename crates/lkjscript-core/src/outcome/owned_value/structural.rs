use crate::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StructuralKind,
    StructuralSnapshotLimits, StructuralSnapshotMetrics, StructuralType,
    MAX_STRUCTURAL_SNAPSHOT_PATH_BYTES,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct OwnedStructuralValue {
    pub(super) value: SemanticValue,
    pub(super) metrics: StructuralSnapshotMetrics,
}

#[derive(Clone, Copy)]
pub(super) enum SnapshotWork {
    Encode,
    Decode,
}

impl OwnedValue {
    /// Consumes one runtime-exported semantic tree and removes all runtime-local
    /// ownership identity at the execution boundary.
    pub fn from_structural(
        value: SemanticValue,
        limits: StructuralSnapshotLimits,
    ) -> Result<Self> {
        let metrics = validate_structural_snapshot(&value, limits, SnapshotWork::Encode)?;
        Ok(Self::from_owned_structural(OwnedStructuralValue { value, metrics }))
    }

    pub fn as_structural(&self) -> Option<&SemanticValue> {
        self.structural.as_ref().map(|value| &value.value)
    }

    pub fn into_structural(self) -> Option<SemanticValue> {
        self.structural.map(|value| value.value)
    }

    pub fn structural_snapshot_metrics(&self) -> Option<StructuralSnapshotMetrics> {
        self.structural.as_ref().map(|value| value.metrics)
    }

    pub(super) fn from_owned_structural(structural: OwnedStructuralValue) -> Self {
        Self {
            root: Value::UNIT,
            lists: Vec::new(),
            unique_byte_vector: None,
            unique_bytes: None,
            symbols: Vec::new(),
            structural: Some(Box::new(structural)),
        }
    }
}
