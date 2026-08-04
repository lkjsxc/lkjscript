use crate::model::Policy;

use super::model::LimitRecord;

mod repository;
mod structural;

pub fn records(policy: &Policy) -> Vec<LimitRecord> {
    let mut records = structural::records();
    records.extend(repository::records(policy));
    records.sort_by_key(|record| record.id);
    records
}

#[allow(clippy::too_many_arguments)]
pub(super) fn record(
    id: &'static str,
    class: &'static str,
    unit: &'static str,
    scope: &'static str,
    lifetime: &'static str,
    authority: &'static str,
    value: u64,
    lower: bool,
    safety: Option<u64>,
    operation: &'static str,
    failure: &'static str,
    accounting: &'static str,
    metrics: &'static str,
    source: bool,
    wire: bool,
) -> LimitRecord {
    LimitRecord {
        id,
        class,
        unit,
        scope,
        lifetime,
        authority_path: authority,
        default_source: authority,
        value,
        host_may_lower: lower,
        validated_safety_maximum: safety,
        responsible_operation: operation,
        typed_failure: failure,
        atomicity: "failure is deterministic and mutation-atomic",
        accounting,
        metrics,
        evidence_requirements: "zero exact plus-one overflow and rollback",
        source_observable: source,
        wire_observable: wire,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affected_limit_inventory_is_sorted_and_projects_authority() -> serde_json::Result<()> {
        let policy: Policy =
            serde_json::from_str(include_str!("../../../../meta/structure/policy.json"))?;
        let records = records(&policy);
        assert_eq!(records.len(), 15);
        assert!(records.windows(2).all(|pair| pair[0].id < pair[1].id));
        assert_eq!(
            records
                .iter()
                .find(|record| record.id == "repository.graph.nodes")
                .map(|record| record.value),
            Some(policy.limits.graph_nodes)
        );
        assert_eq!(
            records
                .iter()
                .find(|record| record.id == "structural.image.tree-nodes.default")
                .map(|record| (record.value, record.class)),
            Some((65_536, "resource-profile-quota"))
        );
        Ok(())
    }
}
