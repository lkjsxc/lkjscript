#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "public subprocess acceptance reports assertion failures"
)]

mod support;

use lkjscript::platform::control::{CompactRecord, parse_records};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Public {
    directory: tempfile::TempDir,
    executable: PathBuf,
    project: PathBuf,
}

impl Public {
    fn new(template: &str) -> Self {
        let source = std::env::var_os("LKJSCRIPT_RELEASE_CANDIDATE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_lkjscript")));
        Self::with_executable(&source, template)
    }

    fn with_executable(source: &Path, template: &str) -> Self {
        assert!(source.is_absolute());
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("lkjscript");
        support::copy_executable(source, &executable);
        let project = directory.path().join("author");
        let public = Self {
            directory,
            executable,
            project,
        };
        public.success(&[
            "new",
            path(&public.project),
            "--template",
            template,
            "--name",
            "structural",
        ]);
        public
    }

    fn output(&self, arguments: &[&str]) -> Output {
        support::output(
            Command::new(&self.executable)
                .args(arguments)
                .current_dir(self.directory.path())
                .env_clear()
                .env("LANG", "C"),
        )
        .unwrap()
    }

    fn success(&self, arguments: &[&str]) -> Vec<CompactRecord> {
        let output = self.output(arguments);
        assert!(
            output.status.success(),
            "{arguments:?}: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        parse_records("public-output", &output.stdout).unwrap()
    }

    fn failure(&self, arguments: &[&str]) -> Vec<CompactRecord> {
        let output = self.output(arguments);
        assert!(
            output.status.code().is_some(),
            "subprocess crashed: {arguments:?}; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.status.success(),
            "unexpected acceptance: {arguments:?}"
        );
        assert!(
            output.stderr.is_empty(),
            "process-level failure: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let records = parse_records("public-error", &output.stdout).unwrap();
        assert!(
            records
                .iter()
                .any(|record| record.operation == "diagnostic")
        );
        records
    }

    fn write(&self, name: &str, source: &str) -> PathBuf {
        let destination = self.directory.path().join(name);
        std::fs::write(&destination, source).unwrap();
        destination
    }

    fn revision(&self) -> String {
        let records = self.success(&["--project", path(&self.project), "status"]);
        field(record(&records, "revision"), "id").to_owned()
    }

    fn plan(&self, input: &Path) -> Vec<CompactRecord> {
        self.success(&[
            "--project",
            path(&self.project),
            "change",
            "plan",
            "--input-file",
            path(input),
        ])
    }

    fn apply(&self, input: &Path, token: &str) -> Vec<CompactRecord> {
        self.success(&[
            "--project",
            path(&self.project),
            "change",
            "apply",
            "--input-file",
            path(input),
            "--plan",
            token,
        ])
    }

    fn run(&self, target: &str, arguments: &str) -> Value {
        let records = self.success(&[
            "--project",
            path(&self.project),
            "run",
            target,
            "--arguments",
            arguments,
        ]);
        let execution = record(&records, "execution");
        assert_eq!(field(execution, "differential"), "equal");
        serde_json::from_str(field(execution, "value")).unwrap()
    }
}

fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}
fn record<'a>(records: &'a [CompactRecord], operation: &str) -> &'a CompactRecord {
    records
        .iter()
        .find(|record| record.operation == operation)
        .unwrap_or_else(|| panic!("missing {operation}: {records:#?}"))
}
fn optional_field<'a>(record: &'a CompactRecord, name: &str) -> Option<&'a str> {
    record
        .fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.as_str())
}
fn field<'a>(record: &'a CompactRecord, name: &str) -> &'a str {
    optional_field(record, name).unwrap_or_else(|| panic!("missing {name}: {record:#?}"))
}
fn identity(records: &[CompactRecord], symbol: &str) -> String {
    field(
        records
            .iter()
            .find(|record| {
                record.operation == "identity" && optional_field(record, "symbol") == Some(symbol)
            })
            .unwrap(),
        "id",
    )
    .to_owned()
}

