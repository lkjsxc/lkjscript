//! Explicit concurrent refresh through an isolated public executable and literal native inputs.
use super::native_byte_buffer::{author, dependency, export, stage};
use super::*;

const FIXTURE: &str = r#"declarations.begin
(units
  (module create refresh-suite
    (function create left (as $left) (visibility private)
      (returns I64) (effect pure) (body (i64 10)))
    (function create right (as $right) (visibility private)
      (returns I64) (effect pure) (body (i64 20)))
    (function create dormant (as $dormant) (visibility private)
      (returns I64) (effect pure) (body (i64 7)))
    (function create ignore (as $ignore) (visibility private)
      (parameter create value (as $parameter) (type I64))
      (returns I64) (effect pure) (body (i64 1)))
    (component create console (visibility private)
      (port create left (type (function () I64)) (function left))
      (port create right (type (function () I64)) (function right))))
  (target create refresh-left (component refresh-suite::console) (runner command)
    (port refresh-suite::console::left))
  (target create refresh-right (component refresh-suite::console) (runner command)
    (port refresh-suite::console::right)))
declarations.end
"#;

fn fixture() -> Native {
    let public = Native::template("command");
    author(&public, FIXTURE);
    public
}

fn find(public: &Native, class: &str, name: &str, parent: Option<&str>) -> String {
    let mut args = vec!["query", "find", class, name];
    if let Some(parent) = parent {
        args.extend(["--parent", parent]);
    }
    compact_field(compact_record(&public.cli(&args, true), "owner"), "id").to_owned()
}

fn draft(public: &Native, name: &str) -> String {
    let file = public.root.path().join(format!("{name}-draft.lkjc"));
    let declaration = format!("refresh-suite::{name}");
    public.cli(
        &[
            "change",
            "draft",
            "--declaration",
            &declaration,
            "--output",
            path(&file),
        ],
        true,
    );
    std::fs::read_to_string(file).unwrap()
}

fn edited(public: &Native, name: &str, before: i64, after: i64) -> PathBuf {
    let source = draft(public, name);
    let old = format!("(i64 {before})");
    let new = format!("(i64 {after})");
    assert!(source.contains(&old));
    public.input(
        &format!("{name}-{after}.lkjc"),
        &source.replacen(&old, &new, 1),
    )
}

fn plan_file(public: &Native, input: &Path, name: &str) -> (Vec<CompactRecord>, PathBuf) {
    let file = public.root.path().join(format!("{name}.lkjplan"));
    let records = public.cli(
        &[
            "change",
            "plan",
            "--input-file",
            path(input),
            "--output",
            path(&file),
        ],
        true,
    );
    let decoded = decode_logical_change_plan(BufReader::new(File::open(&file).unwrap())).unwrap();
    assert_eq!(
        decoded.token,
        compact_field(compact_record(&records, "plan"), "token")
    );
    (records, file)
}

pub(super) fn refresh(
    public: &Native,
    input: &Path,
    plan: &[CompactRecord],
    onto: &str,
    output: Option<&Path>,
    success: bool,
) -> Vec<CompactRecord> {
    let mut args = vec![
        "change",
        "refresh",
        "--input-file",
        path(input),
        "--plan",
        compact_field(compact_record(plan, "plan"), "token"),
        "--onto",
        onto,
    ];
    if let Some(output) = output {
        args.extend(["--output", path(output)]);
    }
    public.cli(&args, success)
}

fn logical_records(file: &Path, operation: &str) -> Vec<CompactRecord> {
    parse_records("retained public review", &std::fs::read(file).unwrap())
        .unwrap()
        .into_iter()
        .filter(|record| record.operation == operation)
        .collect()
}

fn logical_values(records: &[CompactRecord]) -> Vec<(String, Vec<(String, String)>)> {
    records
        .iter()
        .map(|record| {
            (
                record.operation.clone(),
                record
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), field.value.clone()))
                    .collect(),
            )
        })
        .collect()
}

fn retain_evidence(public: Native) {
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        println!(
            "retained native refresh public evidence: {}",
            public.root.keep().display()
        );
    }
}

