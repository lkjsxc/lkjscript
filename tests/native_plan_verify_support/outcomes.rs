use super::{Batch, batches, cases, outcome_cases, outcome_refusals};
use crate::components::matrix::Case;
use crate::components::{Native, quoted};
use serde_json::{Value, json};
use std::path::PathBuf;

pub const DESCRIPTORS: [(&str, &str); 2] = [
    (
        "outcome",
        include_str!("../../examples/dependency-plan/verification/outcomes/single.deployment.json"),
    ),
    (
        "outcome-batch",
        include_str!("../../examples/dependency-plan/verification/outcomes/batch.deployment.json"),
    ),
];
pub struct Suite {
    descriptors: [PathBuf; 2],
    batches: Vec<Batch>,
    count: usize,
    verified: usize,
    true_negatives: usize,
    attached_sample: (Value, Value),
}
impl Suite {
    pub fn new(public: &Native) -> Result<Self, Box<dyn std::error::Error>> {
        let samples = outcome_cases::suite();
        let count = samples.len();
        let verified = samples
            .iter()
            .filter(|(_, report)| report["case"] == "verified")
            .count();
        let true_negatives = samples
            .iter()
            .filter(|(input, report)| {
                input["claim"]["case"] != "valid" && report["case"] == "verified"
            })
            .count();
        Ok(Self {
            attached_sample: samples[0].clone(),
            descriptors: DESCRIPTORS
                .map(|(name, text)| public.input(&format!("{name}.deployment.json"), text)),
            batches: batches(public, "outcome", &samples)?,
            count,
            verified,
            true_negatives,
        })
    }
    pub fn attached_sample(&self, public: &Native) -> Value {
        let arguments = public.input(
            "attached-outcome-sample.json",
            &json!([[self.attached_sample.0]]).to_string(),
        );
        let output = public.compare(
            "attached-outcome-sample",
            &self.descriptors[1],
            &arguments,
            &json!([self.attached_sample.1]),
        );
        json!({"phase":"attached-sample","mode":"outcome","conditions":1,
            "observation":quoted(&output,"production-observation")})
    }
    pub fn matrix_cases(&self) -> Vec<(Case, Value)> {
        self.batches
            .iter()
            .enumerate()
            .map(|(index, batch)| {
                (Case { name:format!("source-deleted-outcome-{index:03}"),
                    descriptor:self.descriptors[1].clone(), arguments:batch.arguments.clone(),
                    expected:batch.expected.clone() },
                json!({"phase":"source-deleted","mode":"outcome","batch":index,"conditions":batch.count}))
            })
            .collect()
    }
    pub fn finish(&self, public: &Native) {
        let empty = public.input("empty-outcome.json", "[[]]");
        public.compare("empty-outcome", &self.descriptors[1], &empty, &json!([]));
        outcome_refusals::exercise(public, &self.descriptors[0]);
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn maximum_items(&self) -> usize {
        self.batches.iter().map(|batch| batch.items).max().unwrap()
    }
    pub fn maximum_bytes(&self) -> usize {
        self.batches.iter().map(|batch| batch.bytes).max().unwrap()
    }
    pub fn summary(&self) -> Value {
        json!({"claims_per_phase":self.count,"phases":1,"full_matrix_phase":"source-deleted",
            "attached_sample_executions":1,"verified_per_phase":self.verified,
            "rejected_per_phase":self.count-self.verified,"true_negatives_per_phase":self.true_negatives,
            "maximum_argument_items":self.maximum_items(),"batches_per_phase":self.batches.len(),"single_executions":2,"empty_batches":1,
            "raw_and_resource_refusals":4,"subsequent_recoveries":4})
    }
}

pub fn before_producer(public: &Native) {
    assert!(!public.root.join("application").exists());
    assert!(!public.root.join("verified-dependency-plan.lkja").exists());
    let descriptor = public.input("outcome-first.deployment.json", DESCRIPTORS[0].1);
    let proposal = json!({"roots":[90,80],"nodes":[]});
    let correct = json!({"case":"invalid","value":{"code":"missing-root","owner":-1,"target":90}});
    let arguments = public.input(
        "outcome-first-negative.json",
        &json!([proposal, correct]).to_string(),
    );
    public.compare(
        "outcome-before-producer-negative",
        &descriptor,
        &arguments,
        &cases::verified(),
    );
    let arguments = public.input(
        "outcome-first-false.json",
        &json!([
            {"roots":[],"nodes":[]}, {"case":"capacity","value":"nodes"}
        ])
        .to_string(),
    );
    public.compare(
        "outcome-before-producer-false",
        &descriptor,
        &arguments,
        &cases::rejected("source-outcome"),
    );
}