const PUBLIC_FLAT: &str = "\
expression.local as=$initial value=$parameter
expression.local as=$previous value=$first
expression.i64 as=$three value=3
expression.call as=$next function=$add
expression.argument parent=$next index=0 expression=$previous
expression.argument parent=$next index=1 expression=$three
expression.bool as=$condition value=true
expression.local as=$answer value=$second
expression.i64 as=$one value=1
expression.i64 as=$zero value=0
expression.call as=$trap function=$divide
expression.argument parent=$trap index=0 expression=$one
expression.argument parent=$trap index=1 expression=$zero
expression.if as=$selected condition=$condition when-true=$answer when-false=$trap
expression.let as=$body body=$selected
expression.binding parent=$body index=0 as=$first name=value type=i64 value=$initial
expression.binding parent=$body index=1 as=$second name=value value=$next
";

const PUBLIC_BLOCK: &str = "\
expression.block as=$body
  (let
    (binding value (type i64) (local $parameter))
    (binding value (call $add (local value) (i64 3)))
    (in (if (bool true) (local value) (call $divide (i64 1) (i64 0)))))
expression.end
";

const PUBLIC_DECLARATIONS: &str = "\
reference.package as=$std source=builtin
reference.owner as=$add package=$std class=declaration name=add
reference.owner as=$divide package=$std class=declaration name=divide
create.module as=$module name=structural-public
create.function as=$function module=$module name=calculate visibility=private result=i64 effect=pure body=$body
add.parameter as=$parameter function=$function name=input type=i64
type.function as=@entry result=i64
type.argument parent=@entry index=0 type=i64
create.component as=$component module=$module name=Component visibility=package
add.port as=$port component=$component name=calculate type=@entry function=$function
create.target as=$target name=calculate component=$component port=$port runner=command
";