fn reject_refresh(public: &Native, input: &Path, plan: &[CompactRecord], onto: &str, name: &str) {
    let before = content_inventory(&public.project);
    let head = public.revision();
    let output = public.root.path().join(format!("rejected-{name}.lkjplan"));
    let rejected = refresh(public, input, plan, onto, Some(&output), false);
    assert!(
        rejected
            .iter()
            .any(|record| record.operation == "diagnostic"),
        "{rejected:?}"
    );
    assert!(
        !output.exists(),
        "failed refresh must not export a partial plan"
    );
    assert_eq!(public.revision(), head);
    assert_eq!(content_inventory(&public.project), before);
}

fn value(public: &Native, target: &str) -> String {
    compact_field(
        compact_record(&public.cli(&["run", target], true), "execution"),
        "value",
    )
    .to_owned()
}

fn create_in_module(public: &Native, filename: &str, declaration: &str) -> PathBuf {
    let module = find(public, "module", "refresh-suite", None);
    public.input(
        filename,
        &format!(
            "request base={}\ndeclarations.begin\n(units (module edit {module} refresh-suite\n{declaration}))\ndeclarations.end\n",
            public.revision()
        ),
    )
}

#[test]
fn native_refresh_disjoint_same_module_edits_keep_original_input_and_keyed_result() {
    let public = fixture();
    let module = find(&public, "module", "refresh-suite", None);
    let left = find(&public, "declaration", "left", Some(&module));
    let right = find(&public, "declaration", "right", Some(&module));
    let origin = public.revision();
    let original = edited(&public, "left", 10, 11);
    let literal = std::fs::read_to_string(&original).unwrap().replacen(
        '\n',
        " idempotency=refresh-left\n",
        1,
    );
    std::fs::write(&original, &literal).unwrap();
    let (review, original_plan) = plan_file(&public, &original, "left-original");
    let other = edited(&public, "right", 20, 22);
    let other_plan = public.plan(&other, true);
    public.apply(&other, &other_plan, true);
    let onto = public.revision();
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    reject_refresh(
        &public,
        &original,
        &other_plan,
        &onto,
        "wrong-original-token",
    );
    let tampered = public.input(
        "tampered-left.lkjc",
        &literal.replace("(i64 11)", "(i64 99)"),
    );
    reject_refresh(&public, &tampered, &review, &onto, "tampered-input");
    reject_refresh(&public, &original, &review, &origin, "old-onto");
    let refreshed_file = public.root.path().join("left-refreshed.lkjplan");
    let refreshed = refresh(
        &public,
        &original,
        &review,
        &onto,
        Some(&refreshed_file),
        true,
    );
    assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    assert_eq!(std::fs::read_to_string(&original).unwrap(), literal);
    assert_ne!(
        compact_field(compact_record(&review, "plan"), "token"),
        compact_field(compact_record(&refreshed, "plan"), "token")
    );
    let decoded =
        decode_logical_change_plan(BufReader::new(File::open(&refreshed_file).unwrap())).unwrap();
    assert_eq!(
        decoded.token,
        compact_field(compact_record(&refreshed, "plan"), "token")
    );
    assert_eq!(
        logical_values(&logical_records(&original_plan, "logical-plan.allocation")),
        logical_values(&logical_records(&refreshed_file, "logical-plan.allocation"))
    );
    let repeated = refresh(&public, &original, &review, &onto, None, true);
    assert_eq!(
        compact_field(compact_record(&refreshed, "plan"), "token"),
        compact_field(compact_record(&repeated, "plan"), "token")
    );
    let accepted = public.apply(&original, &refreshed, true);
    let accepted_revision =
        compact_field(compact_record(&accepted, "revision"), "result").to_owned();
    assert_eq!(value(&public, "refresh-left"), "11");
    assert_eq!(value(&public, "refresh-right"), "22");
    assert_eq!(find(&public, "declaration", "left", Some(&module)), left);
    assert_eq!(find(&public, "declaration", "right", Some(&module)), right);
    author(
        &public,
        "declarations.begin\n(units (module create later-descendant))\ndeclarations.end\n",
    );
    let descendant = public.revision();
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let replayed = public.apply(&original, &refreshed, true);
    assert_eq!(
        compact_field(compact_record(&replayed, "revision"), "result"),
        accepted_revision
    );
    assert_eq!(
        compact_field(compact_record(&replayed, "result"), "status"),
        "already-accepted"
    );
    assert_eq!(public.revision(), descendant);
    assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    reject_refresh(
        &public,
        &original,
        &review,
        &descendant,
        "accepted-key-new-target",
    );
    public.cli(&["check"], true);
    retain_evidence(public);
}

