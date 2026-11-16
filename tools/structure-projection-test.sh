#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
mkdir -p target/gate
fixture=$(mktemp -d "$PWD/target/compiler-module-projection.XXXXXX")
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
cp -a src "$fixture/src"
cat > "$fixture/src/compiler_projection_cases.lkj" <<'LKJSCRIPT_CASES'
//! Pure external-module provenance and projection planner cases.
mod ast; mod check; mod diagnostic; mod diagnostics; mod elaborate; mod facts;
mod frontend; mod generics; mod interpreter; mod ir; mod ir_binary; mod ir_text;
mod json; mod names; mod optimizer; mod resolve; mod reuse; mod runtime_memory;
mod native; mod x86; mod child_image; mod entry_grants; mod runtime;
mod runtime_diagnostics; mod semantic; mod source; mod syntax;
mod structure_edit; mod structure_refactor; mod structure_replay;
pub use std::sha256;
pub fn main(mut world: World) { match cases::run() { Ok(()) => { let mut output = world.stdout(); output.write(b"ok module projection: loader provenance, strict declaration/path checks and scoped replay\n"); }, Err(error) => panic!("{error}") } }
mod cases {
    use super::*;
    struct Snapshot { catalog: semantic::Catalog, inputs: Vec<(String, Vec<u8>)>, checked: ir::CheckedProgram }
    fn file(path: &str, text: &str) -> (String, Vec<u8>) { (path.to_string(), text.as_bytes().to_vec()) }
    fn snapshot(root: &str, library: Option<&str>, inputs: Vec<(String, Vec<u8>)>) -> Result<Snapshot, String> {
        let parsed = source::load(root, library, &mut |path: &str| -> Result<Option<Vec<u8>>, String> {
            for (name, bytes) in &inputs { if name == path { return Ok(Some(bytes.clone())); } }
            Ok(None)
        }).map_err(|problem| -> String { format!("{}: {}", problem.code, problem.message) })?;
        let resolved = resolve::resolve(&parsed).map_err(|problem| problem.message)?;
        let definitions = elaborate::infer_resolved_all(&resolved).map_err(|problems| -> String { problems[0usize].message.clone() })?;
        let raw = elaborate::lower_definitions_recorded_semantic(definitions, &mut None, &mut None)
            .map_err(|problems| -> String { problems[0usize].message.clone() })?;
        let checked = check::check(raw).map_err(|problem| problem.message)?;
        let catalog = semantic::catalog_items(&resolved, &inputs, &checked)?;
        Ok(Snapshot { catalog, inputs, checked })
    }
    fn mounted<'a>(snapshot: &'a Snapshot, identity: &str) -> &'a semantic::ModuleSource {
        for source in &snapshot.catalog.module_sources { if source.identity == identity { return source; } }
        panic!("missing mounted module {identity}")
    }
    fn plan(snapshot: &Snapshot, operations: &str) -> Result<structure_edit::Draft, String> {
        let request = structure_edit::parse(format!("{{\"version\":1,\"operations\":[{operations}]}}").as_bytes())?;
        structure_edit::plan(&snapshot.catalog, &snapshot.inputs, &request)
    }
    fn operation(parent: &str, declaration: &str, path: &str, source: &str) -> String {
        format!("{{\"op\":\"add-item\",\"id\":{},\"source\":{},\"projection\":{{\"path\":{},\"source\":{}}}}}",
            semantic::quote(parent), semantic::quote(declaration), semantic::quote(path), semantic::quote(source))
    }
    fn refusal(before: &Snapshot, operation: &str, rule: &str) -> Result<(), String> {
        match plan(before, operation) {
            Err(error) if error.contains(rule) => Ok(()),
            Err(error) => Err(format!("expected {rule}, got {error}")),
            Ok(_) => Err(format!("expected {rule}: {operation}")),
        }
    }
    fn apply(before: &Snapshot, draft: &structure_edit::Draft, scope: &str) -> Vec<(String, Vec<u8>)> {
        let mut inputs = before.inputs.clone();
        for change in &draft.changes {
            let mut found = false;
            for index in 0usize..inputs.len() {
                if inputs[index].0 == change.file {
                    match &change.before {
                        structure_edit::PreviousText::Present(old) => assert_eq!(inputs[index].1, *old),
                        structure_edit::PreviousText::Absent => panic!("pure planner must not create an unchecked file"),
                    }
                    inputs[index].1 = change.after.clone(); found = true;
                }
            }
            assert!(found);
        }
        for projected in &draft.projections {
            let path = if scope == "." { projected.path.clone() } else { format!("{scope}/{}", projected.path) };
            for (existing, _) in &inputs { assert!(*existing != path); }
            inputs.push((path, projected.source.clone()));
        }
        inputs
    }
    fn value(snapshot: &Snapshot) -> i64 {
        let mut entry = None;
        for (index, function) in snapshot.checked.program().functions.iter().enumerate() {
            if function.name == "main" { entry = Some(index); }
        }
        let args: Vec<u64> = Vec::new(); let mut output: Vec<u8> = Vec::new();
        let result = interpreter::run(snapshot.checked.program(), entry.unwrap(), &args, &mut output).unwrap();
        assert!(output.is_empty());
        match result.value { interpreter::Scalar::Int { value, .. } => value as i64, _ => panic!("integer fixture") }
    }
    fn provenance() -> Result<(), String> {
        let direct = snapshot("main.lkj", None, vec![file("main.lkj", "mod empty; fn main() -> i64 { 1i64 }"), file("empty.lkj", "")])?;
        assert_eq!(direct.catalog.module_sources.len(), 2usize);
        assert_eq!(mounted(&direct, "crate").parent, ""); assert_eq!(mounted(&direct, "crate").alternate, "");
        assert_eq!(mounted(&direct, "empty").file, "empty.lkj");
        assert_eq!(mounted(&direct, "empty").alternate, "empty/mod.lkj"); assert_eq!(mounted(&direct, "empty").parent, "crate");
        let draft = plan(&direct, r#"{"op":"add-item","id":"empty","source":"pub fn value() -> i64 { 3i64 }"}"#)?;
        assert_eq!(draft.changes.len(), 1usize); assert!(draft.projections.is_empty());
        assert_eq!(draft.changes[0usize].file, "empty.lkj");
        match &draft.changes[0usize].before { structure_edit::PreviousText::Present(bytes) => assert!(bytes.is_empty()), _ => panic!("empty is present") }
        snapshot("main.lkj", None, apply(&direct, &draft, "."))?;
        let nested = snapshot("src/main.lkj", None, vec![file("src/main.lkj", "mod empty; fn main() -> i64 { 2i64 }"), file("src/empty/mod.lkj", "")])?;
        assert_eq!(mounted(&nested, "empty").file, "src/empty/mod.lkj"); assert_eq!(mounted(&nested, "empty").alternate, "src/empty.lkj");
        let include = snapshot("main.lkj", None, vec![file("main.lkj", "fn main() -> i64 { include_bytes!(\"data.bin\")[0] as i64 }"), file("data.bin", "x")])?;
        assert_eq!(include.catalog.module_sources.len(), 1usize); assert_eq!(value(&include), 120i64);
        let standard = snapshot("main.lkj", Some("library/lib.lkj"), vec![file("main.lkj", "fn main() -> i64 { 4i64 }"), file("library/lib.lkj", "")])?;
        assert_eq!(mounted(&standard, "std").file, "library/lib.lkj"); assert_eq!(mounted(&standard, "std").parent, "");
        let draft = plan(&standard, &operation("std", "pub mod added;", "library/added.lkj", ""))?;
        assert_eq!(draft.changes[0usize].file, "library/lib.lkj"); assert_eq!(draft.projections[0usize].identity, "std::added");
        let candidate = snapshot("main.lkj", Some("library/lib.lkj"), apply(&standard, &draft, "."))?;
        assert_eq!(mounted(&candidate, "std::added").parent, "std");
        let both = snapshot("main.lkj", None, vec![file("main.lkj", "mod ambiguous;"), file("ambiguous.lkj", ""), file("ambiguous/mod.lkj", "")]);
        match both { Err(error) => assert!(error.contains("E_MODULE_AMBIGUOUS")), Ok(_) => panic!("both candidates admitted") }
        Ok(())
    }
    fn planner() -> Result<(), String> {
        let before = snapshot("main.lkj", None, vec![file("main.lkj", "fn main() -> i64 { 42i64 }")])?;
        let draft = plan(&before, &operation("crate", "pub mod added;", "added.lkj", ""))?;
        assert_eq!(draft.changes.len(), 1usize); assert_eq!(draft.projections.len(), 1usize);
        assert_eq!(draft.projections[0usize].parent, "crate"); assert_eq!(draft.projections[0usize].identity, "added"); assert!(draft.projections[0usize].source.is_empty());
        let candidate = snapshot("main.lkj", None, apply(&before, &draft, "."))?;
        assert_eq!(value(&candidate), 42i64); assert_eq!(mounted(&candidate, "added").file, "added.lkj");
        let inline = snapshot("main.lkj", None, vec![file("main.lkj", "mod inner {} fn main() -> i64 { 42i64 }")])?;
        let draft = plan(&inline, &operation("inner", "pub mod added;", "inner/added.lkj", "pub fn value() -> i64 { 9i64 }"))?;
        let candidate = snapshot("main.lkj", None, apply(&inline, &draft, "."))?;
        assert_eq!(mounted(&candidate, "inner::added").parent, "inner");
        refusal(&before, &operation("crate", "mod added {}", "added.lkj", ""), "E_EDIT_PROJECTION")?;
        refusal(&before, &operation("crate", "fn added() {}", "added.lkj", ""), "E_EDIT_PROJECTION")?;
        refusal(&before, &operation("crate", "mod added; fn extra() {}", "added.lkj", ""), "E_EDIT_PROJECTION")?;
        refusal(&before, &operation("crate", "#[cfg(test)] fn hidden() {} mod added;", "added.lkj", ""), "E_EDIT_PROJECTION")?;
        refusal(&before, &operation("crate", "mod added; #[cfg(test)] fn hidden() {}", "added.lkj", ""), "E_EDIT_PROJECTION")?;
        refusal(&before, &operation("crate", "#[cfg(test)] mod added;", "added.lkj", ""), "E_EDIT_PROJECTION")?;
        refusal(&before, &operation("crate", "mod added;", "/added.lkj", ""), "E_EDIT_PROJECTION")?;
        refusal(&before, &operation("crate", "mod added;", "../added.lkj", ""), "E_EDIT_PROJECTION")?;
        refusal(&before, &operation("crate", "mod added;", "x/../added.lkj", ""), "E_EDIT_PROJECTION")?;
        refusal(&before, &operation("crate", "mod added;", "./added.lkj", ""), "E_EDIT_PROJECTION")?;
        refusal(&before, &operation("crate", "mod added;", "x//added.lkj", ""), "E_EDIT_PROJECTION")?;
        refusal(&before, &operation("crate", "mod added;", "added.rs", ""), "E_EDIT_PROJECTION")?;
        refusal(&before, &operation("crate", "mod added;", "", ""), "E_EDIT_PROJECTION")?;
        refusal(&before, r#"{"op":"replace-body","id":"main","source":"{ 1i64 }","projection":{"path":"added.lkj","source":""}}"#, "E_EDIT_REQUEST")?;
        refusal(&before, r#"{"op":"add-item","id":"crate","source":"mod added;","projection":{"path":"added.lkj","source":"","extra":0}}"#, "E_EDIT_PROJECTION")?;
        refusal(&before, r#"{"op":"add-item","id":"crate","source":"mod added;","projection":{"path":"added.lkj"}}"#, "E_EDIT_REQUEST")?;
        refusal(&before, &format!("{},{}", operation("crate", "mod added;", "added.lkj", ""), operation("crate", "mod other;", "added.lkj", "")), "E_EDIT_PROJECTION")?;
        refusal(&before, &format!("{},{}", operation("crate", "mod added;", "added.lkj", ""), operation("crate", "mod added;", "different.lkj", "")), "E_EDIT_PROJECTION")?;
        refusal(&candidate, &operation("inner", "mod added;", "inner/added.lkj", ""), "E_EDIT_NAME")?;
        refusal(&before, &operation("missing", "mod added;", "missing/added.lkj", ""), "E_EDIT_IDENTITY")?;
        Ok(())
    }
    fn replay_failure(before: &Snapshot, after: &Snapshot, scope: &str, rule: &str) -> Result<(), String> {
        match structure_replay::request_with_scope(&before.catalog, &before.inputs, &after.catalog, &after.inputs, scope) {
            Err(error) if error.contains(rule) => Ok(()),
            Err(error) => Err(format!("expected {rule}, got {error}")),
            Ok(_) => Err(format!("expected replay refusal {rule}")),
        }
    }
    fn replay_draft(before: &Snapshot, after: &Snapshot, scope: &str) -> Result<structure_edit::Draft, String> {
        let request = structure_replay::request_with_scope(&before.catalog, &before.inputs, &after.catalog, &after.inputs, scope)?;
        structure_edit::plan(&before.catalog, &before.inputs, &request)
    }
    fn replay() -> Result<(), String> {
        let before = snapshot("old/main.lkj", None, vec![file("old/main.lkj", "fn main() -> i64 { 1i64 }")])?;
        let after = snapshot("new/main.lkj", None, vec![file("new/main.lkj", "mod added; fn main() -> i64 { added::value() }"), file("new/added.lkj", "pub fn value() -> i64 { 42i64 }")])?;
        replay_failure(&before, &after, "", "E_REPLAY_FILES")?;
        let draft = replay_draft(&before, &after, "old")?;
        assert_eq!(draft.projections.len(), 1usize); assert_eq!(draft.projections[0usize].path, "added.lkj");
        let candidate = snapshot("old/main.lkj", None, apply(&before, &draft, "old"))?;
        assert_eq!(value(&candidate), value(&after)); assert_eq!(mounted(&candidate, "added").file, "old/added.lkj");
        let empty = snapshot("new/main.lkj", None, vec![file("new/main.lkj", "mod empty; fn main() -> i64 { 1i64 }"), file("new/empty.lkj", "")])?;
        let draft = replay_draft(&before, &empty, "old")?; assert!(draft.projections[0usize].source.is_empty());
        snapshot("old/main.lkj", None, apply(&before, &draft, "old"))?;
        let nested = snapshot("new/main.lkj", None, vec![file("new/main.lkj", "mod nested; fn main() -> i64 { 1i64 }"), file("new/nested/mod.lkj", "")])?;
        let draft = replay_draft(&before, &nested, "old")?; assert_eq!(draft.projections[0usize].path, "nested/mod.lkj");
        let child_before = snapshot("old/main.lkj", None, vec![file("old/main.lkj", "mod parent; fn main() -> i64 { 1i64 }"), file("old/parent.lkj", "")])?;
        let child_after = snapshot("new/main.lkj", None, vec![file("new/main.lkj", "mod parent; fn main() -> i64 { 1i64 }"), file("new/parent.lkj", "pub mod child;"), file("new/parent/child.lkj", "")])?;
        let draft = replay_draft(&child_before, &child_after, "old")?; assert_eq!(draft.projections[0usize].path, "parent/child.lkj");
        snapshot("old/main.lkj", None, apply(&child_before, &draft, "old"))?;
        let body_before = snapshot("old/main.lkj", None, vec![file("old/main.lkj", "mod empty; fn main() -> i64 { 1i64 }"), file("old/empty.lkj", "")])?;
        let body_after = snapshot("new/main.lkj", None, vec![file("new/main.lkj", "mod empty; fn main() -> i64 { empty::value() }"), file("new/empty.lkj", "pub fn value() -> i64 { 9i64 }")])?;
        let draft = replay_draft(&body_before, &body_after, "")?; assert!(draft.projections.is_empty());
        assert_eq!(value(&snapshot("old/main.lkj", None, apply(&body_before, &draft, "old"))?), 9i64);
        let include_after = snapshot("new/main.lkj", None, vec![file("new/main.lkj", "mod added; fn main() -> i64 { added::value() }"), file("new/added.lkj", "pub fn value() -> i64 { include_bytes!(\"data.bin\")[0] as i64 }"), file("new/data.bin", "x")])?;
        replay_failure(&before, &include_after, "old", "E_REPLAY_FILES")?;
        let framing = snapshot("new/main.lkj", None, vec![file("new/main.lkj", "use added::value; mod added; fn main() -> i64 { value() }"), file("new/added.lkj", "pub fn value() -> i64 { 42i64 }")])?;
        replay_failure(&before, &framing, "old", "E_REPLAY_FRAME")?;
        let included_module = snapshot("old/main.lkj", None, vec![file("old/main.lkj", "fn main() -> i64 { include_bytes!(\"added.lkj\").len() as i64 }"), file("old/added.lkj", "pub fn value() -> i64 { 42i64 }")])?;
        replay_failure(&included_module, &after, "old", "E_REPLAY_MODULE_EXISTING")?;
        let inline_before = snapshot("old/main.lkj", None, vec![file("old/main.lkj", "mod outer {} fn main() -> i64 { 1i64 }")])?;
        let inline_after = snapshot("new/main.lkj", None, vec![file("new/main.lkj", "mod outer { pub mod added; } fn main() -> i64 { 1i64 }"), file("new/outer/added.lkj", "")])?;
        replay_failure(&inline_before, &inline_after, "old", "E_REPLAY_MODULE_PROJECTION")?;
        let parents_after = snapshot("new/main.lkj", None, vec![file("new/main.lkj", "mod added; fn main() -> i64 { 1i64 }"), file("new/added.lkj", "pub mod child;"), file("new/added/child.lkj", "")])?;
        replay_failure(&before, &parents_after, "old", "E_REPLAY_MODULE_PARENT")?;
        replay_failure(&before, &after, "elsewhere", "E_REPLAY_MODULE_SCOPE")?;
        let changed_before = snapshot("old/main.lkj", None, vec![file("old/main.lkj", "fn main() -> i64 { include_bytes!(\"data.bin\")[0] as i64 }"), file("old/data.bin", "x")])?;
        let changed_after = snapshot("new/main.lkj", None, vec![file("new/main.lkj", "fn main() -> i64 { include_bytes!(\"data.bin\")[0] as i64 }"), file("new/data.bin", "y")])?;
        replay_failure(&changed_before, &changed_after, "old", "E_REPLAY_DATA")?;
        let library_before = snapshot("old/main.lkj", Some("old/library/lib.lkj"), vec![file("old/main.lkj", "fn main() -> i64 { 1i64 }"), file("old/library/lib.lkj", "")])?;
        let library_after = snapshot("new/main.lkj", Some("new/library/lib.lkj"), vec![file("new/main.lkj", "fn main() -> i64 { 1i64 }"), file("new/library/lib.lkj", "pub mod added;"), file("new/library/added.lkj", "")])?;
        let draft = replay_draft(&library_before, &library_after, "old")?;
        assert_eq!(draft.projections[0usize].identity, "std::added"); assert_eq!(draft.projections[0usize].path, "library/added.lkj");
        snapshot("old/main.lkj", Some("old/library/lib.lkj"), apply(&library_before, &draft, "old"))?;
        Ok(())
    }
    pub fn run() -> Result<(), String> { provenance()?; planner()?; replay()?; Ok(()) }
}

LKJSCRIPT_CASES
"$compiler" build "$fixture/src/compiler_projection_cases.lkj" --library-dir "$PWD/library" --no-store -o "$fixture/cases"
"$fixture/cases"
