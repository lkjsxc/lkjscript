//! Reviewed input-contract evolution through an isolated copy of the public executable.

use super::*;

struct ParameterProject {
    temporary: tempfile::TempDir,
    executable: PathBuf,
    project: PathBuf,
}

impl ParameterProject {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let executable = temporary.path().join("lkjscript");
        copy_executable(&binary(), &executable);
        let project = temporary.path().join("project");
        compact_success_at(
            &executable,
            temporary.path(),
            &["new", path(&project), "--template", "command"],
        );
        Self {
            temporary,
            executable,
            project,
        }
    }

    fn run(&self, arguments: &[&str]) -> Vec<CompactRecord> {
        let mut all = vec!["--project", path(&self.project)];
        all.extend_from_slice(arguments);
        compact_success_at(&self.executable, self.temporary.path(), &all)
    }

    fn revision(&self) -> String {
        current_revision_at(&self.executable, self.temporary.path(), &self.project)
    }

    fn request(&self, changes: &str) -> String {
        format!("request base={}\n{changes}", self.revision())
    }

    fn plan(&self, request: &str) -> Vec<CompactRecord> {
        self.run(&["change", "plan", "--input", request])
    }

    fn apply(&self, request: &str) -> Vec<CompactRecord> {
        let plan = self.plan(request);
        self.run(&[
            "change",
            "apply",
            "--input",
            request,
            "--plan",
            token(&plan),
        ]);
        plan
    }

    fn reject(&self, request: &str, code: &str) -> Vec<CompactRecord> {
        let before = content_inventory(&self.project);
        let head = self.revision();
        let output = command_at(
            &self.executable,
            self.temporary.path(),
            &[
                "--project",
                path(&self.project),
                "change",
                "plan",
                "--input",
                request,
            ],
        );
        assert!(!output.status.success(), "unexpectedly admitted: {request}");
        let records = parse_records("rejected parameter edit", &output.stdout).unwrap();
        assert!(
            records.iter().any(|record| {
                record.operation == "diagnostic" && compact_field(record, "code") == Some(code)
            }),
            "{code}: {records:?}"
        );
        assert_eq!(self.revision(), head);
        assert_eq!(content_inventory(&self.project), before);
        records
    }

    fn definition(&self, kind: &str, owner: &str) -> Vec<CompactRecord> {
        self.run(&[
            "inspect",
            "owner",
            kind,
            owner,
            "--detail",
            "definition",
            "--limit",
            "100",
            "--bytes",
            "65536",
        ])
    }
}

fn token(records: &[CompactRecord]) -> &str {
    compact_field(compact_record(records, "plan"), "token").unwrap()
}

fn identity(records: &[CompactRecord], symbol: &str) -> String {
    records
        .iter()
        .find(|record| {
            record.operation == "identity" && compact_field(record, "symbol") == Some(symbol)
        })
        .and_then(|record| compact_field(record, "id"))
        .unwrap()
        .to_owned()
}

fn parameter_fields(records: &[CompactRecord], owner: &str) -> BTreeMap<String, String> {
    let record = records
        .iter()
        .find(|record| {
            record.operation == "definition.parameter" && compact_field(record, "id") == Some(owner)
        })
        .unwrap();
    record
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.value.clone()))
        .collect()
}