#[test]
fn copied_binary_flat_plan_block_apply_and_block_plan_flat_apply_preserve_review_and_edit_identity()
{
    let public = Public::new("command");
    let discovery = public.success(&["capabilities", "--section", "change"]);
    assert!(
        discovery
            .iter()
            .flat_map(|record| &record.fields)
            .any(|field| field.value.contains("expression.block")),
        "structural authoring must be discoverable"
    );
    let base = public.revision();
    let header = format!(
        "request base={base} idempotency=structural-public-creation intent=notation-interchange\n"
    );
    let flat = public.write(
        "independent-flat.lkjc",
        &format!("{header}{PUBLIC_FLAT}{PUBLIC_DECLARATIONS}"),
    );
    let block = public.write(
        "structural.lkjc",
        &format!("{header}{PUBLIC_BLOCK}{PUBLIC_DECLARATIONS}"),
    );
    let before = std::fs::read(public.project.join("HEAD")).unwrap();
    let flat_plan = public.plan(&flat);
    let block_plan = public.plan(&block);
    let token = field(record(&flat_plan, "plan"), "token");
    assert_eq!(token, field(record(&block_plan, "plan"), "token"));
    assert_eq!(before, std::fs::read(public.project.join("HEAD")).unwrap());
    for symbol in [
        "$body",
        "$function",
        "$parameter",
        "$module",
        "$component",
        "$port",
        "$target",
    ] {
        assert_eq!(identity(&flat_plan, symbol), identity(&block_plan, symbol));
    }

    let changed = public.write(
        "changed-input.lkjc",
        &format!(
            "{header}{}{PUBLIC_DECLARATIONS}",
            PUBLIC_BLOCK.replace("(i64 3)", "(i64 4)")
        ),
    );
    public.failure(&[
        "--project",
        path(&public.project),
        "change",
        "apply",
        "--input-file",
        path(&changed),
        "--plan",
        token,
    ]);
    assert_eq!(before, std::fs::read(public.project.join("HEAD")).unwrap());

    let accepted = public.apply(&block, token);
    let function = identity(&accepted, "$function");
    let module = identity(&accepted, "$module");
    let parameter = identity(&accepted, "$parameter");
    let retired_body = identity(&accepted, "$body");
    let accepted_revision = public.revision();
    assert_ne!(base, accepted_revision);
    assert_eq!(public.run("calculate", "[8]"), json!(11));
    public.success(&["--project", path(&public.project), "check"]);
    public.success(&[
        "--project",
        path(&public.project),
        "build",
        "--output",
        path(&public.directory.path().join("structural.lkja")),
    ]);

    // Discover existing owners through the same public route a fresh author uses for an edit.
    let discovered = public.success(&[
        "--project",
        path(&public.project),
        "query",
        "find",
        "declaration",
        "calculate",
        "--parent",
        &module,
    ]);
    assert_eq!(field(record(&discovered, "owner"), "id"), function);
    let definition = public.success(&[
        "--project",
        path(&public.project),
        "inspect",
        "owner",
        "pure_function",
        &function,
        "--detail",
        "definition",
    ]);
    assert!(
        definition
            .iter()
            .flat_map(|record| &record.fields)
            .any(|field| field.value == parameter)
    );
    let edit_header = format!(
        "request base={accepted_revision} idempotency=structural-public-edit\nreference.package as=$std source=builtin\nreference.owner as=$add package=$std class=declaration name=add\n"
    );
    let replacement_flat = public.write("replacement-flat.lkjc", &format!(
        "{edit_header}expression.local as=$read value={parameter}\nexpression.i64 as=$eight value=8\nexpression.call as=$replacement function=$add\nexpression.argument parent=$replacement index=0 expression=$read\nexpression.argument parent=$replacement index=1 expression=$eight\nreplace.body function={function} body=$replacement\n"
    ));
    let replacement_block = public.write("replacement-block.lkjc", &format!(
        "{edit_header}expression.block as=$replacement\n(call $add (local (exact {parameter})) (i64 8))\nexpression.end\nreplace.body function={function} body=$replacement\n"
    ));
    let review_path = public.directory.path().join("replacement.plan");
    let replacement_plan = public.success(&[
        "--project",
        path(&public.project),
        "change",
        "plan",
        "--input-file",
        path(&replacement_block),
        "--output",
        path(&review_path),
    ]);
    let replacement_token = field(record(&replacement_plan, "plan"), "token");
    assert_eq!(
        replacement_token,
        field(record(&public.plan(&replacement_flat), "plan"), "token")
    );
    let logical = parse_records("replacement-plan", &std::fs::read(review_path).unwrap()).unwrap();
    assert!(
        logical
            .iter()
            .any(|record| record.operation == "logical-plan.retirement"
                && optional_field(record, "owner") == Some(&retired_body)
                && optional_field(record, "after-present") == Some("true"))
    );
    assert!(!logical.iter().any(|record| {
        record.operation == "logical-plan.retirement"
            && [Some(function.as_str()), Some(parameter.as_str())]
                .contains(&optional_field(record, "owner"))
    }));
    public.apply(&replacement_flat, replacement_token);
    assert_eq!(public.run("calculate", "[8]"), json!(16));
    let edited_revision = public.revision();
    let discovered = public.success(&[
        "--project",
        path(&public.project),
        "query",
        "find",
        "declaration",
        "calculate",
        "--parent",
        &module,
    ]);
    assert_eq!(field(record(&discovered, "owner"), "id"), function);
    let parameter_after = public.success(&[
        "--project",
        path(&public.project),
        "inspect",
        "owner",
        "pure_function",
        &function,
        "--detail",
        "definition",
    ]);
    assert!(
        parameter_after
            .iter()
            .flat_map(|record| &record.fields)
            .any(|field| field.value == parameter)
    );

    let retry = public.apply(&block, token);
    assert_eq!(
        field(record(&retry, "result"), "status"),
        "already-accepted"
    );
    assert_eq!(public.revision(), edited_revision);
    let stale = public.write(
        "stale.lkjc",
        &format!("request base={base}\n{PUBLIC_BLOCK}{PUBLIC_DECLARATIONS}"),
    );
    public.failure(&[
        "--project",
        path(&public.project),
        "change",
        "plan",
        "--input-file",
        path(&stale),
    ]);
    assert_eq!(public.revision(), edited_revision);

    let invalid = public.write("invalid-unselected-branch.lkjc", &format!(
        "request base={edited_revision}\ncreate.module as=$valid name=must-not-publish\nexpression.block as=$replacement\n(sequence\n  (unit)\n  (if (bool true) (i64 1) (text \"invalid even though unselected\")))\nexpression.end\nreplace.body function={function} body=$replacement\n"
    ));
    let errors = public.failure(&[
        "--project",
        path(&public.project),
        "change",
        "plan",
        "--input-file",
        path(&invalid),
    ]);
    let mismatch = errors
        .iter()
        .find(|record| {
            record.operation == "diagnostic"
                && optional_field(record, "code") == Some("kernel_type_if_branches")
        })
        .expect("complete branch type validation");
    assert_eq!(field(mismatch, "path"), path(&invalid));
    assert_eq!(field(mismatch, "line"), "6");
    assert_eq!(field(mismatch, "column"), "3");
    assert!(!field(mismatch, "message").contains("$__block_"));
    assert_eq!(public.revision(), edited_revision);
}

