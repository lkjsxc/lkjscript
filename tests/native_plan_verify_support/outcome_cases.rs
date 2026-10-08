//! Test-only exact outcome oracle and adversarial negative claims.
use super::cases::{empty_plan, rejected, verified};
use serde_json::{Value, json};
use std::collections::BTreeSet;

type Sample = (Value, Value);
fn invalid(code: &str, owner: i64, target: i64) -> Value {
    json!({"case":"invalid","value":{"code":code,"owner":owner,"target":target}})
}
fn expected(proposal: &Value, claim: &Value) -> Value {
    let actual = crate::plan::oracle::expected(proposal);
    if actual["case"] != claim["case"] {
        return rejected("source-outcome");
    }
    if actual == *claim {
        return verified();
    }
    match actual["case"].as_str().unwrap() {
        "invalid" => rejected("source-diagnostic"),
        "capacity" => rejected("source-capacity"),
        _ => panic!("noncanonical successful plans need the independent predicate oracle"),
    }
}
fn sample(proposal: &Value, claim: Value) -> Sample {
    let report = expected(proposal, &claim);
    (json!({"proposal":proposal,"claim":claim}), report)
}
fn small_invalid() -> Vec<Value> {
    let mut output = Vec::new();
    for a in [i64::MIN, -1, i64::MAX] {
        for b in [i64::MIN, -1, i64::MAX] {
            if a == b {
                continue;
            }
            output.push(json!({"roots":[17],"nodes":[
                {"id":a,"successors":[18]}, {"id":b,"successors":[]},
                {"id":a,"successors":[]}, {"id":b,"successors":[]}]}));
            output.push(json!({"roots":[b,a],"nodes":[]}));
            output.push(json!({"roots":[a],"nodes":[
                {"id":a,"successors":[]}, {"id":b,"successors":[17,18]}]}));
        }
    }
    output
}
fn capacity_inputs() -> Vec<Value> {
    let node = json!({"id":1,"successors":[]});
    let mut all = vec![node.clone(); 4097];
    all[0]["successors"] = json!(vec![1; 16385]);
    vec![
        json!({"roots":[],"nodes":vec![node.clone();4097]}),
        json!({"roots":vec![9;4097],"nodes":[node.clone(),node.clone()]}),
        json!({"roots":[9],"nodes":[{"id":1,"successors":vec![1;16385]},node.clone()]}),
        json!({"roots":vec![9;4097],"nodes":all}),
        json!({"roots":vec![9;4097],"nodes":[{"id":1,"successors":vec![1;16385]},node]}),
    ]
}
fn negative_claims() -> Vec<Value> {
    let mut claims = Vec::new();
    for code in ["duplicate-node", "missing-root", "missing-successor"] {
        for owner in [i64::MIN, -1, i64::MAX] {
            for target in [17, 18, i64::MIN, -1, i64::MAX] {
                claims.push(invalid(code, owner, target));
            }
        }
    }
    claims.push(json!({"case":"valid","value":empty_plan()}));
    for dimension in ["nodes", "roots", "edges"] {
        claims.push(json!({"case":"capacity","value":dimension}));
    }
    claims
}

pub fn suite() -> Vec<Sample> {
    let proposals = crate::plan::cases::suite();
    let mut output: Vec<_> = proposals
        .iter()
        .map(|proposal| sample(proposal, crate::plan::oracle::expected(proposal)))
        .collect();
    // Every directed three-vertex graph is valid, including cycles. None may
    // receive an invented invalidity or graph-capacity diagnosis.
    for proposal in proposals.iter().take(512) {
        for code in ["duplicate-node", "missing-root", "missing-successor"] {
            output.push(sample(proposal, invalid(code, -1, i64::MIN)));
        }
        for dimension in ["nodes", "roots", "edges"] {
            output.push(sample(
                proposal,
                json!({"case":"capacity","value":dimension}),
            ));
        }
    }
    let claims = negative_claims();
    for proposal in small_invalid() {
        let actual = crate::plan::oracle::expected(&proposal);
        let mut unique = BTreeSet::new();
        for claim in claims.iter().cloned().chain([actual.clone()]).chain(
            [
                "",
                "missing-root ",
                "missing-root\u{0000}",
                "MISSING-ROOT",
                "不正",
            ]
            .map(|code| {
                let mut claim = actual.clone();
                claim["value"]["code"] = json!(code);
                claim
            }),
        ) {
            if unique.insert(claim.to_string()) {
                output.push(sample(&proposal, claim));
            }
        }
    }
    for proposal in capacity_inputs() {
        for dimension in ["nodes", "roots", "edges", "nodes\u{0000}", "nodes ", ""] {
            output.push(sample(
                &proposal,
                json!({"case":"capacity","value":dimension}),
            ));
        }
        output.push(sample(
            &proposal,
            json!({"case":"valid","value":empty_plan()}),
        ));
        output.push(sample(&proposal, invalid("duplicate-node", 1, 1)));
    }
    // Keep the established independent predicate-specific rejection reasons for
    // incorrect successful plans. Invalid input now rejects the *claim* instead.
    for (_, input, report) in super::mutations::suite() {
        let report = if matches!(report["case"].as_str(), Some("invalid" | "capacity")) {
            rejected("source-outcome")
        } else {
            report
        };
        output.push((
            json!({"proposal":input["proposal"],
            "claim":{"case":"valid","value":input["plan"]}}),
            report,
        ));
    }
    output
}

#[test]
fn negative_outcome_oracle_preserves_precedence_and_exact_payload() {
    let sources = small_invalid();
    assert_eq!(sources.len(), 18);
    for proposal in sources {
        let actual = crate::plan::oracle::expected(&proposal);
        assert_eq!(expected(&proposal, &actual), verified());
        for field in ["code", "owner", "target"] {
            let mut wrong = actual.clone();
            wrong["value"][field] = if field == "code" {
                json!("wrong")
            } else {
                json!(42)
            };
            assert_eq!(expected(&proposal, &wrong), rejected("source-diagnostic"));
        }
    }
    for (proposal, dimension) in capacity_inputs()
        .iter()
        .zip(["nodes", "roots", "edges", "nodes", "roots"])
    {
        let actual = json!({"case":"capacity","value":dimension});
        assert_eq!(crate::plan::oracle::expected(proposal), actual);
        assert_eq!(expected(proposal, &actual), verified());
    }
}

#[test]
fn outcome_family_preserves_true_negatives_and_rejects_false_claims() {
    let samples = suite();
    let true_negatives = samples
        .iter()
        .filter(|(input, report)| input["claim"]["case"] != "valid" && report["case"] == "verified")
        .count();
    assert_eq!(true_negatives, 29); // Six inherited, eighteen exact small, five capacity.
    assert_eq!(samples.len(), 8760);
    for (input, report) in &samples {
        if report["case"] == "verified" {
            assert_eq!(
                input["claim"],
                crate::plan::oracle::expected(&input["proposal"])
            );
        }
    }
}
