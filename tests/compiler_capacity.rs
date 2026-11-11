use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{compiler, k, wasm};
use std::io::Write;
use std::process::{Command, Stdio};

fn bundle(source: String) -> SourceBundle {
    SourceBundle {
        root: "capacity/main.rs".to_string(),
        library: None,
        files: vec![SourceInput {
            path: "capacity/main.rs".to_string(),
            text: source,
        }],
    }
}

fn isolated_small_stack(name: &str, work: fn()) {
    const CHILD: &str = "LKJSCRIPT_TYPE_COMPILER_CHILD";
    if std::env::var(CHILD).ok().as_deref() == Some(name) {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(work)
            .unwrap()
            .join()
            .unwrap();
        return;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env(CHILD, name)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "small-stack compiler subprocess failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn execute(bytes: &[u8], expected: i64) {
    let mut child = Command::new("node")
        .arg("-e")
        .arg(format!(
            "const chunks=[];process.stdin.on('data',b=>chunks.push(b));
             process.stdin.on('end',async()=>{{
                 const {{instance}}=await WebAssembly.instantiate(Buffer.concat(chunks),
                    {{lkjscript:{{trap(code,line){{throw Error('trap '+code+':'+line)}}}}}});
                 if(instance.exports['fn:probe']()!=={expected}n)throw Error('probe result');
             }});"
        ))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn nested_generic_types_compile_on_small_native_stack() {
    isolated_small_stack("nested_generic_types_compile_on_small_native_stack", || {
        for depth in [512, 2048] {
            for (definition, constructor) in [("", "Box"), ("type Layer<T> = Box<T>;", "Layer")] {
                let source = format!(
                    "{definition} fn consume(_x: {}i64{}) {{}} fn probe() -> i64 {{ 0 }}",
                    format!("{constructor}<").repeat(depth),
                    ">".repeat(depth),
                );
                // This is the ordinary complete source-bundle compiler entry,
                // including loading, resolution, checking and Wasm emission.
                let bytes = compiler::compile(&bundle(source)).unwrap();
                execute(&bytes, 0);
            }
        }
    });
}

#[test]
fn type_frames_preserve_nominal_identity_fields_and_recursive_aliases() {
    isolated_small_stack(
        "type_frames_preserve_nominal_identity_fields_and_recursive_aliases",
        || {
            let depth = 512;
            let source = format!(
                "struct Box<T> {{ value: T }}
             fn consume(_x: {}i64{}) {{}}
             fn probe() -> i64 {{ 0 }}",
                "Box<".repeat(depth),
                ">".repeat(depth),
            );
            let checked = compiler::check_bundle(&bundle(source)).unwrap();
            assert!(
                checked
                    .program()
                    .types
                    .iter()
                    .all(|ty| !matches!(ty.kind, k::TypeKind::Box { .. }))
            );
            assert_eq!(
                checked
                    .program()
                    .types
                    .iter()
                    .filter(|ty| matches!(ty.kind, k::TypeKind::Struct { .. }))
                    .count(),
                depth
            );
            execute(&wasm::emit(&checked).unwrap(), 0);

            // Reverse declaration order requires traversing nominal field edges;
            // it cannot succeed merely by pre-resolving declarations in source order.
            let mut source = String::new();
            for index in (1..=depth).rev() {
                source.push_str(&format!("struct N{index} {{ value: N{} }}\n", index - 1));
            }
            source.push_str("struct N0 { value: i64 } fn probe() -> i64 { 0 }");
            execute(&compiler::compile(&bundle(source)).unwrap(), 0);

            // A repeated alias is valid through a recursive nominal placeholder.
            // Treating every active alias key as a cycle would reject this program.
            let source = "
            type Link<T> = Chain<T>;
            enum Chain<T> { End, Next(T, Box<Link<T>>) }
            fn probe() -> i64 {
                let mut list: Link<i64> = Chain::End;
                let mut count: i64 = 0;
                while count < 1000 {
                    list = Chain::Next(count, Box::new(list));
                    count = count + 1;
                }
                count
            }
        ";
            execute(
                &compiler::compile(&bundle(source.to_string())).unwrap(),
                1000,
            );
        },
    );
}

#[test]
fn alias_cycles_fail_semantically_before_type_work_expands() {
    for source in [
        "type A = B; type B = A; fn probe() -> i64 { 0 }",
        "type A<T> = B<T>; type B<T> = A<T>; fn probe() -> i64 { 0 }",
    ] {
        let error = match compiler::compile(&bundle(source.to_string())) {
            Ok(_) => panic!("recursive alias accepted"),
            Err(error) => error,
        };
        assert_eq!(error.code, "E_TYPE");
        assert!(error.message.contains("recursive type alias"), "{error:?}");
    }
}