#[test]
fn copied_binary_structural_pure_map_record_nested_keys_ordered_calls_and_lazy_branch_have_independent_results()
 {
    let public = Public::new("command");
    let request = public.write("map-record.lkjc", &format!(
        "request base={}\n{}",
        public.revision(),
        r#"reference.package as=$std source=builtin
reference.owner as=$add package=$std class=declaration name=add
reference.owner as=$multiply package=$std class=declaration name=multiply
reference.owner as=$subtract package=$std class=declaration name=subtract
reference.owner as=$divide package=$std class=declaration name=divide
create.module as=$module name=structural-composition
type.map as=@Map key=text value=i64
type.structural-record as=@Result
type.field parent=@Result index=0 name=table type=@Map
type.field parent=@Result index=1 name=ordered type=i64
type.field parent=@Result index=2 name=lazy type=i64
expression.block as=$body
  (record structural
    (field table
      (map text i64
        (entry
          (field (record structural (field key (text "first"))) (name key))
          (call $add (i64 2) (i64 3)))
        (entry (text "second")
          (let (binding value (i64 9))
            (in (call $divide (local value) (i64 3)))))))
    (field ordered
      (let
        (binding value (call $multiply (i64 2) (i64 3)))
        (binding value (call $subtract (local value) (i64 4)))
        (in (local value))))
    (field lazy
      (if (bool true) (i64 5) (call $divide (i64 1) (i64 0)))))
expression.end
create.function as=$function module=$module name=compose visibility=private result=@Result effect=pure body=$body
type.function as=@Entry result=@Result
create.component as=$component module=$module name=Component visibility=package
add.port as=$port component=$component name=compose type=@Entry function=$function
create.target as=$target name=compose component=$component port=$port runner=command
"#
    ));
    let plan = public.plan(&request);
    public.apply(&request, field(record(&plan, "plan"), "token"));
    public.success(&["--project", path(&public.project), "check"]);
    assert_eq!(
        public.run("compose", "[]"),
        json!({"table": [["first", 5], ["second", 3]], "ordered": 2, "lazy": 5})
    );
}

fn nested_sequence(depth: usize) -> String {
    assert!(depth > 0);
    format!(
        "{}(unit){}",
        "(sequence ".repeat(depth - 1),
        ")".repeat(depth - 1)
    )
}

fn depth_request(base: &str, flat_ancestors: usize, block_depth: usize) -> String {
    let mut input = format!(
        "request base={base}\nexpression.block as=$block\n{}\nexpression.end\n",
        nested_sequence(block_depth)
    );
    for level in 0..flat_ancestors {
        let child = if level == 0 {
            "$block".to_owned()
        } else {
            format!("$flat-{}", level - 1)
        };
        input.push_str(&format!("expression.sequence as=$flat-{level}\nexpression.argument parent=$flat-{level} index=0 expression={child}\n"));
    }
    let root = if flat_ancestors == 0 {
        "$block".to_owned()
    } else {
        format!("$flat-{}", flat_ancestors - 1)
    };
    input.push_str(&format!("create.module as=$module name=depth\ncreate.function as=$function module=$module name=deep visibility=private result=unit effect=pure body={root}\n"));
    input
}

