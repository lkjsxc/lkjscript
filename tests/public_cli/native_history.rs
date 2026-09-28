//! Copied public executable, native edits and exact recorded history.
use super::*;

fn entries(records: &[CompactRecord]) -> Vec<&CompactRecord> {
    records
        .iter()
        .filter(|record| record.operation == "history.entry")
        .collect()
}

fn populate(public: &Native) -> String {
    let input = public.input(
        "create.lkjc",
        &format!("request base={}\n{COMPLETE}", public.revision()),
    );
    public.apply(&input, &public.plan(&input, true), true);
    compact_field(
        compact_record(
            &public.cli(&["query", "find", "module", "app"], true),
            "owner",
        ),
        "id",
    )
    .to_owned()
}

#[test]
fn history_native_edit_records_intent_and_never_turns_selected_tests_into_executed_tests() {
    let public = Native::new();
    let initial = public.revision();
    let module = populate(&public);
    let before_edit = public.revision();
    let intent = "Readable name: 日本語 \"quotes\"\nnot-a-record\t\\end";
    let mut args = vec![
        "change",
        "plan",
        "rename.owner",
        "--base",
        &before_edit,
        "--owner",
        &module,
        "--name",
        "application",
        "--intent",
        intent,
        "--idempotency",
        "history-rename",
    ];
    let plan = public.cli(&args, true);
    let token = compact_field(compact_record(&plan, "plan"), "token");
    args[1] = "apply";
    args.extend(["--plan", token]);
    public.cli(&args, true);
    let after_edit = public.revision();
    public.cli(&args, true); // Exact accepted retry must not publish a second revision.
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let history = public.cli(&["inspect", "history"], true);
    let summary = compact_record(&history, "history");
    assert_eq!(compact_field(summary, "current-validation"), "not-run");
    assert_eq!(compact_field(summary, "truncated"), "false");
    assert_eq!(compact_field(summary, "observed"), after_edit);
    let rows = entries(&history);
    assert_eq!(rows.len(), 3);
    for (row, expected) in rows.iter().zip([&after_edit, &before_edit, &initial]) {
        assert_eq!(compact_field(row, "revision"), expected);
    }
    assert_eq!(compact_field(rows[0], "intent"), intent);
    assert_eq!(compact_field(rows[0], "intent-present"), "true");
    assert_eq!(compact_field(rows[0], "owners-updated"), "1");
    assert_eq!(compact_field(rows[0], "owners-created"), "0");
    assert_eq!(compact_field(rows[0], "owners-deleted"), "0");
    assert_eq!(compact_field(rows[0], "tests-executed"), "0");
    assert_eq!(compact_field(rows[0], "tests-passed"), "0");
    assert!(
        compact_field(rows[1], "tests-selected")
            .parse::<u64>()
            .unwrap()
            > 0
    );
    assert_eq!(compact_field(rows[1], "tests-executed"), "0");
    assert_eq!(compact_field(rows[1], "tests-passed"), "0");
    assert_eq!(compact_field(rows[2], "status"), "project-created");
    assert_eq!(compact_field(rows[2], "parent"), "none");
    for pair in rows.windows(2) {
        assert_eq!(
            compact_field(pair[0], "parent"),
            compact_field(pair[1], "revision")
        );
        assert_eq!(
            compact_field(pair[0], "parent-record"),
            compact_field(pair[1], "record")
        );
    }
    let short = public.cli(&["inspect", "history", "--limit", "1"], true);
    let summary = compact_record(&short, "history");
    assert_eq!(compact_field(summary, "truncated"), "true");
    assert_eq!(compact_field(summary, "next-revision"), before_edit);
    assert_eq!(
        compact_field(summary, "next-record"),
        compact_field(rows[1], "record")
    );
    assert_eq!(compact_field(summary, "objects-read"), "2");
    assert_eq!(entries(&short).len(), 1);
    // The original accepted base is stale for a new request, with no extra history.
    public.cli(
        &[
            "change",
            "plan",
            "rename.owner",
            "--base",
            &before_edit,
            "--owner",
            &module,
            "--name",
            "stale",
        ],
        false,
    );
    assert_eq!(public.cli(&["inspect", "history"], true), history);
    assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    public.cli(&["check"], true);
    // A later successful check must not rewrite earlier recorded acceptance evidence.
    assert_eq!(public.cli(&["inspect", "history"], true), history);
}

#[test]
fn history_rejects_invalid_or_foreign_grammar_and_advertises_its_scope() {
    let public = Native::new();
    let before = public.cli(&["inspect", "history"], true);
    for options in [
        vec!["--limit", "0"],
        vec!["--limit", "101"],
        vec!["--limit", "-1"],
        vec!["--limit", "+1"],
        vec!["--limit", "1.0"],
        vec!["--limit", "184467440737095516160"],
        vec!["--limit", "1", "--limit", "2"],
        vec!["--limit"],
        vec!["--bytes", "64"],
        vec!["--continuation", "none"],
        vec!["--at", "HEAD"],
        vec!["foreign"],
    ] {
        let mut args = vec!["inspect", "history"];
        args.extend(options);
        let rejected = public.cli(&args, false);
        assert!(entries(&rejected).is_empty());
    }
    public.cli(&["history"], false); // Removed predecessor spelling stays removed.
    let advertised = public.cli(&["capabilities", "--section", "inspection"], true);
    let contract = compact_record(&advertised, "inspection.history");
    assert_eq!(compact_field(contract, "default-items"), "20");
    assert_eq!(compact_field(contract, "maximum-items"), "100");
    assert_eq!(compact_field(contract, "mutation"), "none");
    assert_eq!(compact_field(contract, "continuation"), "none");
    assert_eq!(compact_field(contract, "maximum-catalog-lookups"), "200");
    assert_eq!(compact_field(contract, "maximum-store-bytes"), "8388608");
    // Discovery must describe the fields actually returned, not just the command spelling.
    let fields: std::collections::BTreeSet<_> = advertised
        .iter()
        .filter(|record| record.operation == "inspection.history-response-field")
        .map(|record| {
            (
                compact_field(record, "record"),
                compact_field(record, "name"),
            )
        })
        .collect();
    for record in before
        .iter()
        .filter(|record| record.operation == "history" || record.operation == "history.entry")
    {
        for field in &record.fields {
            assert!(
                fields.contains(&(record.operation.as_str(), field.name.as_str())),
                "missing discovered field {}",
                field.name
            );
        }
    }
    assert_eq!(public.cli(&["inspect", "history"], true), before);
}

#[test]
fn history_default_truncates_twenty_and_explicit_larger_window_completes() {
    let public = Native::new();
    let module = populate(&public);
    let mut revisions = vec![public.revision()];
    for index in 0..20 {
        let name = format!("app-{index}");
        let input = public.input(
            "rename.lkjc",
            &format!(
                "request base={}\nrename.owner owner={} name={name}\n",
                public.revision(),
                module
            ),
        );
        public.apply(&input, &public.plan(&input, true), true);
        revisions.push(public.revision());
    }
    let recent = public.cli(&["inspect", "history"], true);
    assert_eq!(entries(&recent).len(), 20);
    assert_eq!(
        compact_field(compact_record(&recent, "history"), "truncated"),
        "true"
    );
    let all = public.cli(&["inspect", "history", "--limit", "100"], true);
    let rows = entries(&all);
    assert_eq!(rows.len(), 22);
    assert_eq!(
        compact_field(compact_record(&all, "history"), "truncated"),
        "false"
    );
    for (row, revision) in rows.iter().zip(revisions.iter().rev()) {
        assert_eq!(compact_field(row, "revision"), revision);
    }
}
