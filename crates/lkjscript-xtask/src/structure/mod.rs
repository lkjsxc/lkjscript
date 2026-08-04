mod capsule_actual;
mod commands;
mod detector;
mod graph;
mod graph_edges;
#[cfg(test)]
mod graph_tests;
mod query;
mod repository;
pub(crate) mod repository_support;
#[cfg(test)]
mod repository_tests;
mod rules;
mod source_facts;
mod validation;
pub(crate) use commands::agent_context;
#[cfg(test)]
mod validation_tests;

use std::path::Path;

use crate::model::{Audit, Policy, ProvenanceFile};

const INPUTS: (&str, &str) = (
    "meta/structure/policy.json",
    "meta/structure/provenance.json",
);

pub fn run(root: &Path, args: &[String]) -> i32 {
    let command = args.first().map(String::as_str).unwrap_or("");
    let policy: Policy = match repository::load_json(&root.join(INPUTS.0)) {
        Ok(value) => value,
        Err(error) => return fail(command, &error),
    };
    let provenance: ProvenanceFile = match repository::load_json(&root.join(INPUTS.1)) {
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
    let registry = if matches!(
        command,
        "explain" | "graph" | "context" | "impact" | "tests"
    ) {
        match crate::public_facts::load(root) {
            Ok(value) => Some(value),
            Err(error) => return fail(command, &format!("public facts unavailable: {error}")),
        }
    } else {
        None
    };
    match command {
        "audit" => audit_command(&audit, args.get(1).map(String::as_str)),
        "check" => check(&audit),
        "explain" => explain(root, &audit, &policy, registry.as_ref(), args.get(1)),
        "graph" => commands::graph(
            root,
            &audit,
            &policy,
            registry.as_ref(),
            args.get(1).map(String::as_str),
        ),
        "context" | "impact" | "tests" => commands::query(
            command,
            root,
            &audit,
            &policy,
            registry.as_ref(),
            &args[1..],
        ),
        _ => {
            usage();
            2
        }
    }
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

fn explain(
    root: &Path,
    audit: &Audit,
    policy: &Policy,
    registry: Option<&crate::public_facts::Registry>,
    query: Option<&String>,
) -> i32 {
    let Some(query) = query else {
        eprintln!("usage: structure explain <rule-path-or-fact>");
        return 2;
    };
    let Some(registry) = registry else {
        eprintln!("public facts unavailable");
        return 1;
    };
    let graph = graph::build_with_facts(root, audit, policy, registry);
    let result = query::explain(audit, policy, registry, &graph.input_identity, query);
    crate::util::print_json_bounded(&result, policy.limits.query_bytes).map_or_else(
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
            "{{\"schema\":\"lkjscript.repository-audit-error\",\"contract\":\"{}\",\"error\":{encoded}}}",
            lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex()
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
