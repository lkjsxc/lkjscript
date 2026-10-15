use super::*;

fn inventory() -> public_inventory::Inventory {
    public_inventory::Inventory {
        all: [
            "current_public_matrix",
            "future_semantic_case",
            public_inventory::OPT_IN_SCALE,
        ]
        .map(str::to_owned)
        .into_iter()
        .collect(),
        selected: BTreeSet::new(),
    }
}

fn complete() -> String {
    format!(
        "running 3 tests\ntest current_public_matrix ... ok\ntest future_semantic_case ... ok\ntest {} ... ignored, release-scale fixture; run explicitly\ntest result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s\n",
        public_inventory::OPT_IN_SCALE
    )
}

#[test]
fn original_full_public_block_binds_all_ordinary_cases_and_the_one_explicit_scale_exception() {
    let current = inventory();
    passed(&complete(), &current).unwrap();
    let unrelated = "running 3 tests\ntest foreign_one ... ok\ntest foreign_two ... ok\ntest foreign_three ... ok\ntest result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n";
    passed(&format!("{unrelated}{}{unrelated}", complete()), &current).unwrap();
    let noticed = complete().replace(
        "running 3 tests\n",
        "running 3 tests\ntest future_semantic_case has been running for over 60 seconds\n",
    );
    passed(&noticed, &current).unwrap();
    // The opt-in scale case may also be executed explicitly; it is never allowed
    // to justify ignoring any other test in the dynamically enumerated inventory.
    passed(
        &complete()
            .replace(
                &format!(
                    "test {} ... ignored, release-scale fixture; run explicitly",
                    public_inventory::OPT_IN_SCALE
                ),
                &format!("test {} ... ok", public_inventory::OPT_IN_SCALE),
            )
            .replace(
                "2 passed; 0 failed; 1 ignored",
                "3 passed; 0 failed; 0 ignored",
            ),
        &current,
    )
    .unwrap();
}

#[test]
fn original_source_cannot_hide_ignored_missing_failed_repeated_or_cross_binary_semantics() {
    let current = inventory();
    let original = complete();
    for changed in [
        original.replace("test current_public_matrix ... ok", "test current_public_matrix ... ignored"),
        original.replace("test future_semantic_case ... ok\n", ""),
        original.replace("test current_public_matrix ... ok", "test current_public_matrix ... FAILED"),
        original.replace("test current_public_matrix ... ok", "test current_public_matrix ... ok\ntest current_public_matrix ... ok"),
        original.replace("future_semantic_case ... ok", "impostor::future_semantic_case ... ok"),
        original.replace("running 3 tests", "running 2 tests"),
        original.replace("2 passed", "3 passed"),
        original.replace("0 filtered out", "1 filtered out"),
        original.replace("test result: ok.", "test result: FAILED."),
        original.clone() + &original,
        original.clone() + "running 1 test\ntest future_semantic_case ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n",
        original.replace("running 3 tests\n", "running 3 tests\ntest future_semantic_case has been running for over 60 seconds\ntest future_semantic_case has been running for over 60 seconds\n"),
    ] {
        assert!(passed(&changed, &current).is_err(), "accepted source outcome {changed}");
    }
    let mut newer = inventory();
    newer.all.insert("newly_added_semantic_matrix".to_owned());
    assert!(passed(&original, &newer).is_err());
}

#[test]
fn source_cargo_stderr_must_name_the_unique_matching_public_test_binary() {
    let repository = Path::new("/owned/repository");
    let selected = repository.join("target/debug/deps/public_cli-exact");
    let relative = "     Running tests/public_cli.rs (target/debug/deps/public_cli-exact)\n";
    harness(relative, repository, &selected).unwrap();
    harness(
        &format!("Running tests/public_cli.rs ({})\n", selected.display()),
        repository,
        &selected,
    )
    .unwrap();
    for changed in [
        String::new(),
        relative.repeat(2),
        relative.replace("public_cli-exact", "public_cli-foreign"),
        relative.replace("tests/public_cli.rs", "tests/other.rs"),
        relative.replace(')', ""),
    ] {
        assert!(harness(&changed, repository, &selected).is_err());
    }
}
