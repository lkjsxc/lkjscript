//! The production canonical-name helper executes with the owned UTF-8 library.
#[path = "../bridge/library.rs"]
mod bootstrap_library;
#[path = "support/native.rs"]
mod native_test;

use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{compiler, names};

const NAMES: &str = include_str!("../src/names.rs");

fn bundle(module: &str, cases: &[(String, String)]) -> SourceBundle {
    let mut root = "mod names; use names::qualified as join;\n".to_string();
    for (index, (parent, name)) in cases.iter().enumerate() {
        let expected = names::qualified(parent, name);
        root.push_str(&format!(
            "pub fn compare_{index}()->usize {{\n\
             let value=join({parent:?},{name:?});\n\
             let actual=value.as_bytes();\n\
             let expected={expected:?}.as_bytes();\n\
             if actual.len()!=expected.len(){{return 1;}}\n\
             let mut index:usize=0;\n\
             while index<actual.len(){{\n\
                 if actual[index]!=expected[index]{{return index+2;}}\n\
                 index=index+1;\n\
             }}\n\
             0\n\
             }}\n"
        ));
    }
    let mut files = bootstrap_library::standard_library();
    files.push(SourceInput {
        path: "text_component.rs".to_string(),
        text: root,
    });
    files.push(SourceInput {
        path: "names.rs".to_string(),
        text: module.to_string(),
    });
    SourceBundle {
        root: "text_component.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}

fn run(checked: &lkjscript::k::CheckedProgram, expected: &[usize]) {
    let names: Vec<_> = (0..expected.len())
        .map(|index| format!("compare_{index}"))
        .collect();
    let values: Vec<_> = names
        .iter()
        .zip(expected)
        .map(|(name, value)| (name.as_str(), *value as i64))
        .collect();
    native_test::assert_values(checked, &values);
}

#[test]
fn production_qualified_names_match_rust_over_every_output_byte() {
    let ascii: String = (0u8..=127).map(char::from).collect();
    let cases = vec![
        (String::new(), String::new()),
        (String::new(), "root".to_string()),
        ("outer::inner".to_string(), "leaf".to_string()),
        ("α::水".to_string(), "😀\0leaf\u{10ffff}".to_string()),
        (ascii.clone(), ascii),
        ("module_segment::".repeat(256), "水😀".repeat(256)),
    ];
    let artifact = compiler::check_bundle(&bundle(NAMES, &cases)).unwrap();
    run(&artifact, &vec![0; cases.len()]);
}

#[test]
fn changing_the_production_join_changes_generated_behavior() {
    let cases = vec![
        (String::new(), "leaf".to_string()),
        ("module".to_string(), "leaf".to_string()),
    ];
    let original = compiler::check_bundle(&bundle(NAMES, &cases)).unwrap();
    let changed = NAMES.replace("push_str(\"::\")", "push_str(\":\")");
    assert_ne!(NAMES, changed);
    let mutated = compiler::check_bundle(&bundle(&changed, &cases)).unwrap();
    assert_ne!(
        lkjscript::k_text::write(original.program()),
        lkjscript::k_text::write(mutated.program())
    );
    run(&original, &[0, 0]);
    run(&mutated, &[0, 1]);
}

#[test]
fn invalid_production_join_mutation_is_checked() {
    let changed = NAMES.replace("result.push_str(name)", "result.push_str(1i64)");
    assert_ne!(NAMES, changed);
    let cases = vec![("module".to_string(), "leaf".to_string())];
    let error = compiler::check_bundle(&bundle(&changed, &cases))
        .err()
        .unwrap();
    assert_eq!(error.code, "E_TYPE", "{error:?}");
    assert_eq!(error.path, "names.rs");
}
