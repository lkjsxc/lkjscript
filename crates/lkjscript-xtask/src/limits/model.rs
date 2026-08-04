use serde::Serialize;

#[derive(Serialize)]
pub struct LimitInventory {
    pub schema: &'static str,
    pub contract: String,
    pub records: Vec<LimitRecord>,
}

#[derive(Serialize)]
pub struct LimitRecord {
    pub id: &'static str,
    pub class: &'static str,
    pub unit: &'static str,
    pub scope: &'static str,
    pub lifetime: &'static str,
    pub authority_path: &'static str,
    pub default_source: &'static str,
    pub value: u64,
    pub host_may_lower: bool,
    pub validated_safety_maximum: Option<u64>,
    pub responsible_operation: &'static str,
    pub typed_failure: &'static str,
    pub atomicity: &'static str,
    pub accounting: &'static str,
    pub metrics: &'static str,
    pub evidence_requirements: &'static str,
    pub source_observable: bool,
    pub wire_observable: bool,
}
