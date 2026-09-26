//! Independent layout-policy and vendored-authority witnesses.
use super::*;
use crate::platform::kernel::OwnerRecord;
use crate::platform::publication::GraphRepository;
use serde_json::{Value, json};

const ORIGINAL: &str = include_str!("../../../../docs/guides/examples/editor.deployment.json");

#[test]
fn starter_changes_only_artifact_and_data_placement() {
    let mut expected: Value = serde_json::from_str(ORIGINAL).unwrap();
    expected["artifact"] = json!("generated/application.lkja");
    expected["grants"][3]["adapter"]["root"] = json!("notes.lkjdata");
    let actual: Value = serde_json::from_slice(&descriptor().unwrap()).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(
        actual["configuration"]["origin"]["value"],
        "http://127.0.0.1:8080"
    );
    assert_eq!(
        actual["secrets"][0]["variable"],
        "LKJSCRIPT_EDITOR_AUTHORIZATION"
    );
    for quota in [
        "instruction_fuel",
        "maximum_allocated_bytes",
        "maximum_collection_items",
        "maximum_capability_calls",
    ] {
        assert!(actual["execution"][quota].is_null());
    }
}

#[test]
fn changed_template_linkage_is_rejected_not_silently_reinterpreted() {
    let original: Value = serde_json::from_str(ORIGINAL).unwrap();
    let mut variants = Vec::new();
    let mut changed = original.clone();
    changed["target"] = json!("another");
    variants.push(changed);
    let mut changed = original.clone();
    changed["listen"] = json!("0.0.0.0:8080");
    variants.push(changed);
    let mut changed = original.clone();
    changed["grants"][3]["adapter"]["root"] = json!("other");
    variants.push(changed);
    let mut changed = original.clone();
    changed["grants"][3]["adapter"]["namespace"] = json!("other");
    variants.push(changed);
    let mut changed = original.clone();
    changed["grants"].as_array_mut().unwrap().pop();
    variants.push(changed);
    let mut changed = original.clone();
    changed["grants"]
        .as_array_mut()
        .unwrap()
        .push(original["grants"][3].clone());
    let duplicate = from_source(&serde_json::to_vec(&changed).unwrap()).unwrap_err();
    assert_eq!(duplicate.code, "deployment_grant_duplicate");
    let mut changed = original.clone();
    changed["secrets"][0]["variable"] = json!("OTHER_SECRET_VARIABLE");
    variants.push(changed);
    for source in variants {
        let error = from_source(&serde_json::to_vec(&source).unwrap()).unwrap_err();
        assert_eq!(error.code, "new_editor_deployment");
    }
}

#[test]
fn durable_starter_has_local_editable_modules_and_no_initialized_data() {
    let temporary = tempfile::TempDir::new().unwrap();
    let destination = temporary.path().join("notes");
    let created = super::super::create_project(
        &destination,
        "notes",
        super::super::ProjectTemplate::WebEditor,
    )
    .unwrap();
    assert_eq!(created.dependencies, 1);
    assert_eq!(created.targets, 1);
    assert_eq!(created.tests, 105); // 19 UI + 48 forms + 38 editor.
    let snapshot = GraphRepository::open(&destination)
        .unwrap()
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    let mut modules: Vec<_> = snapshot
        .owners
        .values()
        .filter_map(|owner| {
            if let OwnerRecord::Module(module) = owner {
                Some(module.name.as_str())
            } else {
                None
            }
        })
        .collect();
    modules.sort_unstable();
    assert_eq!(
        modules,
        ["editor", "editor-tests", "forms", "ui", "ui-tests"]
    );
    let deployment = created.deployment.unwrap();
    assert_eq!(deployment.target, "editor");
    assert_eq!(
        deployment.required_data_root,
        Some(destination.join("notes.lkjdata"))
    );
    assert_eq!(
        deployment.required_secret_variable,
        Some("LKJSCRIPT_EDITOR_AUTHORIZATION")
    );
    assert_eq!(deployment.configured_listener, Some("127.0.0.1:8080"));
    assert!(!deployment.recommended_artifact_output.exists());
    assert!(!destination.join("notes.lkjdata").exists());
    assert!(!destination.join("data").exists());
    assert_eq!(std::fs::read_dir(temporary.path()).unwrap().count(), 1);
}