#[test]
fn native_refresh_supplied_shared_type_closure_and_generated_identities_survive_store_growth() {
    let public = fixture();
    let original = create_in_module(
        &public,
        "nested-original.lkjc",
        r#"
    (function create nested-first (as $nested) (visibility private)
      (returns (list (list I64))) (effect pure)
      (body (list (list I64) (list I64 (i64 1) (i64 2)))))"#,
    );
    let (review, original_file) = plan_file(&public, &original, "nested-original");
    let allocations = logical_records(&original_file, "logical-plan.allocation");
    assert!(
        allocations.len() >= 4,
        "must cover anonymous nested expression identities"
    );
    let types = logical_records(&original_file, "logical-plan.type-addition");
    assert!(
        types.len() >= 2,
        "must supply the complete nested list type closure"
    );
    let other = create_in_module(
        &public,
        "nested-other.lkjc",
        r#"
    (function create nested-second (visibility private)
      (returns (list (list I64))) (effect pure)
      (body (list (list I64) (list I64 (i64 3) (i64 4)))))"#,
    );
    public.apply(&other, &public.plan(&other, true), true);
    let onto = public.revision();
    let file = public.root.path().join("nested-refreshed.lkjplan");
    let refreshed = refresh(&public, &original, &review, &onto, Some(&file), true);
    assert_eq!(
        logical_values(&logical_records(&file, "logical-plan.allocation")),
        logical_values(&allocations)
    );
    assert_eq!(
        logical_values(&logical_records(&file, "logical-plan.type-addition")),
        logical_values(&types)
    );
    assert_eq!(
        identity(&review, "$nested"),
        identity(&refreshed, "$nested")
    );
    public.apply(&original, &refreshed, true);
    let module = find(&public, "module", "refresh-suite", None);
    assert_eq!(
        find(&public, "declaration", "nested-first", Some(&module)),
        identity(&review, "$nested")
    );
    find(&public, "declaration", "nested-second", Some(&module));
    public.cli(&["check"], true);
    retain_evidence(public);
}

#[test]
fn native_refresh_extraction_preserves_moved_expressions_and_generated_capture_parameters() {
    let public = fixture();
    let module = find(&public, "module", "refresh-suite", None);
    let creation = public.input("capture-fixture.lkjc", &format!(
        "request base={}\n\
         expression.text as=$value value=hello\n\
         expression.local as=$first value=$binding\n\
         expression.local as=$second value=$binding\n\
         expression.sequence as=$selected\n\
         expression.argument parent=$selected index=0 expression=$first\n\
         expression.argument parent=$selected index=1 expression=$second\n\
         expression.let as=$body body=$selected\n\
         expression.binding parent=$body index=0 as=$binding name=value value=$value type=text\n\
         create.function as=$captured module={module} name=captured visibility=private result=text effect=pure body=$body\n\
         type.function as=@Captured result=text\n\
         create.component as=$console module={module} name=CaptureConsole visibility=private\n\
         add.port as=$port component=$console name=main type=@Captured function=$captured\n\
         create.target as=$target name=refresh-capture component=$console port=$port runner=command\n",
        public.revision()
    ));
    let created = public.apply(&creation, &public.plan(&creation, true), true);
    let function = identity(&created, "$captured");
    let selected = identity(&created, "$selected");
    let first = identity(&created, "$first");
    let second = identity(&created, "$second");
    assert_eq!(value(&public, "refresh-capture"), "\"hello\"");
    let original = public.input("capture-original.lkjc", &format!(
        "request base={} idempotency=refresh-extraction\nextract.function as=$helper function={function} expression={selected} name=read-captured\n",
        public.revision()
    ));
    let (review, original_file) = plan_file(&public, &original, "capture-original");
    let captures = logical_records(&original_file, "logical-plan.extraction-capture");
    assert_eq!(captures.len(), 1);
    let parameter = compact_field(&captures[0], "parameter").to_owned();
    let allocations = logical_records(&original_file, "logical-plan.allocation");
    assert!(
        allocations
            .iter()
            .any(|record| compact_field(record, "owner") == parameter)
    );
    let other = edited(&public, "right", 20, 22);
    public.apply(&other, &public.plan(&other, true), true);
    let file = public.root.path().join("capture-refreshed.lkjplan");
    let refreshed = refresh(
        &public,
        &original,
        &review,
        &public.revision(),
        Some(&file),
        true,
    );
    assert_eq!(
        logical_values(&logical_records(&file, "logical-plan.allocation")),
        logical_values(&allocations)
    );
    assert_eq!(
        logical_values(&logical_records(&file, "logical-plan.extraction-capture")),
        logical_values(&captures)
    );
    let helper = identity(&refreshed, "$helper");
    assert_eq!(helper, identity(&review, "$helper"));
    public.apply(&original, &refreshed, true);
    let definition = public.cli(
        &[
            "inspect",
            "owner",
            "pure_function",
            &helper,
            "--detail",
            "definition",
            "--limit",
            "100",
            "--bytes",
            "65536",
        ],
        true,
    );
    assert_eq!(
        compact_field(compact_record(&definition, "page"), "complete"),
        "true"
    );
    for owner in [&selected, &first, &second] {
        assert!(
            definition
                .iter()
                .any(|record| record.operation == "definition.expression"
                    && compact_field(record, "id") == owner)
        );
    }
    assert!(
        definition
            .iter()
            .any(|record| record.operation == "definition.parameter"
                && compact_field(record, "id") == parameter)
    );
    assert_eq!(
        find(&public, "declaration", "captured", Some(&module)),
        function
    );
    assert_eq!(value(&public, "refresh-capture"), "\"hello\"");
    assert_eq!(value(&public, "refresh-right"), "22");
    public.cli(&["check"], true);
    retain_evidence(public);
}

