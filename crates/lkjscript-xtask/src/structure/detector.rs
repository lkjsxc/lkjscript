use std::path::Path;

use crate::model::Finding;
use crate::structure_validation::simple;

pub fn content(root: &Path, path: &str, findings: &mut Vec<Finding>) {
    let Ok(bytes) = crate::repository_support::read_bounded(&root.join(path), 4 * 1024 * 1024)
    else {
        return;
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return;
    };
    if packed_rust(path, text) {
        findings.push(simple(
            "error",
            "structure.source.packed-rust",
            path,
            "conservative packed Rust statement detection",
        ));
    }
    if path.ends_with(".json") && text.len() > 512 && text.lines().count() <= 2 {
        findings.push(simple(
            "error",
            "structure.source.minified-json",
            path,
            "conservative minified JSON detection",
        ));
    }
    if text
        .split(|value: char| {
            !value.is_ascii_alphanumeric() && value != '+' && value != '/' && value != '='
        })
        .any(|word| word.len() >= 256)
    {
        findings.push(simple(
            "error",
            "structure.source.base64",
            path,
            "conservative giant base64 literal detection",
        ));
    }
    if text
        .lines()
        .any(|line| line.len() >= 1024 && (line.contains("\\x") || line.contains("r#\"")))
    {
        findings.push(simple(
            "error",
            "structure.source.hidden-literal",
            path,
            "conservative giant hidden-source literal detection",
        ));
    }
    if path.ends_with(".rs") {
        let (items, fanout) = rust_shape(text);
        if items > 16 {
            crate::structure_validation::warning(
                findings,
                "structure.source.top-level-items",
                path,
                items,
                16,
                "conservative Rust top-level item count exceeds 16",
            );
        }
        if fanout > 16 {
            crate::structure_validation::warning(
                findings,
                "structure.source.fanout",
                path,
                fanout,
                16,
                "conservative Rust import/module fanout exceeds 16",
            );
        }
    }
}

fn rust_shape(text: &str) -> (u64, u64) {
    let mut items = 0_u64;
    let mut fanout = 0_u64;
    for line in text
        .lines()
        .filter(|line| !line.starts_with(char::is_whitespace))
    {
        let line = line
            .trim_start_matches("pub ")
            .trim_start_matches("pub(crate) ");
        if [
            "fn ", "struct ", "enum ", "trait ", "type ", "const ", "static ",
        ]
        .iter()
        .any(|prefix| line.starts_with(prefix))
        {
            items += 1;
        }
        if line.starts_with("use ") || line.starts_with("mod ") {
            fanout += 1;
        }
    }
    (items, fanout)
}

fn packed_rust(path: &str, text: &str) -> bool {
    path.ends_with(".rs")
        && text.lines().any(|line| {
            let code = without_strings(line);
            code.matches("; let ").count() >= 2 || code.matches("); ").count() >= 2
        })
}

fn without_strings(line: &str) -> String {
    let mut result = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for value in line.chars() {
        if !quoted && value == '/' && result.ends_with('/') {
            break;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if quoted && value == '\\' {
            escaped = true;
            continue;
        }
        if value == '"' {
            quoted = !quoted;
            result.push(' ');
        } else if !quoted {
            result.push(value);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn conservative_packed_statement_detection() {
        assert!(super::packed_rust("a.rs", "let a=1; let b=2; let c=3;"));
        assert!(!super::packed_rust("a.rs", "let x=\"a;b;c\";"));
        assert!(!super::packed_rust("a.rs", "a();\nb();\nc();"));
    }
    #[test]
    fn minified_boundary_shape() {
        let short = format!("{{\"x\":\"{}\"}}", "a".repeat(490));
        let long = format!("{{\"x\":\"{}\"}}", "a".repeat(510));
        assert!(short.len() <= 512 && long.len() > 512);
    }
    #[test]
    fn hidden_literal_and_base64_are_detected() {
        let root = std::env::temp_dir().join(format!("lkjscript-hidden-{}", std::process::id()));
        assert!(std::fs::create_dir_all(&root).is_ok());
        let source = format!(
            "const X: &str = r#\"{}\"#;\n{}\n",
            "\\x00".repeat(300),
            "A".repeat(256)
        );
        assert!(std::fs::write(root.join("a.rs"), source).is_ok());
        let mut findings = Vec::new();
        super::content(&root, "a.rs", &mut findings);
        assert!(findings
            .iter()
            .any(|finding| finding.rule == "structure.source.hidden-literal"));
        assert!(findings
            .iter()
            .any(|finding| finding.rule == "structure.source.base64"));
        assert!(std::fs::remove_dir_all(root).is_ok());
    }
}