#[test]
fn copied_binary_admits_useful_depth_and_rejects_combined_depth_and_cumulative_growth_without_crashing()
 {
    let public = Public::new("minimal");
    let base = public.revision();
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    for (name, flat, block, accepted) in [
        ("block-1024", 0, 1024, true),
        ("block-1025", 0, 1025, false),
        ("mixed-1024", 512, 512, true),
        ("mixed-1025", 513, 512, false),
        ("flat-1024", 1023, 1, true),
        ("flat-1025", 1024, 1, false),
    ] {
        let input = public.write(&format!("{name}.lkjc"), &depth_request(&base, flat, block));
        if accepted {
            public.plan(&input);
        } else {
            let errors = public.failure(&[
                "--project",
                path(&public.project),
                "change",
                "plan",
                "--input-file",
                path(&input),
            ]);
            assert!(
                errors
                    .iter()
                    .filter(|record| record.operation == "diagnostic")
                    .any(|record| field(record, "code").contains("depth")),
                "{name}: {errors:#?}"
            );
        }
        assert_eq!(
            head,
            std::fs::read(public.project.join("HEAD")).unwrap(),
            "{name}"
        );
    }

    // Each block is below the default 100,000-identity request admission. Together they exceed
    // it despite using few physical records and less than the complete 4 MiB input allowance.
    let body = format!("(sequence {})", "(unit) ".repeat(60_000));
    let many = public.write("cumulative.lkjc", &format!(
        "request base={base}\ncreate.module as=$module name=cumulative\nexpression.block as=$first\n{body}\nexpression.end\ncreate.function as=$a module=$module name=a visibility=private result=unit effect=pure body=$first\nexpression.block as=$second\n{body}\nexpression.end\ncreate.function as=$b module=$module name=b visibility=private result=unit effect=pure body=$second\n"
    ));
    let errors = public.failure(&[
        "--project",
        path(&public.project),
        "change",
        "plan",
        "--input-file",
        path(&many),
    ]);
    assert!(
        errors
            .iter()
            .filter(|record| record.operation == "diagnostic")
            .any(|record| optional_field(record, "class") == Some("resource")),
        "{errors:#?}"
    );
    assert_eq!(head, std::fs::read(public.project.join("HEAD")).unwrap());
}

#[test]
fn copied_binary_charges_expanded_type_and_effect_copies_before_allocation() {
    let public = Public::new("minimal");
    let base = public.revision();
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    // Very few physical records describe an exponentially expanded type tree. The preflight
    // rejects it before cloning or recursively constructing the expanded value.
    let mut types = format!("request base={base}\ntype.map as=@T0 key=unit value=unit\n");
    for index in 1..20 {
        types.push_str(&format!(
            "type.map as=@T{index} key=@T{} value=@T{}\n",
            index - 1,
            index - 1
        ));
    }
    // Structural density must also charge every materialized effect row, including repeated
    // use of a shared alias. These invalid repeated requirements never reach semantic admission.
    let mut effects = format!("request base={base}\neffect.row as=@E\n");
    for index in 0..1_000 {
        effects.push_str(&format!(
            "effect.requirement parent=@E index={index} requirement=pkg_00000000000000000000000000000001/req_00000000000000000000000000000001\n"
        ));
    }
    effects.push_str("expression.block as=$body\n(sequence ");
    effects.push_str(&"(function-value $function (effects @E)) ".repeat(1_100));
    effects.push_str(");\nexpression.end\ncreate.module as=$module name=capacity\ncreate.function as=$function module=$module name=function visibility=private result=unit effect=pure body=$body\n");
    for (name, input) in [("expanded-types", types), ("expanded-effects", effects)] {
        let input = public.write(&format!("{name}.lkjc"), &input);
        let errors = public.failure(&[
            "--project",
            path(&public.project),
            "change",
            "plan",
            "--input-file",
            path(&input),
        ]);
        assert!(
            errors
                .iter()
                .filter(|record| record.operation == "diagnostic")
                .any(|record| {
                    optional_field(record, "class") == Some("resource")
                        && field(record, "code") == "change_input_type_capacity"
                }),
            "{name}: {errors:#?}"
        );
        assert_eq!(head, std::fs::read(public.project.join("HEAD")).unwrap());
    }
}

