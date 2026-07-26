#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CacheLimits {
    pub max_object_bytes: u64,
    pub max_objects: u64,
    pub max_total_bytes: u64,
    pub max_records: u64,
}

impl Default for CacheLimits {
    fn default() -> Self {
        Self {
            max_object_bytes: 16 * 1024 * 1024,
            max_objects: 64,
            max_total_bytes: 256 * 1024 * 1024,
            max_records: 100_000,
        }
    }
}