#[test]
fn native_refresh_rejects_changed_negative_namespace_and_new_incoming_reference() {
    for incoming in [false, true] {
        let public = fixture();
        let module = find(&public, "module", "refresh-suite", None);
        let dormant = find(&public, "declaration", "dormant", Some(&module));
        let original = if incoming {
            public.input(
                "delete-original.lkjc",
                &format!(
                    "request base={}\ndelete.owner owner={dormant} policy=owned-closure\n",
                    public.revision()
                ),
            )
        } else {
            create_in_module(
                &public,
                "name-original.lkjc",
                r#"
    (function create claimed-name (visibility private)
      (returns I64) (effect pure) (body (i64 5)))"#,
            )
        };
        let review = public.plan(&original, true);
        let other = create_in_module(
            &public,
            "conflicting-other.lkjc",
            if incoming {
                r#"
    (function create new-referrer (visibility private)
      (returns I64) (effect pure) (body (call refresh-suite::dormant)))"#
            } else {
                r#"
    (function create claimed-name (visibility private)
      (returns I64) (effect pure) (body (i64 6)))"#
            },
        );
        public.apply(&other, &public.plan(&other, true), true);
        reject_refresh(
            &public,
            &original,
            &review,
            &public.revision(),
            if incoming { "incoming" } else { "namespace" },
        );
        assert_eq!(
            find(&public, "declaration", "dormant", Some(&module)),
            dormant
        );
        public.cli(&["check"], true);
        retain_evidence(public);
    }
}

#[test]
fn native_refresh_rejects_changed_callee_type_and_effect_contracts() {
    for effect in [false, true] {
        let public = fixture();
        let module = find(&public, "module", "refresh-suite", None);
        let callee = find(
            &public,
            "declaration",
            if effect { "dormant" } else { "ignore" },
            Some(&module),
        );
        let original = create_in_module(
            &public,
            "contract-original.lkjc",
            if effect {
                r#"
    (function create proposed-reader (visibility private)
      (returns I64) (effect pure) (body (call refresh-suite::dormant)))"#
            } else {
                r#"
    (function create proposed-reader (visibility private)
      (returns I64) (effect pure) (body (call refresh-suite::ignore (i64 3))))"#
            },
        );
        let review = public.plan(&original, true);
        let change = if effect {
            format!("set.function-contract as=%changed function={callee} result=i64 effect=task\n")
        } else {
            let parameter = find(&public, "parameter", "value", Some(&callee));
            format!("set.parameter-type parameter={parameter} type=bool\n")
        };
        let other = public.input(
            "changed-contract.lkjc",
            &format!("request base={}\n{change}", public.revision()),
        );
        public.apply(&other, &public.plan(&other, true), true);
        reject_refresh(
            &public,
            &original,
            &review,
            &public.revision(),
            if effect { "effect" } else { "type" },
        );
        public.cli(&["check"], true);
        retain_evidence(public);
    }
}

