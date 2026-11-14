//! Library bodies are checked on demand; user definitions and library headers are not deferred.
#[path = "support/native.rs"]
mod native;
use lkjscript::{ast, check, elaborate, interpreter, ir, resolve, source};

fn parsed(user: &str, library: &str) -> ast::Program {
    source::parse_bundle(&source::SourceBundle {
        root: "case/main.rs".to_string(),
        library: Some("dependencies/library.rs".to_string()),
        files: vec![
            source::SourceInput {
                path: "case/main.rs".to_string(),
                text: user.to_string(),
            },
            source::SourceInput {
                path: "dependencies/library.rs".to_string(),
                text: library.to_string(),
            },
        ],
    })
    .unwrap()
}
fn checked(
    user: &str,
    library: &str,
    all_library: bool,
) -> Result<ir::CheckedProgram, Vec<ast::Diagnostic>> {
    let parsed = parsed(user, library);
    let resolved = resolve::resolve(&parsed).map_err(|error| vec![error])?;
    let definitions = if all_library {
        elaborate::infer_resolved_report_all_library(&resolved)?
    } else {
        elaborate::infer_resolved_report(&resolved)?
    };
    check::check_all(elaborate::lower_definitions(definitions)?)
}
fn errors(user: &str, library: &str, all_library: bool) -> Vec<ast::Diagnostic> {
    match checked(user, library, all_library) {
        Ok(_) => panic!("invalid body was accepted"),
        Err(errors) => errors,
    }
}

#[test]
fn every_unused_user_body_is_checked() {
    let problems = errors(
        "fn bad() -> i64 { true } fn generic<T>(value:T) -> i64 { true } fn main() -> i64 { 42 }",
        "pub fn unused() -> i64 { true }",
        false,
    );
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(
        problems
            .iter()
            .all(|problem| problem.code == "E_TYPE" && problem.file == 0)
    );
}

#[test]
fn unused_library_bodies_are_checked_by_the_full_library_mode() {
    let library = "pub fn unused() -> i64 { true } pub fn generic<T>(value:T) -> i64 { true }";
    let program = checked("fn main() -> i64 { 42 }", library, false).unwrap();
    assert_eq!(program.program().functions.len(), 1);
    let problems = errors("fn main() -> i64 { 42 }", library, true);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(
        problems
            .iter()
            .all(|problem| problem.code == "E_TYPE" && problem.file == 1)
    );
}

#[test]
fn demanded_library_bodies_and_their_dependencies_are_checked() {
    let library = "pub fn entry() -> i64 { leaf() } fn leaf() -> i64 { true }";
    let problems = errors("fn main() -> i64 { std::entry() }", library, false);
    assert!(
        problems
            .iter()
            .any(|problem| problem.code == "E_TYPE" && problem.file == 1)
    );
    let generic = "pub fn entry<T>(value:T) -> i64 { true }";
    let problems = errors("fn main() -> i64 { std::entry(1i64) }", generic, false);
    assert!(
        problems
            .iter()
            .any(|problem| problem.code == "E_TYPE" && problem.file == 1)
    );
}

#[test]
fn user_and_demanded_library_errors_are_both_reported() {
    let problems = errors(
        "fn bad() -> i64 { true } fn main() -> i64 { std::bad() }",
        "pub fn bad() -> i64 { true }",
        false,
    );
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems.iter().all(|problem| problem.code == "E_TYPE"));
    assert_ne!(problems[0].file, problems[1].file);
}

#[test]
fn demanded_generic_methods_keep_definition_site_trait_selection() {
    let library = "pub trait Read { fn read(&self) -> i64; } pub fn choose<T:Read>(value:T) -> i64 { let call=|| value.read(); call() } pub fn unused() -> i64 { 9 }";
    let user = "struct Token; impl Token { fn read(&self) -> i64 { 7 } } impl std::Read for Token { fn read(&self) -> i64 { 42 } } fn main() -> i64 { std::choose(Token) }";
    let checked = checked(user, library, false).unwrap();
    let program = checked.program();
    assert!(
        !program
            .functions
            .iter()
            .any(|function| function.name.ends_with("unused"))
    );
    let entry = program
        .functions
        .iter()
        .position(|function| function.name == "main")
        .unwrap();
    let result = interpreter::run(program, entry, &[], &mut Vec::new())
        .unwrap_or_else(|error| panic!("{}: {}", error.code(), error.message));
    match result.value {
        interpreter::Scalar::Int { value, .. } => assert_eq!(value, 42),
        _ => panic!("expected integer result"),
    }
    native::assert_values(&checked, &[("main", 42)]);
}

#[test]
fn unused_library_destructors_are_deferred_with_other_bodies() {
    let library =
        "pub struct Unused; impl Drop for Unused { fn drop(&mut self) { let broken:i64=true; } }";
    assert!(checked("fn main() -> i64 { 42 }", library, false).is_ok());
    assert!(
        errors("fn main() -> i64 { 42 }", library, true)
            .iter()
            .any(|problem| problem.code == "E_TYPE")
    );
    assert!(
        errors(
            "fn main() -> i64 { let value=std::Unused; 42 }",
            library,
            false
        )
        .iter()
        .any(|problem| problem.code == "E_TYPE")
    );
}

#[test]
fn unused_library_layouts_still_obey_the_header_rules() {
    assert!(
        checked(
            "fn main() -> i64 { 42 }",
            "pub struct Bad { values:Vec<&'static i64> }",
            false
        )
        .is_err()
    );
}
