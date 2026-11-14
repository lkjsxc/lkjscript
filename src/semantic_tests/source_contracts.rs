//! Source loading is an authority boundary even when the reader is a test closure.
use crate::ast::Diagnostic;
use crate::ir;
use crate::{embedded, frontend, source};

fn compile(root: &str, files: &[(String, Vec<u8>)]) -> Result<ir::CheckedProgram, Diagnostic> {
    let parsed = source::load(root, Some("$std/lib.rs"), &mut |path: &str| -> Result<
        Option<Vec<u8>>,
        String,
    > {
        for (name, bytes) in files {
            if name.as_str() == path {
                return Ok(Some(bytes.clone()));
            }
        }
        Ok(embedded::library(path))
    })?;
    frontend::check_source(&parsed)
}
fn embedded_bytes() {
    let files=vec![
        ("src/main.rs".to_string(),"mod child;\nfn data()->&'static [u8;4]{include_bytes!(\"../assets/data.bin\")}\npub fn probe()->i64 {let bytes:[u8;4]=*data();bytes[0] as i64+bytes[1] as i64+bytes[2] as i64+bytes[3] as i64+include_str!(\"../assets/message.txt\").len() as i64+child::length()}".as_bytes().to_vec()),
        ("src/child.rs".to_string(),"pub fn length()->i64{include_str!(\"../assets/child.txt\",).len() as i64}".as_bytes().to_vec()),
        ("assets/data.bin".to_string(),vec![255u8,0u8,10u8,27u8]),
        ("assets/message.txt".to_string(),"π\n".as_bytes().to_vec()),
        ("assets/child.txt".to_string(),"four".as_bytes().to_vec()),
    ];
    let checked = compile("src/main.rs", &files).unwrap();
    let mut entry = 0usize;
    while checked.program().functions[entry].name != "probe" {
        entry += 1;
    }
    let value =
        crate::interpreter::run(checked.program(), entry, &[0u64; 0], &mut Vec::<u8>::new())
            .unwrap();
    assert!(matches!(
        value.value,
        crate::interpreter::Scalar::Int { value: 299, .. }
    ));
}
fn call_site_diagnostics() {
    for (name, bytes, code) in vec![
        (
            "include_str".to_string(),
            Some(vec![255u8]),
            "E_INCLUDE_UTF8".to_string(),
        ),
        (
            "include_str".to_string(),
            None,
            "E_INCLUDE_MISSING".to_string(),
        ),
        (
            "include_bytes".to_string(),
            None,
            "E_INCLUDE_MISSING".to_string(),
        ),
    ] {
        let text: String = format!("fn probe() {{\n  {name}!(\"asset\");\n}}");
        let mut files = vec![("main.rs".to_string(), text.into_bytes())];
        if let Some(bytes) = bytes {
            files.push(("asset".to_string(), bytes));
        }
        let error = compile("main.rs", &files).err().unwrap();
        assert!(
            error.code == code && error.path == "main.rs" && error.line == 2 && error.column == 3
        );
    }
}
fn confined_read(root: &str, requested: &str) {
    let text: String = format!("fn probe() {{ include_bytes!(\"{requested}\"); }}");
    let mut reads: Vec<String> = Vec::new();
    let report = source::load_report(
        root,
        None,
        &mut |path: &str| -> Result<Option<Vec<u8>>, String> {
            reads.push(path.to_string());
            assert!(path == root);
            Ok(Some(text.as_bytes().to_vec()))
        },
    );
    assert_eq!(report.diagnostics[0].code, "E_INCLUDE_PATH");
    assert!(reads.len() == 1 && reads[0] == root);
}
fn confined_reads() {
    confined_read("src/main.rs", "../../secret");
    confined_read("main.rs", "/etc/passwd");
    confined_read("$std/root.rs", "../secret");
}
fn literal_path(expression: &str) {
    let text: String = format!("fn probe() {{let path=\"a\";{expression};}}");
    let report = source::load_report("main.rs", None, &mut |path: &str| -> Result<
        Option<Vec<u8>>,
        String,
    > {
        assert_eq!(path, "main.rs");
        Ok(Some(text.as_bytes().to_vec()))
    });
    assert_eq!(report.diagnostics[0].code, "E_INCLUDE_PATH");
}
fn literal_paths_and_disabled_items() {
    literal_path("include_str!()");
    literal_path("include_bytes!(path)");
    literal_path("include_bytes!(\"a\", \"b\")");
    literal_path("include_str!(\"a\"; 1)");
    let report = source::load_report("main.rs", None, &mut |path: &str| -> Result<
        Option<Vec<u8>>,
        String,
    > {
        assert_eq!(path, "main.rs");
        Ok(Some(
            "#[cfg(test)] fn ignored(){include_bytes!(\"absent\");} fn probe()->i64{1}"
                .as_bytes()
                .to_vec(),
        ))
    });
    assert!(report.diagnostics.is_empty());
}
fn io_errors() {
    let report = source::load_report("main.rs", None, &mut |path: &str| -> Result<
        Option<Vec<u8>>,
        String,
    > {
        if path == "main.rs" {
            Ok(Some(
                "fn probe(){include_bytes!(\"blocked\");}"
                    .as_bytes()
                    .to_vec(),
            ))
        } else {
            Err("permission denied".to_string())
        }
    });
    assert_eq!(report.diagnostics[0].code, "E_IO");
    assert!(report.diagnostics[0].message.contains("permission denied"));
}
fn absolute_root(root: &str, child: &str, asset: &str, requested: &str) {
    let text: String = format!(
        "mod child;fn probe()->i64{{child::value()+include_bytes!(\"{requested}\")[0] as i64}}"
    );
    let mut reads: Vec<String> = Vec::new();
    let program = source::load(
        root,
        None,
        &mut |path: &str| -> Result<Option<Vec<u8>>, String> {
            reads.push(path.to_string());
            Ok(if path == root {
                Some(text.as_bytes().to_vec())
            } else if path == child {
                Some("pub fn value()->i64{1}".as_bytes().to_vec())
            } else if path == asset {
                Some(vec![41u8])
            } else {
                None
            })
        },
    )
    .unwrap();
    assert!(program.files.iter().any(|file| file.path == child));
    assert!(reads.iter().any(|path| path.as_str() == asset));
    for path in &reads {
        assert!(path.starts_with('/') && !path.starts_with("//"));
    }
    frontend::check_source(&program).unwrap();
}
fn absolute_roots() {
    absolute_root("/main.rs", "/child.rs", "/asset", "asset");
    absolute_root("/nested/main.rs", "/nested/child.rs", "/asset", "../asset");
}
fn module_locations() {
    for (text, code) in vec![
        (
            "pub fn probe()->i64{true}".to_string(),
            "E_TYPE".to_string(),
        ),
        (
            "pub fn probe()->i64{let value=Box::new(42);let moved=value;*value}".to_string(),
            "E_MOVE".to_string(),
        ),
    ] {
        let files = vec![
            (
                "main.rs".to_string(),
                "mod child;fn probe()->i64{child::probe()}"
                    .as_bytes()
                    .to_vec(),
            ),
            ("child.rs".to_string(), text.into_bytes()),
        ];
        let error = compile("main.rs", &files).err().unwrap();
        assert!(
            error.code == code
                && error.path == "child.rs"
                && error.file != 0usize
                && error.line == 1usize
        );
    }
}
pub fn run(world: &mut World, trace: bool) -> usize {
    if trace {
        world.stdout().write(b"source loading contracts\n");
    }
    embedded_bytes();
    call_site_diagnostics();
    confined_reads();
    literal_paths_and_disabled_items();
    io_errors();
    absolute_roots();
    module_locations();
    7usize
}
