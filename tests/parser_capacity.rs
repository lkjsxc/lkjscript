use lkjscript::syntax;
use std::process::Command;

// A stack overflow aborts the process, so isolate the embedding probe from the
// Cargo harness. The 2 MiB stack is a test environment, not a language limit.
#[test]
fn nested_types_parse_and_drop_on_small_native_stack() {
    const CHILD: &str = "LKJSCRIPT_TYPE_PARSER_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "nested_types_parse_and_drop_on_small_native_stack",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "small-stack parser subprocess failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        return;
    }
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for depth in [512, 8192] {
                for (opening, closing, nodes_per_level) in [
                    ("Box<", ">", 1),
                    ("[", "; 1]", 1),
                    ("(", ",)", 1),
                    ("fn() -> ", "", 1),
                    ("FnOnce() -> ", "", 2),
                    ("&", "", 1),
                    ("(", ")", 0),
                    ("Box<(fn() -> [", "; 1],)>", 4),
                ] {
                    let source = format!(
                        "type Nested = {}i64{};",
                        opening.repeat(depth),
                        closing.repeat(depth),
                    );
                    let parsed = syntax::parse(&source).unwrap();
                    assert_eq!(parsed.aliases.len(), 1);
                    assert_eq!(parsed.types.len(), 3 + depth * nodes_per_level);
                    drop(parsed);
                }
                let source = format!("type Broken = {}i64;", "Box<".repeat(depth));
                let error = syntax::parse(&source).unwrap_err();
                assert_eq!(error.code, "E_PARSE");
                assert!(error.message.contains("expected `>`"), "{error:?}");
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
