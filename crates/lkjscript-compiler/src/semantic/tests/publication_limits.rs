use lkjscript_core::{BudgetLedger, ResourceCategory, ResourceProfile};

use super::*;
use crate::semantic::schema::{ResponseResult, SnapshotResult};

const SOURCE: &str = concat!(
    "def/\nname/\nf\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\n",
    "params/\n/params\nunit\n/fn\n/def\n",
    "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nf/\n/f\n/main\n",
);

struct Case {
    root: PathBuf,
    publish: Vec<u8>,
    staged_bytes: u64,
    output_bytes: u64,
}

fn setup(label: &str) -> Case {
    let directory = case_dir(label);
    let root = directory.join("main.lkjscript");
    std::fs::write(&root, SOURCE).expect("write source");
    let snapshot = response(
        &crate::semantic::execute(&request(&root, "{\"kind\":\"snapshot\"}")).expect("snapshot"),
    );
    let revision = snapshot.revision.expect("revision");
    let ResponseResult::Snapshot { snapshot } = snapshot.result else {
        panic!("expected snapshot");
    };
    let operation = transaction(&revision, &snapshot, "preview");
    let mut ledger = BudgetLedger::default();
    crate::semantic::execute_with_ledger(&request(&root, &operation), &mut ledger)
        .expect("preview transaction");
    assert_eq!(
        std::fs::read_to_string(&root).expect("preview source"),
        SOURCE
    );
    let publish = request(&root, &operation.replace("\"preview\"", "\"publish\""));
    let mut output_bytes = ResourceProfile::default()
        .ceilings()
        .limit(ResourceCategory::ProtocolResponseBytes)
        - 1;
    for _ in 0..4 {
        let profile = ResourceProfile::default()
            .lowered(ResourceCategory::ProtocolResponseBytes, output_bytes)
            .expect("lower measured output profile");
        let typed = serde_json::from_slice(&publish).expect("decode typed publish request");
        let mut output_ledger = BudgetLedger::new(profile);
        let outcome = crate::semantic::engine::execute_request_with_ledger(
            typed,
            publish.len(),
            &mut output_ledger,
        )
        .expect("prepare unpublished response");
        let measured = u64::try_from(outcome.prepared.bytes).expect("output bytes");
        drop(outcome);
        if measured == output_bytes {
            break;
        }
        output_bytes = measured;
    }
    Case {
        root,
        publish,
        staged_bytes: ledger.used(ResourceCategory::StagedPublicationBytes),
        output_bytes,
    }
}

fn transaction(revision: &str, snapshot: &SnapshotResult, mode: &str) -> String {
    let declaration = snapshot
        .declarations
        .iter()
        .find(|item| item.name == "f")
        .expect("declaration");
    let preconditions = snapshot
        .source_units
        .iter()
        .map(|unit| {
            serde_json::json!({
                "path": unit.path,
                "bytes": unit.bytes,
                "sha256": unit.sha256,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "kind": "apply-transaction",
        "mode": mode,
        "base_revision": revision,
        "file_preconditions": preconditions,
        "operations": [{
            "kind": "rename-declaration",
            "declaration_key": declaration.key,
            "entity_fingerprint": declaration.fingerprint,
            "new_name": "g",
        }],
    })
    .to_string()
}

fn execute(case: &Case, category: ResourceCategory, limit: u64) -> Result<Vec<u8>, String> {
    let profile = ResourceProfile::default()
        .lowered(category, limit)
        .expect("lower profile");
    let mut ledger = BudgetLedger::new(profile);
    crate::semantic::execute_with_ledger(&case.publish, &mut ledger)
        .map(crate::semantic::SemanticExecution::into_response)
        .map_err(|error| error.to_string())
}

#[test]
fn transaction_staging_exact_succeeds_and_plus_one_never_publishes() {
    let exact = setup("staging-exact");
    execute(
        &exact,
        ResourceCategory::StagedPublicationBytes,
        exact.staged_bytes,
    )
    .expect("exact staging budget");
    assert!(std::fs::read_to_string(&exact.root)
        .expect("published")
        .contains("name/\ng\n/name"));

    let rejected = setup("staging-plus-one");
    execute(
        &rejected,
        ResourceCategory::StagedPublicationBytes,
        rejected.staged_bytes - 1,
    )
    .expect("bounded semantic error response");
    assert_eq!(
        std::fs::read_to_string(rejected.root).expect("rejected source"),
        SOURCE
    );
}

#[test]
fn transaction_output_exact_succeeds_and_plus_one_never_publishes() {
    let exact = setup("output-exact");
    execute(
        &exact,
        ResourceCategory::ProtocolResponseBytes,
        exact.output_bytes,
    )
    .expect("exact output budget");
    assert!(std::fs::read_to_string(&exact.root)
        .expect("published")
        .contains("name/\ng\n/name"));

    let rejected = setup("output-plus-one");
    assert!(execute(
        &rejected,
        ResourceCategory::ProtocolResponseBytes,
        rejected.output_bytes - 1,
    )
    .is_err());
    assert_eq!(
        std::fs::read_to_string(rejected.root).expect("rejected source"),
        SOURCE
    );
}
