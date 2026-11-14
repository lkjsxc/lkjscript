use std::fs;
use std::path::PathBuf;

#[test]
fn interpreter_imports_only_ir_and_its_own_memory_model() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut paths = vec![root.join("interpreter.rs"), root.join("runtime_memory.rs")];
    for entry in fs::read_dir(root.join("interpreter")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            paths.push(path);
        }
    }
    for path in paths {
        let text = fs::read_to_string(&path).unwrap();
        for qualified in text.split("crate::").skip(1) {
            let module: String = qualified
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            assert!(
                matches!(module.as_str(), "ir" | "runtime_memory"),
                "{} imports shared implementation module {module}",
                path.display()
            );
        }
        assert!(
            !text.contains("super::super::"),
            "{} escapes its module boundary",
            path.display()
        );
    }
}