#[test]
fn native_refresh_rejects_changed_exact_dependency_even_with_compatible_interface() {
    let library = Native::new();
    author(
        &library,
        "declarations.begin\n(units (module create refresh-library\n(function create value (visibility public) (returns I64) (effect pure) (body (i64 42)))))\ndeclarations.end\n",
    );
    let first = export(&library);
    let public = fixture();
    stage(&public, &first);
    author(&public, &dependency(&first));
    let original = public.input(
        "dependency-original.lkjc",
        &format!(
            r#"request base={}
declarations.begin
(units (use api {} {})
  (module create proposed-consumer
    (function create read (visibility private) (returns I64) (effect pure)
      (body (call api::value)))))
declarations.end
"#,
            public.revision(),
            first.package,
            first.revision
        ),
    );
    let review = public.plan(&original, true);
    let file = library.root.path().join("library-draft.lkjc");
    library.cli(
        &[
            "change",
            "draft",
            "--declaration",
            "refresh-library::value",
            "--output",
            path(&file),
        ],
        true,
    );
    let source = std::fs::read_to_string(&file).unwrap();
    let changed = library.input(
        "library-changed.lkjc",
        &source.replacen("(i64 42)", "(i64 43)", 1),
    );
    library.apply(&changed, &library.plan(&changed, true), true);
    // Preserve both immutable transports: the authoring and export helper owns this disposable path.
    std::fs::remove_file(&first.path).unwrap();
    let second = export(&library);
    stage(&public, &second);
    let replacement = public.input("dependency-other.lkjc", &format!(
        "request base={}\nreplace.dependency package={} semantic-revision={} package-revision={}\n",
        public.revision(), second.package, second.semantic, second.revision));
    public.apply(&replacement, &public.plan(&replacement, true), true);
    reject_refresh(
        &public,
        &original,
        &review,
        &public.revision(),
        "dependency",
    );
    public.cli(&["check"], true);
    retain_evidence(library);
    retain_evidence(public);
}

#[test]
fn native_refresh_stale_publication_target_and_concurrent_applies_have_one_complete_winner() {
    let public = fixture();
    let left = edited(&public, "left", 10, 11);
    let right = edited(&public, "right", 20, 22);
    let left_plan = public.plan(&left, true);
    let right_plan = public.plan(&right, true);
    author(
        &public,
        "declarations.begin\n(units (module create concurrent-marker))\ndeclarations.end\n",
    );
    let onto = public.revision();
    let refreshed_left = refresh(&public, &left, &left_plan, &onto, None, true);
    let refreshed_right = refresh(&public, &right, &right_plan, &onto, None, true);
    let children: Vec<_> = [(&left, &refreshed_left), (&right, &refreshed_right)]
        .into_iter()
        .map(|(input, plan)| {
            support::spawn(
                Command::new(&public.executable)
                    .args([
                        "--project",
                        path(&public.project),
                        "change",
                        "apply",
                        "--input-file",
                        path(input),
                        "--plan",
                        compact_field(compact_record(plan, "plan"), "token"),
                    ])
                    .current_dir(public.root.path())
                    .env_clear()
                    .env("PATH", "")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped()),
            )
            .unwrap()
        })
        .collect();
    // Join every child before inspecting either result, including when one loses the race.
    let outputs: Vec<_> = children
        .into_iter()
        .map(|child| child.wait_with_output().unwrap())
        .collect();
    assert_eq!(
        outputs
            .iter()
            .filter(|output| output.status.success())
            .count(),
        1,
        "{}",
        outputs
            .iter()
            .map(|output| String::from_utf8_lossy(&output.stdout))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let left_won = outputs[0].status.success();
    assert_eq!(
        value(&public, "refresh-left"),
        if left_won { "11" } else { "10" }
    );
    assert_eq!(
        value(&public, "refresh-right"),
        if left_won { "20" } else { "22" }
    );
    let loser = parse_records(
        "lost concurrent apply",
        &outputs[usize::from(left_won)].stdout,
    )
    .unwrap();
    assert!(loser.iter().any(|record| record.operation == "diagnostic"));
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    public.apply(
        if left_won { &right } else { &left },
        if left_won {
            &refreshed_right
        } else {
            &refreshed_left
        },
        false,
    );
    assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    public.cli(&["check"], true);
    retain_evidence(public);
}
