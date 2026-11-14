#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::{bootstrap_runtime, check, compiler, ir, ir_text, native, source};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

static EXECUTION: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn source_case(name: &str, text: &str, entry: &str, expected: u64, stdout: &[u8]) {
    let mut files = bootstrap_library::standard_library();
    files.push(source::SourceInput {
        path: name.to_string(),
        text: text.to_string(),
    });
    let checked = compiler::check_bundle(&source::SourceBundle {
        root: name.to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    })
    .unwrap_or_else(|error| {
        panic!(
            "{}:{} {} {}",
            error.path, error.line, error.code, error.message
        )
    });
    checked_case(name, &checked, entry, expected, stdout);
}
fn checked_case(
    name: &str,
    checked: &ir::CheckedProgram,
    entry: &str,
    expected: u64,
    stdout: &[u8],
) {
    let _execution = EXECUTION.lock().unwrap();
    let function = checked
        .program()
        .functions
        .iter()
        .position(|function| function.name == entry)
        .unwrap();
    let image = native::encode(
        checked,
        bootstrap_runtime::get().unwrap(),
        native::Entry {
            function,
            arguments: if entry == "main" { vec![1] } else { vec![] },
            result_fd: Some(2),
        },
    )
    .unwrap_or_else(|error| {
        panic!(
            "{}:{} {} {}",
            error.file, error.line, error.code, error.message
        )
    });
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/native-native");
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{}-{name}.elf", std::process::id()));
    std::fs::write(&path, image.bytes).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(&path).output().unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(
        output.stderr.len(),
        24,
        "{name}: process terminated without result: {output:?}"
    );
    let words: Vec<u64> = output
        .stderr
        .chunks_exact(8)
        .map(|word| u64::from_le_bytes(word.try_into().unwrap()))
        .collect();
    assert_eq!(words[0], 0, "{name}: trap packet {words:?}");
    assert_eq!(words[1], expected, "{name}: scalar result");
    assert!(output.status.success(), "{name}: {output:?}");
    assert_eq!(output.stdout, stdout, "{name}: output bytes");
}

#[test]
fn scalar_calls_loops_and_inline_values() {
    source_case(
        "scalar",
        "fn sum(a:i64,b:i64,c:i64,d:i64,e:i64,f:i64,g:i64,h:i64)->i64{a+b+c+d+e+f+g+h} pub fn probe()->i64{let mut x=0i64;let mut i=0i64;while i<6 {x=x+i;i=i+1;}sum(x,1,2,3,4,5,6,6)}",
        "probe",
        42,
        &[],
    );
    source_case(
        "inline",
        "#[derive(Clone,Copy)] struct Pair{a:i64,b:i64} fn swap(p:Pair)->Pair{Pair{a:p.b,b:p.a}} pub fn probe()->i64{let mut p=Pair{a:20,b:22};p=swap(p);let values=[p,p];values[0].a+values[1].b}",
        "probe",
        42,
        &[],
    );
}
#[test]
fn owned_collections_and_partial_moves() {
    source_case(
        "partial",
        include_str!("conformance/pass/partial_move_reinitialize.rs"),
        "probe",
        42,
        &[],
    );
    source_case(
        "recursive",
        include_str!("conformance/pass/recursive_box.rs"),
        "probe",
        42,
        &[],
    );
    source_case(
        "vector",
        "struct Pair{a:Box<i64>,b:i64} pub fn probe()->i64{let mut values=Vec::new();let mut i=0i64;while i<100 {values.push(Pair{a:Box::new(i),b:i+1});i=i+1;}let last=values.pop().unwrap();*values[20].a+last.b-78}",
        "probe",
        42,
        &[],
    );
}
#[test]
fn partial_box_cleanup_and_conditional_custom_drop() {
    source_case(
        "box_partial",
        "struct Pair{a:Box<i64>,b:Box<i64>} pub fn probe()->i64{let value=Box::new(Pair{a:Box::new(20i64),b:Box::new(22i64)});let a=(*value).a;*a+*value.b}",
        "probe",
        42,
        &[],
    );
    source_case(
        "drop_order",
        include_str!("conformance/pass/closure_capture_drop_order_expanded.rs"),
        "main",
        0,
        b"A1a",
    );
}
#[test]
fn inline_views_keep_lengths_and_utf8_boundaries() {
    source_case(
        "views",
        "pub fn probe()->i64{let values=[10i64,20i64,22i64,99i64];let middle=&values[1..3];let text=\"aéz\";let part=&text[1..3];middle[0]+middle[1]+part.len() as i64-2}",
        "probe",
        42,
        &[],
    );
}

#[test]
fn checked_entry_arguments_cannot_construct_addresses_or_invalid_scalars() {
    for (text, argument) in [
        ("pub fn probe(value:&i64)->i64{*value}", 1),
        ("pub fn probe(value:bool)->bool{value}", 2),
    ] {
        let checked = compiler::check_bundle(&source::SourceBundle {
            root: "entry.rs".to_string(),
            library: None,
            files: vec![source::SourceInput {
                path: "entry.rs".to_string(),
                text: text.to_string(),
            }],
        })
        .unwrap_or_else(|error| panic!("{} {}", error.code, error.message));
        let function = checked
            .program()
            .functions
            .iter()
            .position(|function| function.name == "probe")
            .unwrap();
        let result = native::encode(
            &checked,
            bootstrap_runtime::get().unwrap(),
            native::Entry {
                function,
                arguments: vec![argument],
                result_fd: Some(2),
            },
        );
        assert!(matches!(result, Err(error) if error.code == "E_ENTRY"));
    }
}

#[test]
fn aggregate_result_can_reuse_consumed_argument_storage() {
    let program = ir_text::read(include_str!(
        "conformance/ir/inline_call_result_reuses_argument.lkjir"
    ))
    .unwrap();
    let checked = check::check(program).unwrap();
    checked_case("alias_call", &checked, "probe", 42, &[]);
}