#[test]
fn copied_binary_block_scope_and_private_root_rejections_leave_the_complete_request_unpublished() {
    let public = Public::new("command");
    let base = public.revision();
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let prefix = format!("request base={base}\ncreate.module as=$module name=unpublished\n");
    for (name, source) in [
        (
            "self",
            "expression.block as=$body\n(let (binding x (local x)) (in (local x)))\nexpression.end\ncreate.function as=$f module=$module name=f visibility=private result=i64 effect=pure body=$body\n",
        ),
        (
            "forward",
            "expression.block as=$body\n(let (binding x (local y)) (binding y (i64 1)) (in (local x)))\nexpression.end\ncreate.function as=$f module=$module name=f visibility=private result=i64 effect=pure body=$body\n",
        ),
        (
            "cross-block",
            "expression.block as=$one\n(let (binding x (i64 1)) (in (local x)))\nexpression.end\ncreate.function as=$f module=$module name=f visibility=private result=i64 effect=pure body=$one\nexpression.block as=$two\n(local x)\nexpression.end\ncreate.function as=$g module=$module name=g visibility=private result=i64 effect=pure body=$two\n",
        ),
        (
            "wrong-function",
            "expression.block as=$one\n(sequence\n  (unit)\n  (local $other-parameter))\nexpression.end\ncreate.function as=$f module=$module name=f visibility=private result=i64 effect=pure body=$one\nexpression.block as=$two\n(local $other-parameter)\nexpression.end\ncreate.function as=$g module=$module name=g visibility=private result=i64 effect=pure body=$two\nadd.parameter as=$other-parameter function=$g name=value type=i64\n",
        ),
        (
            "extend",
            "expression.block as=$one\n(sequence (unit))\nexpression.end\nexpression.unit as=$extra\nexpression.argument parent=$one index=1 expression=$extra\ncreate.function as=$f module=$module name=f visibility=private result=unit effect=pure body=$one\n",
        ),
        (
            "shared",
            "expression.block as=$one\n(unit)\nexpression.end\ncreate.function as=$f module=$module name=f visibility=private result=unit effect=pure body=$one\ncreate.function as=$g module=$module name=g visibility=private result=unit effect=pure body=$one\n",
        ),
    ] {
        let input = public.write(&format!("{name}.lkjc"), &format!("{prefix}{source}"));
        let errors = public.failure(&[
            "--project",
            path(&public.project),
            "change",
            "plan",
            "--input-file",
            path(&input),
        ]);
        if name == "wrong-function" {
            let mismatch = errors
                .iter()
                .find(|record| {
                    record.operation == "diagnostic"
                        && optional_field(record, "code") == Some("kernel_type_parameter_scope")
                })
                .expect("exact function parameter scope validation");
            assert_eq!(field(mismatch, "path"), path(&input));
            assert_eq!(field(mismatch, "line"), "6");
            assert_eq!(field(mismatch, "column"), "3");
            assert!(!field(mismatch, "message").contains("$__block_"));
        }
        assert_eq!(
            head,
            std::fs::read(public.project.join("HEAD")).unwrap(),
            "{name}"
        );
    }

    let request = public.write("two-independent-scopes.lkjc", &format!(
        "{prefix}expression.block as=$one\n(let (binding value (i64 1)) (in (local value)))\nexpression.end\ncreate.function as=$f module=$module name=f visibility=private result=i64 effect=pure body=$one\nexpression.block as=$two\n(let (binding value (i64 2)) (in (local value)))\nexpression.end\ncreate.function as=$g module=$module name=g visibility=private result=i64 effect=pure body=$two\n"
    ));
    let plan = public.plan(&request);
    let identities = plan
        .iter()
        .filter(|record| record.operation == "identity")
        .map(|record| field(record, "id"))
        .collect::<Vec<_>>();
    assert_eq!(
        identities.len(),
        identities.iter().copied().collect::<BTreeSet<_>>().len()
    );
    public.apply(&request, field(record(&plan, "plan"), "token"));
}

#[test]
#[ignore = "requires an authenticated predecessor executable in LKJSCRIPT_PREDECESSOR_CANDIDATE"]
fn authentic_predecessor_rejects_structural_input_before_writing() {
    let predecessor = std::env::var_os("LKJSCRIPT_PREDECESSOR_CANDIDATE")
        .expect("exact authenticated predecessor executable path");
    let public = Public::with_executable(Path::new(&predecessor), "minimal");
    let request = public.write("new-syntax.lkjc", &depth_request(&public.revision(), 0, 1));
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    public.failure(&[
        "--project",
        path(&public.project),
        "change",
        "plan",
        "--input-file",
        path(&request),
    ]);
    assert_eq!(head, std::fs::read(public.project.join("HEAD")).unwrap());
}