#[test]
fn reviewed_parameter_type_edit_repairs_complete_consumers_and_preserves_identity() {
    let fixture = ParameterProject::new();
    let initial = fixture.apply(&fixture.request(r#"create.module as=$m name=inputs
add.parameter as=$sample function=$pick name=sample type=f64
add.parameter as=$enabled function=$pick name=enabled type=bool
expression.block as=$body
  (local $sample)
expression.end
create.function as=$pick module=$m name=pick visibility=public result=f64 effect=pure body=$body
expression.block as=$direct-body
  (call $pick (f64 3) (bool true))
expression.end
create.function as=$direct module=$m name=direct visibility=private result=f64 effect=pure body=$direct-body
expression.block as=$unreachable-body
  (if (bool true) (f64 0) (call $pick (f64 4) (bool false)))
expression.end
create.function as=$unreachable module=$m name=unreachable visibility=private result=f64 effect=pure body=$unreachable-body
type.function as=@Pick result=f64
type.argument parent=@Pick index=0 type=f64
type.argument parent=@Pick index=1 type=bool
expression.block as=$bound-body
  (function-value $pick)
expression.end
create.function as=$bound module=$m name=bound visibility=private result=@Pick effect=pure body=$bound-body
create.component as=$component module=$m name=compute visibility=public
add.port as=$port component=$component name=pick type=@Pick function=$pick
create.target as=$target name=pick component=$component port=$port runner=command
"#));
    let module = identity(&initial, "$m");
    let pick = identity(&initial, "$pick");
    let sample = identity(&initial, "$sample");
    let enabled = identity(&initial, "$enabled");
    let direct = identity(&initial, "$direct");
    let unreachable = identity(&initial, "$unreachable");
    let bound = identity(&initial, "$bound");
    let port = identity(&initial, "$port");
    let component = identity(&initial, "$component");
    let target = identity(&initial, "$target");
    let before = fixture.definition("pure_function", &pick);
    let sample_before = parameter_fields(&before, &sample);
    let enabled_before = parameter_fields(&before, &enabled);
    assert_eq!(sample_before["parent"], pick);
    assert_eq!(sample_before["index"], "0");
    assert_eq!(enabled_before["index"], "1");
    assert_eq!(sample_before["use"], "unrestricted");
    assert_eq!(sample_before["requirement"], "none");

    let new_contract = format!(
        r#"reference.owner as=$module package=local class=module name=inputs
reference.owner as=$pick package=local class=declaration parent=$module name=pick
reference.owner as=$sample package=local class=parameter parent=$pick name=sample
create.record as=$Reading module={module} name=Reading visibility=public
add.field as=$value record=$Reading name=value type=f64
type.named as=@Reading declaration=$Reading
set.parameter-type parameter=$sample type=@Reading
expression.block as=$body
  (field (local $sample) $value)
expression.end
replace.body function={pick} body=$body
type.function as=@Pick result=f64
type.argument parent=@Pick index=0 type=@Reading
type.argument parent=@Pick index=1 type=bool
"#
    );
    let direct_repair = format!(
        r#"expression.block as=$direct
  (call $pick (record $Reading (field $value (f64 3))) (bool true))
expression.end
replace.body function={direct} body=$direct
"#
    );
    let dead_repair = format!(
        r#"expression.block as=$unreachable
  (if (bool true) (f64 0) (call $pick (record $Reading (field $value (f64 4))) (bool false)))
expression.end
replace.body function={unreachable} body=$unreachable
"#
    );
    let bound_repair =
        format!("set.function-contract as=%contract function={bound} result=@Pick effect=pure\n");
    let port_repair = format!("set.port-contract port={port} type=@Pick\n");
    for (omitted, expected) in [
        (0, "kernel_type_argument"),
        (1, "kernel_type_argument"),
        (2, "kernel_type_root"),
        (3, "kernel_type_port_function"),
    ] {
        let repairs = [&direct_repair, &dead_repair, &bound_repair, &port_repair];
        let changes = new_contract.clone()
            + &repairs
                .into_iter()
                .enumerate()
                .filter(|(index, _)| *index != omitted)
                .map(|(_, repair)| repair.as_str())
                .collect::<String>();
        fixture.reject(&fixture.request(&changes), expected);
        assert_eq!(
            parameter_fields(&fixture.definition("pure_function", &pick), &sample),
            sample_before
        );
    }
    let complete = new_contract + &direct_repair + &dead_repair + &bound_repair + &port_repair;
    let old_base = fixture.revision();
    let request = format!("request base={old_base} idempotency=parameter-evolution\n{complete}");
    let review = fixture.plan(&request);
    let inventory = content_inventory(&fixture.project);
    let tampered = request.replace("(f64 3)", "(f64 7)");
    let rejected = command_at(
        &fixture.executable,
        fixture.temporary.path(),
        &[
            "--project",
            path(&fixture.project),
            "change",
            "apply",
            "--input",
            &tampered,
            "--plan",
            token(&review),
        ],
    );
    assert!(!rejected.status.success());
    let diagnostics = parse_records("tampered parameter review", &rejected.stdout).unwrap();
    assert_eq!(
        compact_field(compact_record(&diagnostics, "diagnostic"), "code"),
        Some("change_request_commitment_mismatch")
    );
    assert_eq!(content_inventory(&fixture.project), inventory);
    let applied = fixture.run(&[
        "change",
        "apply",
        "--input",
        &request,
        "--plan",
        token(&review),
    ]);
    let accepted = compact_field(compact_record(&applied, "revision"), "result")
        .unwrap()
        .to_owned();
    let mut sample_after = parameter_fields(&fixture.definition("pure_function", &pick), &sample);
    assert_ne!(sample_after["type"], sample_before["type"]);
    sample_after.insert("type".to_owned(), sample_before["type"].clone());
    assert_eq!(sample_after, sample_before);
    assert_eq!(
        parameter_fields(&fixture.definition("pure_function", &pick), &enabled),
        enabled_before
    );
    for (class, name, owner, parent) in [
        (
            "declaration",
            "compute",
            component.as_str(),
            Some(module.as_str()),
        ),
        ("target", "pick", target.as_str(), None),
        ("port", "pick", port.as_str(), Some(component.as_str())),
    ] {
        let mut arguments = vec!["query", "find", class, name];
        if let Some(parent) = parent {
            arguments.extend(["--parent", parent]);
        }
        let owners = fixture.run(&arguments);
        assert!(owners.iter().any(
            |record| record.operation == "owner" && compact_field(record, "id") == Some(owner)
        ));
    }
    let retried = fixture.run(&[
        "change",
        "apply",
        "--input",
        &request,
        "--plan",
        token(&review),
    ]);
    assert_eq!(
        compact_field(compact_record(&retried, "revision"), "result"),
        Some(accepted.as_str())
    );
    assert_eq!(fixture.revision(), accepted);
    fixture.reject(
        &request.replace(" idempotency=parameter-evolution", ""),
        "change_authored_stale_base",
    );
    fixture.run(&["check"]);
}

#[test]
fn parameter_type_exact_and_reference_alias_publish_the_same_canonical_meaning() {
    let fixture = ParameterProject::new();
    let plan = fixture.apply(&fixture.request(
        r#"create.module as=$m name=parity
expression.unit as=$body
create.function as=$f module=$m name=ignore visibility=private result=unit effect=pure body=$body
add.parameter as=$p function=$f name=value type=f64
"#,
    ));
    let function = identity(&plan, "$f");
    let parameter = identity(&plan, "$p");
    let alias_project = fixture.temporary.path().join("alias");
    copy_regular_tree(&fixture.project, &alias_project);
    let base = fixture.revision();
    fixture.apply(&format!(
        "request base={base}\nset.parameter-type parameter={parameter} type=i64\n"
    ));
    let alias = format!(
        "request base={base}\nreference.owner as=$module package=local class=module name=parity\nreference.owner as=$function package=local class=declaration parent=$module name=ignore\nreference.owner as=$input package=local class=parameter parent=$function name=value\nset.parameter-type parameter=$input type=i64\n"
    );
    let run = |arguments: &[&str]| {
        let mut all = vec!["--project", path(&alias_project)];
        all.extend_from_slice(arguments);
        compact_success_at(&fixture.executable, fixture.temporary.path(), &all)
    };
    let review = run(&["change", "plan", "--input", &alias]);
    run(&[
        "change",
        "apply",
        "--input",
        &alias,
        "--plan",
        token(&review),
    ]);
    let literal = fixture.run(&["status"]);
    let symbolic = run(&["status"]);
    assert_eq!(
        compact_field(compact_record(&literal, "root"), "digest"),
        compact_field(compact_record(&symbolic, "root"), "digest")
    );
    assert_eq!(
        compact_field(compact_record(&literal, "state"), "digest"),
        compact_field(compact_record(&symbolic, "state"), "digest")
    );
    let definition = run(&[
        "inspect",
        "owner",
        "pure_function",
        &function,
        "--detail",
        "definition",
        "--limit",
        "100",
        "--bytes",
        "65536",
    ]);
    assert_eq!(
        parameter_fields(&fixture.definition("pure_function", &function), &parameter),
        parameter_fields(&definition, &parameter)
    );
}

#[test]
fn parameter_type_uses_the_defining_generic_scope_and_admits_task_operation_and_external_contracts()
{
    let fixture = ParameterProject::new();
    let plan = fixture.apply(&fixture.request(r#"create.module as=$m name=scopes
expression.unit as=$body
create.function as=$f module=$m name=generic visibility=private result=unit effect=pure body=$body
add.type-parameter as=$T declaration=$f name=T
add.parameter as=$p function=$f name=value type=unit
expression.unit as=$other-body
create.function as=$other module=$m name=other visibility=private result=unit effect=pure body=$other-body
add.type-parameter as=$U declaration=$other name=U
expression.unit as=$task-body
create.function as=$task module=$m name=task visibility=private result=unit effect=task body=$task-body
add.parameter as=$task-input function=$task name=value type=unit
create.interface as=$interface module=$m name=Authority visibility=private
add.operation as=$operation interface=$interface name=observe result=unit idempotency=idempotent external-visibility=none
add.parameter as=$operation-input operation=$operation name=value type=unit
create.external as=$external module=$m name=add visibility=private result=f64 implementation=core.f64.add
add.parameter as=$left function=$external name=left type=f64
add.parameter as=$right function=$external name=right type=f64
"#));
    let parameter = identity(&plan, "$p");
    let own_type = identity(&plan, "$T");
    let foreign_type = identity(&plan, "$U");
    let function = identity(&plan, "$f");
    let module = identity(&plan, "$m");
    let task = identity(&plan, "$task");
    let task_input = identity(&plan, "$task-input");
    let operation_input = identity(&plan, "$operation-input");
    let external_input = identity(&plan, "$left");
    fixture.reject(&fixture.request(&format!("type.parameter as=@Foreign parameter={foreign_type}\nset.parameter-type parameter={parameter} type=@Foreign\n")), "kernel_type_parameter_scope");
    fixture.reject(
        &fixture.request(&format!(
            "set.parameter-type parameter={external_input} type=bool\n"
        )),
        "intrinsic_signature",
    );
    let wrong_kind = fixture.reject(
        &fixture.request(&format!(
            "set.parameter-type parameter={function} type=unit\n"
        )),
        "change_mutation_owner_kind",
    );
    assert_eq!(
        compact_field(compact_record(&wrong_kind, "diagnostic"), "line"),
        Some("2")
    );
    fixture.reject(&fixture.request("reference.package as=$std source=builtin\nreference.owner as=$add package=$std class=declaration name=f64-add\nreference.owner as=$input package=$std class=parameter parent=$add name=left\nset.parameter-type parameter=$input type=unit\n"), "change_reference_foreign_local");
    let missing = format!("param_{}", "1".repeat(32));
    // A syntactically valid absent identity reaches owner admission, never allocation.
    let absent = fixture.reject(
        &fixture.request(&format!(
            "set.parameter-type parameter={missing} type=unit\n"
        )),
        "change_authored_owner_missing",
    );
    assert_eq!(
        compact_field(compact_record(&absent, "diagnostic"), "line"),
        Some("2")
    );

    let generic_before =
        parameter_fields(&fixture.definition("pure_function", &function), &parameter);
    let task_before = parameter_fields(&fixture.definition("task_function", &task), &task_input);
    let coherent = fixture.request(&format!(r#"type.parameter as=@Own parameter={own_type}
type.parameter as=@Foreign parameter={foreign_type}
expression.unit as=$recent-body
create.function as=$recent module={module} name=most-recent visibility=private result=unit effect=pure body=$recent-body
add.type-parameter as=$Recent declaration=$recent name=Recent
set.parameter-type parameter={parameter} type=@Own
set.parameter-type parameter={task_input} type=bool
set.parameter-type parameter={operation_input} type=bool
set.parameter-type parameter={external_input} type=f64
"#));
    fixture.apply(&coherent);
    for (kind, owner, selected, original) in [
        (
            "pure_function",
            function.as_str(),
            parameter.as_str(),
            generic_before,
        ),
        (
            "task_function",
            task.as_str(),
            task_input.as_str(),
            task_before,
        ),
    ] {
        let mut updated = parameter_fields(&fixture.definition(kind, owner), selected);
        assert_ne!(updated["type"], original["type"]);
        updated.insert("type".to_owned(), original["type"].clone());
        assert_eq!(updated, original);
    }
    let base = fixture.revision();
    let stale_request =
        format!("request base={base}\nset.parameter-type parameter={task_input} type=unit\n");
    let stale_plan = fixture.plan(&stale_request);
    fixture.apply(&fixture.request(&format!("rename.owner owner={task_input} name=renamed\n")));
    let after_rename = content_inventory(&fixture.project);
    let rejected = command_at(
        &fixture.executable,
        fixture.temporary.path(),
        &[
            "--project",
            path(&fixture.project),
            "change",
            "apply",
            "--input",
            &stale_request,
            "--plan",
            token(&stale_plan),
        ],
    );
    assert_eq!(rejected.status.code(), Some(7));
    assert_eq!(content_inventory(&fixture.project), after_rename);
    fixture.run(&["check"]);
}

#[test]
fn parameter_type_intrinsic_rejection_locates_the_bad_argument_before_a_later_noop() {
    let fixture = ParameterProject::new();
    let plan = fixture.apply(&fixture.request(
        r#"create.module as=$m name=intrinsic-locations
create.external as=$add module=$m name=add visibility=private result=f64 implementation=core.f64.add
add.parameter as=$left function=$add name=left type=f64
add.parameter as=$right function=$add name=right type=f64
"#,
    ));
    let left = identity(&plan, "$left");
    let right = identity(&plan, "$right");
    for (invalid, unchanged) in [(&left, &right), (&right, &left)] {
        let diagnostics = fixture.reject(
            &fixture.request(&format!(
                "set.parameter-type parameter={invalid} type=bool\nset.parameter-type parameter={unchanged} type=f64\n"
            )),
            "intrinsic_signature",
        );
        let diagnostic = compact_record(&diagnostics, "diagnostic");
        assert_eq!(compact_field(diagnostic, "line"), Some("2"));
        let notes = compact_field(diagnostic, "notes").unwrap();
        assert!(notes.contains(invalid));
        assert!(!notes.contains(unchanged));
    }
}

#[test]
fn parameter_type_generic_unary_intrinsic_rejection_identifies_its_input() {
    let fixture = ParameterProject::new();
    let plan = fixture.apply(&fixture.request(
        r#"create.module as=$m name=generic-intrinsic-locations
type.list as=@Samples item=f64
create.external as=$length module=$m name=length visibility=private result=i64 implementation=core.list.length
add.parameter as=$input function=$length name=items type=@Samples
"#,
    ));
    let input = identity(&plan, "$input");
    let diagnostics = fixture.reject(
        &fixture.request(&format!("set.parameter-type parameter={input} type=bool\n")),
        "intrinsic_signature",
    );
    let diagnostic = compact_record(&diagnostics, "diagnostic");
    assert_eq!(compact_field(diagnostic, "line"), Some("2"));
    assert!(compact_field(diagnostic, "notes").unwrap().contains(&input));
}

#[test]
fn parameter_type_edits_cannot_reinterpret_affine_modes_or_exact_requirement_authority() {
    let fixture = ParameterProject::new();
    let plan = fixture.apply(&fixture.request(r#"create.module as=$m name=affine-inputs
create.interface as=$interface module=$m name=Lease visibility=private
type.capability-resource as=@Lease interface=$interface
add.operation as=$finish interface=$interface name=finish result=unit idempotency=non-idempotent external-visibility=possible
add.parameter as=$operation-input operation=$finish name=lease type=@Lease use=consume
create.component as=$component module=$m name=Scope visibility=private
expression.unit as=$noop-body
create.function as=$noop module=$m name=noop visibility=private result=unit effect=pure body=$noop-body
type.function as=@Noop result=unit
add.port as=$noop-port component=$component name=noop type=@Noop function=$noop
add.requirement as=$requirement component=$component name=lease-authority interface=$interface
requirement.operation parent=$requirement index=0 operation=$finish
expression.local as=$local value=$lease
expression.capability-call as=$body requirement=$requirement operation=$finish
expression.argument parent=$body index=0 expression=$local
create.function as=$task module=$m name=finish-lease visibility=private result=unit effect=task body=$body
effect.requirement parent=$task index=0 requirement=$requirement
add.parameter as=$tag function=$task name=tag type=unit
add.parameter as=$lease function=$task name=lease type=@Lease use=consume requirement=$requirement
"#));
    let task = identity(&plan, "$task");
    let lease = identity(&plan, "$lease");
    let tag = identity(&plan, "$tag");
    let operation_input = identity(&plan, "$operation-input");
    let before = parameter_fields(&fixture.definition("task_function", &task), &lease);
    assert_eq!(before["use"], "consume");
    assert!(before["requirement"].ends_with(&identity(&plan, "$requirement")));
    for (selected, code) in [
        (lease.as_str(), "kernel_affine_function_parameter_use"),
        (
            operation_input.as_str(),
            "kernel_affine_nonresource_parameter_use",
        ),
    ] {
        fixture.reject(
            &fixture.request(&format!(
                "set.parameter-type parameter={selected} type=unit\n"
            )),
            code,
        );
        assert_eq!(
            parameter_fields(&fixture.definition("task_function", &task), &lease),
            before
        );
    }
    // Repeating the exact resource type alongside an ordinary input edit retains its authority.
    fixture.apply(&fixture.request(&format!(
        r#"reference.owner as=$m package=local class=module name=affine-inputs
reference.owner as=$interface package=local class=declaration parent=$m name=Lease
type.capability-resource as=@Lease interface=$interface
set.parameter-type parameter={lease} type=@Lease
set.parameter-type parameter={tag} type=bool
"#
    )));
    assert_eq!(
        parameter_fields(&fixture.definition("task_function", &task), &lease),
        before
    );
    fixture.run(&["check"]);
}
