use super::super::{StructuralLimits, StructuralRootTableLimits};
use super::{StructuralValueError, StructuralValueLimit};

pub const DEFAULT_STRUCTURAL_TREE_NODES: u64 = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralValueRuntimeLimits {
    pub domains: StructuralLimits,
    pub roots: StructuralRootTableLimits,
    pub max_objects: u32,
    pub max_destinations: u32,
    pub max_views: u32,
    pub max_tree_nodes: u64,
    pub max_tree_depth: u16,
    pub max_fields: usize,
    pub max_payload_bytes: u64,
    pub max_events: u32,
    pub max_cleanup_reports: u32,
    pub max_generation: u32,
}

impl StructuralValueRuntimeLimits {
    pub fn validate(self) -> Result<Self, StructuralValueError> {
        self.domains.validate()?;
        self.roots.validate()?;
        let values = [
            self.max_objects,
            self.max_destinations,
            self.max_views,
            u32::from(self.max_tree_depth),
            self.max_events,
            self.max_cleanup_reports,
            self.max_generation,
        ];
        if values.contains(&0)
            || self.max_tree_nodes == 0
            || self.max_fields == 0
            || self.max_payload_bytes == 0
        {
            return Err(StructuralValueError::InvalidLimits);
        }
        if self.max_objects > self.domains.max_domains {
            return Err(StructuralValueError::LimitExceeded(
                StructuralValueLimit::Objects,
            ));
        }
        Ok(self)
    }
}

impl Default for StructuralValueRuntimeLimits {
    fn default() -> Self {
        Self {
            domains: StructuralLimits::default(),
            roots: StructuralRootTableLimits::default(),
            max_objects: 4_096,
            max_destinations: 1_024,
            max_views: 4_096,
            max_tree_nodes: DEFAULT_STRUCTURAL_TREE_NODES,
            max_tree_depth: 64,
            max_fields: 1_024,
            max_payload_bytes: 64 * 1024 * 1024,
            max_events: 4_096,
            max_cleanup_reports: 256,
            max_generation: u32::MAX,
        }
    }
}
