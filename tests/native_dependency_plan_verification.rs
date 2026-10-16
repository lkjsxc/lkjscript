//! A separately built native verifier checks untrusted results and is then consumed
//! through an exact exported package by both native planning implementations.
//! Host matrices share prepared code; candidate runs and bounded detached smoke runs
//! execute the copied binary. Execution mode and cache observations remain explicit.
#![allow(clippy::unwrap_used, clippy::panic)]
use crate::{components, plan};
#[path = "native_plan_verify_support/mod.rs"]
mod verification;
use components::matrix::Case;
use components::{Native, digest, quoted};
use serde_json::json;
use std::fs;

#[test]
fn native_checker_rejects_false_plans_and_guards_an_independent_producer()
-> Result<(), Box<dyn std::error::Error>> {
    let public = Native::in_memory();
    let executable_hash = digest(&public.root.join("lkjscript"));
    verification::author(&public);
    let verifier_artifact = public.root.join("dependency-plan-verifier.lkja");
    let producer_artifact = public.root.join("verified-dependency-plan.lkja");
    let verifier_hash = digest(&verifier_artifact);
    let producer_hash = digest(&producer_artifact);
    let verifier_transport_hash = digest(&public.root.join("verifier.lkjp"));
    let descriptors = verification::DESCRIPTORS
        .map(|(name, source)| public.input(&format!("{name}.deployment.json"), source));
    let storage_descriptors = verification::STORAGE_DESCRIPTORS
        .map(|(name, source)| public.input(&format!("{name}.deployment.json"), source));
    let single_modes: Vec<_> = ["flat", "chunked"]
        .into_iter()
        .zip(&descriptors[..2])
        .chain(
            plan::STORAGE_MODES
                .into_iter()
                .zip(&storage_descriptors[..4]),
        )
        .collect();
    let batch_modes: Vec<_> = ["flat", "chunked"]
        .into_iter()
        .zip(&descriptors[2..4])
        .chain(
            plan::STORAGE_MODES
                .into_iter()
                .zip(&storage_descriptors[4..]),
        )
        .collect();
    let proposals = plan::cases::suite();
    let expected: Vec<_> = proposals
        .iter()
        .map(|p| (p.clone(), plan::oracle::expected(p)))
        .collect();
    let normal = verification::cases::normal(&proposals);
    let plans = verification::batches(&public, "plans", &expected)?;
    let claims = verification::batches(&public, "claims", &normal)?;
    let exhaustive = verification::cases::exhaustive();
    let candidates = verification::batches(&public, "candidates", &exhaustive)?;
    let mutations = verification::mutations::suite();
    public.input(
        "mutation-index.json",
        &json!(
            mutations
                .iter()
                .map(|(name, _, expected)| json!({"name":name,"expected":expected}))
                .collect::<Vec<_>>()
        )
        .to_string(),
    );
    let mutation_samples: Vec<_> = mutations
        .iter()
        .map(|(_, input, expected)| (input.clone(), expected.clone()))
        .collect();
    let adversaries = verification::batches(&public, "mutations", &mutation_samples)?;
    let outcome_suite = verification::outcomes::Suite::new(&public)?;
    let example = components::cases::example();
    let example_expected = plan::oracle::expected(&example);
    let sample_arguments =
        public.input("attached-plan-sample.json", &json!([[example]]).to_string());
    let sample_expected = json!([example_expected]);
    let mut attached_observations = Vec::new();
    for &(mode, descriptor) in &batch_modes {
        let output = public.compare(
            &format!("attached-plan-sample-{mode}"),
            descriptor,
            &sample_arguments,
            &sample_expected,
        );
        attached_observations.push(json!({"phase":"attached-sample","mode":mode,"conditions":1,
            "observation":quoted(&output,"production-observation")}));
    }
    let sample_claim = verification::cases::normal(std::slice::from_ref(&example));
    let claim_arguments = public.input(
        "attached-claim-sample.json",
        &json!([[sample_claim[0].0]]).to_string(),
    );
    let claim_expected = json!([sample_claim[0].1]);
    let output = public.compare(
        "attached-claim-sample",
        &descriptors[5],
        &claim_arguments,
        &claim_expected,
    );
    attached_observations.push(
        json!({"phase":"attached-sample","mode":"verify","conditions":1,
        "observation":quoted(&output,"production-observation")}),
    );
    attached_observations.push(outcome_suite.attached_sample(&public));
    public.remove_project("verifier");
    fs::remove_file(public.root.join("verifier.lkjp")).unwrap();
    plan::detach(&public);
    public.matrix_boundary(
        "producer-detached-ordinary-boundary",
        &Case {
            name: "producer-detached-ordinary-boundary".to_owned(),
            descriptor: batch_modes[0].1.clone(),
            arguments: sample_arguments,
            expected: sample_expected,
        },
    );
    public.matrix_boundary(
        "verifier-detached-ordinary-boundary",
        &Case {
            name: "verifier-detached-ordinary-boundary".to_owned(),
            descriptor: descriptors[5].clone(),
            arguments: claim_arguments,
            expected: claim_expected,
        },
    );
    let mut matrix = Vec::new();
    let mut observations = Vec::new();
    for &(mode, descriptor) in &batch_modes {
        for (index, batch) in plans.iter().enumerate() {
            matrix.push(Case {
                name: format!("source-deleted-{mode}-{index:03}"),
                descriptor: descriptor.clone(),
                arguments: batch.arguments.clone(),
                expected: batch.expected.clone(),
            });
            observations.push(json!({"phase":"source-deleted","mode":mode,"batch":index,"conditions":batch.count}));
        }
    }
    for (index, batch) in claims.iter().enumerate() {
        matrix.push(Case {
            name: format!("source-deleted-verify-{index:03}"),
            descriptor: descriptors[5].clone(),
            arguments: batch.arguments.clone(),
            expected: batch.expected.clone(),
        });
        observations.push(json!({"phase":"source-deleted","mode":"verify","batch":index,"conditions":batch.count}));
    }
    for (case, observation) in outcome_suite.matrix_cases() {
        matrix.push(case);
        observations.push(observation);
    }
    for (family, batches) in [("candidates", &candidates), ("mutations", &adversaries)] {
        for (index, batch) in batches.iter().enumerate() {
            matrix.push(Case {
                name: format!("detached-{family}-{index:03}"),
                descriptor: descriptors[5].clone(),
                arguments: batch.arguments.clone(),
                expected: batch.expected.clone(),
            });
            observations.push(json!({"phase":"source-deleted","mode":family,"batch":index,"conditions":batch.count}));
        }
    }
    assert_eq!(matrix.len(), 46);
    assert_eq!(
        observations
            .iter()
            .map(|row| row["conditions"].as_u64().unwrap())
            .sum::<u64>(),
        53_006
    );
    for (observation, output) in observations
        .iter_mut()
        .zip(public.matrix("dependency-plan-verification", &matrix))
    {
        observation["observation"] = quoted(&output, "production-observation");
    }
    let arguments = public.input("single-example.json", &json!([example]).to_string());
    for &(mode, descriptor) in &single_modes {
        public.compare(
            &format!("single-{mode}"),
            descriptor,
            &arguments,
            &plan::oracle::expected(&example),
        );
    }
    let claimed = public.input(
        "single-claim.json",
        &json!([example, plan::oracle::expected(&example)["value"]]).to_string(),
    );
    public.compare(
        "single-verify",
        &descriptors[4],
        &claimed,
        &verification::cases::verified(),
    );
    let empty = public.input("empty-batch.json", "[[]]");
    for (mode, descriptor) in [
        ("flat", &descriptors[2]),
        ("chunked", &descriptors[3]),
        ("verify", &descriptors[5]),
    ] {
        public.compare(&format!("empty-{mode}"), descriptor, &empty, &json!([]));
    }
    for &(mode, descriptor) in &batch_modes[2..] {
        public.compare(&format!("empty-{mode}"), descriptor, &empty, &json!([]));
    }
    verification::refusals(&public, &descriptors[4]);
    outcome_suite.finish(&public);
    assert_eq!(executable_hash, digest(&public.root.join("lkjscript")));
    assert_eq!(verifier_hash, digest(&verifier_artifact));
    assert_eq!(producer_hash, digest(&producer_artifact));
    let maximum_input = plans
        .iter()
        .chain(&claims)
        .chain(&candidates)
        .chain(&adversaries)
        .map(|b| b.bytes)
        .max()
        .unwrap()
        .max(outcome_suite.maximum_bytes());
    let maximum_items = plans
        .iter()
        .chain(&claims)
        .chain(&candidates)
        .chain(&adversaries)
        .map(|batch| batch.items)
        .max()
        .unwrap()
        .max(outcome_suite.maximum_items());
    let conditions = proposals.len() * (batch_modes.len() + 1)
        + exhaustive.len()
        + mutations.len()
        + outcome_suite.count();
    public.input("witness-summary.json", &json!({
        "purpose":"Native result-checker evidence; not a source or distribution acceptance receipt",
        "status":"passed", "source_graphs":proposals.len(), "checked_plan_comparisons":proposals.len()*batch_modes.len(),
        "storage_combinations":plan::STORAGE_MODES,
        "ordinary_claim_comparisons":proposals.len(), "exhaustive_candidate_claims":exhaustive.len(),
        "valid_exhaustive_claims":512,"rejected_exhaustive_claims":11264,"targeted_mutations_and_recovery":mutations.len(),
        "batch_conditions":conditions,"batch_executions":observations.len(),"maximum_argument_bytes":maximum_input,"maximum_argument_items":maximum_items,
        "full_matrix_phase":"source-deleted", "attached_sample_executions":attached_observations.len(),
        "detached_matrix_boundary_executions":2,
        "attached_observations":attached_observations,
        "single_executions":single_modes.len()+3,"empty_batches":batch_modes.len()+2,
        "raw_and_resource_refusals":6,"subsequent_recoveries":6,
        "outcome_verification":outcome_suite.summary(),"standalone_before_producer_executions":3,
        "source_projects_and_transports_deleted":true,"binary_sha256":executable_hash,
        "execution":public.execution_evidence(),
        "verifier_artifact_sha256":verifier_hash,"producer_artifact_sha256":producer_hash,
        "verifier_transport_sha256":verifier_transport_hash,"observations":observations,
    }).to_string());
    Ok(())
}
