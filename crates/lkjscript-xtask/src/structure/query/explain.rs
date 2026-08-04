use crate::model::{Audit, ExplainResult, FactExplanation, Policy};
use crate::public_facts::{Authority, Registry};

pub fn run(
    audit: &Audit,
    policy: &Policy,
    registry: &Registry,
    graph_identity: &str,
    query: &str,
) -> ExplainResult {
    let unsupported = audit.unsupported.clone();
    let facts = fact_explanations(query, registry);
    ExplainResult {
        schema: "lkjscript.structure.explain".into(),
        contract: lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex(),
        graph_identity: graph_identity.into(),
        query: query.into(),
        rules: policy
            .rules
            .iter()
            .filter(|rule| rule.id == query)
            .cloned()
            .collect(),
        files: audit
            .files
            .iter()
            .filter(|file| file.path == query)
            .cloned()
            .collect(),
        facts,
        findings: audit
            .findings
            .iter()
            .filter(|item| item.rule == query || item.path == query)
            .cloned()
            .collect(),
        unsupported,
    }
}

fn fact_explanations(
    query: &str,
    registry: &crate::public_facts::Registry,
) -> Vec<FactExplanation> {
    let id = query.strip_prefix("fact:").unwrap_or(query);
    registry
        .facts
        .get(id)
        .map(|located| {
            let authority = match &located.fact.authority {
                Authority::RepositoryPath { path } => path.clone(),
                Authority::MachineSource { source } => format!("machine:{}", source.name()),
            };
            FactExplanation {
                id: located.fact.id.clone(),
                status: located.fact.status.name().into(),
                digest: located.digest.clone(),
                interface: located.fact.interface.clone(),
                exclusions: located
                    .fact
                    .exclusions
                    .iter()
                    .map(|value| value.interface.clone())
                    .collect(),
                authority,
                evidence: located
                    .fact
                    .evidence
                    .iter()
                    .map(|value| value.path.clone())
                    .collect(),
                projections: located.fact.projections.clone(),
            }
        })
        .into_iter()
        .collect()
}
