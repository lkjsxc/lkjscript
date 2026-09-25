//! Native recipe boundary and private-stage failure tests.

use super::super::{ProjectRecipe, ProjectTemplate, lower_recipe};
use super::*;
use crate::platform::kernel::{KernelSnapshot, Name, OwnerRecord, PackageId};
use crate::platform::semantic_id::RepositoryId;
use std::path::Path;

const FIRST: Input = Input {
    path: "first.lkjc",
    base: "BASE",
    source: "request base=BASE\ndeclarations.begin\n(units (module create first\n(function create answer (visibility public) (returns Text) (effect pure)\n(body (text \"BASE\")))))\ndeclarations.end\n",
};
const SECOND: Input = Input {
    path: "second.lkjc",
    base: "BASE",
    source: "request base=BASE\ndeclarations.begin\n(units (module create second\n(function create answer (visibility public) (returns Text) (effect pure)\n(body (call first::answer)))))\ndeclarations.end\n",
};

fn lower(parent: &Path, inputs: &'static [Input]) -> Result<KernelSnapshot, Diagnostic> {
    lower_recipe(
        parent,
        RepositoryId::generate().unwrap(),
        PackageId::generate().unwrap(),
        Name::new("native-test").unwrap(),
        &ProjectRecipe {
            changes: Vec::new(),
            native_inputs: inputs,
            transports: Vec::new(),
            template: ProjectTemplate::Minimal,
            auxiliary: None,
        },
    )
}

#[test]
fn binds_only_the_exact_first_line_without_rewriting_native_text() {
    let revision: RevisionId =
        "rev_1111111111111111111111111111111111111111111111111111111111111111"
            .parse()
            .unwrap();
    let bound = FIRST.bind(revision).unwrap();
    assert!(bound.starts_with(&format!("request base={revision}\n")));
    assert_eq!(
        bound.split_once('\n').unwrap().1,
        FIRST.source.split_once('\n').unwrap().1
    );
    assert!(bound.contains("(text \"BASE\")"));
    for source in [
        "",
        "request base=OTHER\n",
        " request base=BASE\n",
        "request base=BASE",
        "request base=BASE\r\n",
        "request base=BASE idempotency=other\n",
    ] {
        let error = Input { source, ..FIRST }.bind(revision).unwrap_err();
        assert_eq!(error.code, "new_native_recipe_header");
    }
}

#[test]
fn successive_native_units_resolve_the_preceding_accepted_private_revision() {
    let temporary = tempfile::TempDir::new().unwrap();
    let snapshot = lower(temporary.path(), &[FIRST, SECOND]).unwrap();
    let modules: Vec<_> = snapshot
        .owners
        .values()
        .filter_map(|owner| match owner {
            OwnerRecord::Module(record) => Some(record.name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(modules.len(), 2);
    assert!(modules.contains(&"first") && modules.contains(&"second"));
    assert!(snapshot.dependencies.is_empty());
    assert_eq!(std::fs::read_dir(temporary.path()).unwrap().count(), 0);
}

#[test]
fn later_native_failures_remove_the_entire_owned_lowering_stage() {
    const MALFORMED: Input = Input {
        path: "malformed.lkjc",
        base: "BASE",
        source: "request base=BASE\ndeclarations.begin\n(units (module create broken\ndeclarations.end\n",
    };
    const WRONG_TYPE: Input = Input {
        path: "wrong-type.lkjc",
        base: "BASE",
        source: "request base=BASE\ndeclarations.begin\n(units (module create broken\n(function create answer (visibility private) (returns I64) (effect pure)\n(body (text \"not an integer\")))))\ndeclarations.end\n",
    };
    const WRONG_HEADER: Input = Input {
        path: "wrong-header.lkjc",
        source: "request base=OTHER\n",
        base: "BASE",
    };
    for (name, expected_code, inputs) in [
        (
            "syntax",
            "change_block_parenthesis",
            &[FIRST, MALFORMED][..],
        ),
        ("type", "kernel_type_root", &[FIRST, WRONG_TYPE][..]),
        ("duplicate", "change_derived_collision", &[FIRST, FIRST][..]),
        (
            "header",
            "new_native_recipe_header",
            &[FIRST, WRONG_HEADER][..],
        ),
    ] {
        let temporary = tempfile::TempDir::new().unwrap();
        let unrelated = temporary.path().join("unrelated");
        std::fs::write(&unrelated, b"not recipe-owned").unwrap();
        let error = lower(temporary.path(), inputs)
            .err()
            .expect("invalid later input must reject");
        assert_eq!(error.code, expected_code, "{name}");
        assert_eq!(std::fs::read(&unrelated).unwrap(), b"not recipe-owned");
        assert_eq!(
            std::fs::read_dir(temporary.path()).unwrap().count(),
            1,
            "{name}"
        );
    }
}

#[test]
fn web_recipe_vendors_native_modules_without_hidden_application_dependencies() {
    let temporary = tempfile::TempDir::new().unwrap();
    let destination = temporary.path().join("web");
    let created = super::super::create_project(&destination, "web", ProjectTemplate::Web).unwrap();
    assert_eq!(created.dependencies, 1);
    assert_eq!(created.targets, 1);
    assert_eq!(created.tests, 32); // 19 shared UI declarations + 13 app declarations.
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
        .filter_map(|owner| match owner {
            OwnerRecord::Module(record) => Some(record.name.as_str()),
            _ => None,
        })
        .collect();
    modules.sort_unstable();
    assert_eq!(modules, ["ui", "ui-tests", "web"]);
    assert_eq!(std::fs::read_dir(temporary.path()).unwrap().count(), 1);
    let descriptor = created.deployment.unwrap();
    assert_eq!(descriptor.target, "serve");
    assert_eq!(descriptor.runner, "http");
    assert_eq!(descriptor.configured_listener, Some("127.0.0.1:0"));
    assert!(descriptor.descriptor.is_file());
    assert!(!descriptor.recommended_artifact_output.exists());
}
