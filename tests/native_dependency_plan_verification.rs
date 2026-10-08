//! A separately built native verifier checks untrusted results and is then consumed
//! through an exact exported package by both native planning implementations.
#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "native_components_support/mod.rs"]
mod components;
#[allow(dead_code)]
#[path = "native_plan_support/mod.rs"]
mod plan;
#[path = "native_plan_verify_support/mod.rs"]
mod verification;
use components::{Native, digest, quoted};
use serde_json::json;
use std::fs;

#[test]
fn native_checker_rejects_false_plans_and_guards_an_independent_producer() {
    let public = Native::new();
    let executable_hash = digest(&public.root.join("lkjscript"));
    verification::author(&public);
    let verifier_artifact = public.root.join("dependency-plan-verifier.lkja");
    let producer_artifact = public.root.join("verified-dependency-plan.lkja");
    let verifier_hash = digest(&verifier_artifact);
    let producer_hash = digest(&producer_artifact);
    let verifier_transport_hash = digest(&public.root.join("verifier.lkjp"));
    let descriptors = verification::DESCRIPTORS
        .map(|(name, source)| public.input(&format!("{name}.deployment.json"), source));
    let proposals = plan::cases::suite();
    let expected: Vec<_> = proposals
        .iter()
        .map(|p| (p.clone(), plan::oracle::expected(p)))
        .collect();
    let normal = verification::cases::normal(&proposals);
    let plans = verification::batches(&public, "plans", &expected);
    let claims = verification::batches(&public, "claims", &normal);
    let exhaustive = verification::cases::exhaustive();
    let candidates = verification::batches(&public, "candidates", &exhaustive);
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
    let adversaries = verification::batches(&public, "mutations", &mutation_samples);
    let mut observations = Vec::new();
    for phase in ["attached", "source-deleted"] {
        if phase == "source-deleted" {
            fs::remove_dir_all(public.root.join("verifier")).unwrap();
            fs::remove_file(public.root.join("verifier.lkjp")).unwrap();
            public.detach();
        }
        for (mode, descriptor) in ["flat", "chunked"].into_iter().zip(&descriptors[2..4]) {
            for (index, batch) in plans.iter().enumerate() {
                let output = public.compare(
                    &format!("{phase}-{mode}-{index:03}"),
                    descriptor,
                    &batch.arguments,
                    &batch.expected,
                );
                observations.push(
                    json!({"phase":phase,"mode":mode,"batch":index,"conditions":batch.count,
                    "observation":quoted(&output,"production-observation")}),
                );
            }
        }
        for (index, batch) in claims.iter().enumerate() {
            let output = public.compare(
                &format!("{phase}-verify-{index:03}"),
                &descriptors[5],
                &batch.arguments,
                &batch.expected,
            );
            observations.push(
                json!({"phase":phase,"mode":"verify","batch":index,"conditions":batch.count,
                "observation":quoted(&output,"production-observation")}),
            );
        }
    }
    for (family, batches) in [("candidates", &candidates), ("mutations", &adversaries)] {
        for (index, batch) in batches.iter().enumerate() {
            let output = public.compare(
                &format!("detached-{family}-{index:03}"),
                &descriptors[5],
                &batch.arguments,
                &batch.expected,
            );
            observations.push(json!({"phase":"source-deleted","mode":family,"batch":index,"conditions":batch.count,
                "observation":quoted(&output,"production-observation")}));
        }
    }
    let example = components::cases::example();
    let arguments = public.input("single-example.json", &json!([example]).to_string());
    for (mode, descriptor) in ["flat", "chunked"].into_iter().zip(&descriptors[..2]) {
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
    verification::refusals(&public, &descriptors[4]);
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
        .unwrap();
    let conditions = proposals.len() * 6 + exhaustive.len() + mutations.len();
    public.input("witness-summary.json", &json!({
        "purpose":"Native result-checker evidence; not a source or distribution acceptance receipt",
        "status":"passed", "source_graphs":proposals.len(), "checked_plan_comparisons":proposals.len()*4,
        "ordinary_claim_comparisons":proposals.len()*2, "exhaustive_candidate_claims":exhaustive.len(),
        "valid_exhaustive_claims":512,"rejected_exhaustive_claims":11264,"targeted_mutations_and_recovery":mutations.len(),
        "batch_conditions":conditions,"batch_executions":observations.len(),"maximum_argument_bytes":maximum_input,
        "single_executions":3,"empty_batches":3,"raw_and_resource_refusals":2,"subsequent_recoveries":2,
        "source_projects_and_transports_deleted":true,"binary_sha256":executable_hash,
        "verifier_artifact_sha256":verifier_hash,"producer_artifact_sha256":producer_hash,
        "verifier_transport_sha256":verifier_transport_hash,"observations":observations,
    }).to_string());
}
