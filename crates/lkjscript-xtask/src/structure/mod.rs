mod capsule_actual;
mod commands;
mod detector;
mod graph;
mod graph_edges;
#[cfg(test)]
mod graph_tests;
mod query;
mod repository;
mod repository_support;
#[cfg(test)]
mod repository_tests;
mod rules;
mod source_facts;
mod validation;
#[cfg(test)]
mod validation_tests;

use std::path::Path;

use crate::model::{Audit, ExplainResult, Policy, ProvenanceFile};

const POLICY: &str = "meta/structure/policy.json";
const PROVENANCE: &str = "meta/structure/provenance.json";

pub fn run(root: &Path, args: &[String]) -> i32 {
    let command = args.first().map(String::as_str).unwrap_or("");
    let policy: Policy = match repository::load_json(&root.join(POLICY)) {
        Ok(value) => value,
        Err(error) => return fail(command, &error),
    };
    let provenance: ProvenanceFile = match repository::load_json(&root.join(PROVENANCE)) {
        Ok(value) => value,
        Err(error) => return fail(command, &error),
    };
    if !current_structure_contract(&policy, &provenance) {
        return fail(command, "structure policy or provenance contract mismatch");
    }
    let audit = match repository::capture(root, &provenance.entries) {
        Ok(snapshot) => rules::audit(root, &policy, provenance.entries, snapshot),
        Err(error) => return fail(command, &error),
    };
    match command {
        "audit" => audit_command(&audit, args.get(1).map(String::as_str)),
        "check" => check(&audit),
        "explain" => explain(&audit, &policy, args.get(1)),
        "graph" => commands::graph(root, &audit, &policy, args.get(1).map(String::as_str)),
        "context" | "impact" | "tests" => {
            commands::query(command, root, &audit, &policy, &args[1..])
        }
        _ => {
            usage();
            2
        }
    }
}

pub(crate) fn agent_context(
    root: &Path,
    targets: &[String],
    profile: &str,
) -> Result<Vec<crate::model::QueryResult>, String> {
    let policy: Policy = repository::load_json(&root.join(POLICY))?;
    let provenance: ProvenanceFile = repository::load_json(&root.join(PROVENANCE))?;
    if !current_structure_contract(&policy, &provenance) {
        return Err("structure policy or provenance contract mismatch".into());
    }
    let snapshot = repository::capture(root, &provenance.entries)?;
    let audit = rules::audit(root, &policy, provenance.entries, snapshot);
    let graph = graph::build(root, &audit, &policy);
    Ok(targets
        .iter()
        .map(|target| query::run("context", target, Some(profile), &graph, &policy))
        .collect())
}

fn current_structure_contract(policy: &Policy, provenance: &ProvenanceFile) -> bool {
    let expected = Some(lkjscript_contracts::REPOSITORY_GRAPH_DIGEST);
    policy.schema == "lkjscript.structure.policy"
        && lkjscript_contracts::ContractDigest::from_hex(&policy.contract) == expected
        && provenance.schema == "lkjscript.structure.provenance"
        && lkjscript_contracts::ContractDigest::from_hex(&provenance.contract) == expected
}

fn audit_command(audit: &Audit, flag: Option<&str>) -> i32 {
    if flag == Some("--json") {
        if let Err(error) = crate::util::print_json(audit) {
            eprintln!("{error}");
            return 1;
        }
    } else if flag.is_none() {
        println!(
            "revision {}: {} files, {} directories, {} findings",
            audit.revision,
            audit.files.len(),
            audit.directories.len(),
            audit.findings.len()
        );
        for finding in &audit.findings {
            println!(
                "{} {} {}: {}",
                finding.severity, finding.rule, finding.path, finding.message
            );
        }
    } else {
        eprintln!("usage: structure audit [--json]");
        return 2;
    }
    0
}

fn check(audit: &Audit) -> i32 {
    let failures = rules::check_findings(audit);
    for finding in &failures {
        eprintln!("{} {}: {}", finding.rule, finding.path, finding.message);
    }
    if failures.is_empty() {
        println!("structure check passed at {}", audit.revision);
        0
    } else {
        1
    }
}

fn explain(audit: &Audit, policy: &Policy, query: Option<&String>) -> i32 {
    let Some(query) = query else {
        eprintln!("usage: structure explain <rule-or-path>");
        return 2;
    };
    let result = ExplainResult {
        schema: "lkjscript.structure.explain".into(),
        contract: lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex(),
        query: query.clone(),
        rules: policy
            .rules
            .iter()
            .filter(|rule| rule.id == *query)
            .cloned()
            .collect(),
        files: audit
            .files
            .iter()
            .filter(|file| file.path == *query)
            .cloned()
            .collect(),
        findings: audit
            .findings
            .iter()
            .filter(|item| item.rule == *query || item.path == *query)
            .cloned()
            .collect(),
        unsupported: audit.unsupported.clone(),
    };
    crate::util::print_json(&result).map_or_else(
        |error| {
            eprintln!("{error}");
            1
        },
        |()| 0,
    )
}

fn fail(command: &str, error: &str) -> i32 {
    if command == "audit" {
        let encoded =
            serde_json::to_string(error).unwrap_or_else(|_| "\"serialization failure\"".into());
        println!(
            "{{\"schema\":\"lkjscript.repository-audit-error\",\"version\":1,\"error\":{encoded}}}"
        );
        1
    } else {
        eprintln!("{error}");
        1
    }
}

fn usage() {
    eprintln!("usage: structure [audit|check|explain|graph|context|impact|tests]");
}
