use super::{StructuralError, StructuralLimit};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralLimits {
    pub max_domains: u32,
    pub max_objects_per_domain: u32,
    pub max_bytes_per_domain: u64,
    pub max_chunks_per_domain: u32,
    pub chunk_objects: u32,
    pub large_object_bytes: u32,
    pub max_dependencies: u32,
    pub max_drop_entries: u32,
    pub max_release_work: u32,
    pub max_region_owners: u32,
    pub max_pool_slots: u32,
    pub max_generation: u32,
}

impl StructuralLimits {
    pub fn validate(self) -> Result<Self, StructuralError> {
        let values = [
            self.max_domains,
            self.max_objects_per_domain,
            self.max_chunks_per_domain,
            self.chunk_objects,
            self.large_object_bytes,
            self.max_dependencies,
            self.max_drop_entries,
            self.max_release_work,
            self.max_region_owners,
            self.max_pool_slots,
            self.max_generation,
        ];
        if values.contains(&0) || self.max_bytes_per_domain == 0 {
            return Err(StructuralError::LimitExceeded(StructuralLimit::Domains));
        }
        Ok(self)
    }
}

impl Default for StructuralLimits {
    fn default() -> Self {
        Self {
            max_domains: 4_096,
            max_objects_per_domain: 65_536,
            max_bytes_per_domain: 64 * 1024 * 1024,
            max_chunks_per_domain: 1_024,
            chunk_objects: 256,
            large_object_bytes: 16 * 1024,
            max_dependencies: 4_096,
            max_drop_entries: 4_096,
            max_release_work: 8_192,
            max_region_owners: 65_536,
            max_pool_slots: 1_048_576,
            max_generation: u32::MAX,
        }
    }
}