#[test]
fn copied_binary_f64_review_parity_decimal_ingress_and_canonical_observation() {
    let public = Public::new("minimal");
    let discovery = public.success(&["capabilities", "--section", "change"]);
    assert!(
        discovery
            .iter()
            .flat_map(|record| &record.fields)
            .any(|field| field.value == "f64")
    );
    let declarations = r#"create.module as=$numeric name=numeric
type.f64 as=@Float
create.external as=$add module=$numeric name=add-floats visibility=private result=@Float implementation=core.f64.add
add.parameter as=$left function=$add name=left type=f64
add.parameter as=$right function=$add name=right type=f64
create.external as=$format module=$numeric name=format-float visibility=private result=text implementation=core.f64.to-text
add.parameter as=$formatted function=$format name=value type=f64
create.function as=$calculate module=$numeric name=calculate visibility=private result=f64 effect=pure body=$body
add.parameter as=$input function=$calculate name=input type=f64
expression.block as=$show-body
  (call $format (local $shown))
expression.end
create.function as=$show module=$numeric name=show visibility=private result=text effect=pure body=$show-body
add.parameter as=$shown function=$show name=value type=f64
expression.block as=$inf-body
  (f64 inf)
expression.end
create.function as=$infinite module=$numeric name=infinite visibility=private result=f64 effect=pure body=$inf-body
type.function as=@Calculate result=f64
type.argument parent=@Calculate index=0 type=f64
type.function as=@Show result=text
type.argument parent=@Show index=0 type=f64
type.function as=@Infinite result=f64
create.component as=$component module=$numeric name=Numbers visibility=package
add.port as=$calculate-port component=$component name=calculate type=@Calculate function=$calculate
add.port as=$show-port component=$component name=show type=@Show function=$show
add.port as=$infinite-port component=$component name=infinite type=@Infinite function=$infinite
create.target as=$calculate-target name=calculate component=$component port=$calculate-port runner=command
create.target as=$show-target name=show component=$component port=$show-port runner=command
create.target as=$infinite-target name=infinite component=$component port=$infinite-port runner=command
"#;
    let header = format!(
        "request base={} idempotency=binary64-public\n",
        public.revision()
    );
    let flat = public.write("f64-flat.lkjc", &format!("{header}{declarations}expression.local as=$read value=$input\nexpression.f64 as=$half value=5e-1\nexpression.call as=$body function=$add\nexpression.argument parent=$body index=0 expression=$read\nexpression.argument parent=$body index=1 expression=$half\n"));
    let block = public.write("f64-block.lkjc", &format!("{header}{declarations}expression.block as=$body\n(call $add (local $input) (f64 0.5))\nexpression.end\n"));
    let flat_plan = public.plan(&flat);
    let block_plan = public.plan(&block);
    let token = field(record(&flat_plan, "plan"), "token");
    assert_eq!(token, field(record(&block_plan, "plan"), "token"));
    for symbol in ["$body", "$calculate", "$input"] {
        assert_eq!(identity(&flat_plan, symbol), identity(&block_plan, symbol));
    }
    public.apply(&block, token);
    assert_eq!(public.run("calculate", "[1.5]"), json!(2.0));
    assert_eq!(public.run("show", "[-0]"), json!("-0.0"));
    assert_eq!(public.run("show", "[-0.0]"), json!("-0.0"));
    assert_eq!(
        public.run("show", "[9007199254740993]"),
        json!("9007199254740992.0")
    );
    for input in ["[1e400]", "[1.]", "[nan]", "[\"1.0\"]"] {
        public.failure(&[
            "--project",
            path(&public.project),
            "run",
            "show",
            "--arguments",
            input,
        ]);
    }
    let failure = public.failure(&["--project", path(&public.project), "run", "infinite"]);
    assert_eq!(
        field(record(&failure, "diagnostic"), "code"),
        "normalized_json_nonfinite"
    );
    public.success(&["--project", path(&public.project), "check"]);
}
