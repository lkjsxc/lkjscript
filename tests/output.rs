use lkjscript::{check, compiler, k, k_text, source};
use std::path::Path;
use std::process::Command;

#[test]
fn capabilities_cannot_be_fabricated_by_source_or_raw_k() {
    for text in [
        "fn probe() -> World { 42 }",
        "fn probe(mut world: World) { let out = world.stdout::<i64>(); }",
        "fn probe(mut world: World) { let mut out = world.stdout(); out.write::<i64>(&[42u8]); }",
        "fn probe() -> Stdout { Stdout {} }",
        "fn probe(mut world: World) { let other: World = world; let again: World = world; }",
        "fn probe(mut world: World) { let mut out: Stdout = world.stdout(); let copied: Stdout = out.clone(); }",
    ] {
        let bundle = source::SourceBundle {
            root: "forged.rs".to_string(),
            library: None,
            files: vec![source::SourceInput {
                path: "forged.rs".to_string(),
                text: text.to_string(),
            }],
        };
        assert!(compiler::compile(&bundle).is_err(), "accepted: {text}");
    }
    for kind in [k::TypeKind::World, k::TypeKind::Stdout] {
        for op in [
            k::Op::Int { dst: 0, value: 1 },
            k::Op::Unit { dst: 0 },
            k::Op::Struct {
                dst: 0,
                fields: vec![],
            },
        ] {
            let mut types = k::primitives();
            types.push(k::Type {
                name: "capability".to_string(),
                kind: kind.clone(),
            });
            let program = k::Program {
                types,
                destructors: vec![],
                functions: vec![k::Function {
                    file: 0,
                    name: "probe".to_string(),
                    params: 0,
                    result: 7,
                    return_source: None,
                    test: false,
                    locals: vec![k::Local {
                        name: "cap".to_string(),
                        ty: 7,
                        mutable: false,
                    }],
                    body: vec![
                        k::Instruction {
                            column: 1,
                            op,
                            line: 1,
                        },
                        k::Instruction {
                            column: 1,
                            op: k::Op::Return { src: 0 },
                            line: 2,
                        },
                    ],
                }],
            };
            let text = k_text::write(&program);
            assert!(check::check(k_text::read(&text).unwrap()).is_err());
        }
    }
}

#[test]
fn byte_boundary_checks_ranges_copies_and_partial_writes() {
    let host = Path::new(env!("CARGO_MANIFEST_DIR")).join("bridge/output.mjs");
    let script = format!(
        r#"
        import assert from 'node:assert/strict';
        import {{writeBytes}} from '{}';
        const memory={{buffer:new ArrayBuffer(8)}};
        new Uint8Array(memory.buffer).set([0,255,65,10]);
        const received=[];
        const partial=(fd,bytes,at,len)=>{{
          assert.equal(fd,1); const n=Math.min(len,2);
          received.push(...bytes.subarray(at,at+n)); return n;
        }};
        assert.equal(writeBytes(memory,0n,4n,7,partial),0n);
        assert.deepEqual(received,[0,255,65,10]);
        let copied;
        writeBytes(memory,0n,4n,7,(fd,bytes,at,len)=>{{copied=bytes;return len;}});
        new Uint8Array(memory.buffer).fill(42);
        assert.deepEqual([...copied],[0,255,65,10]);
        for(const [pointer,length] of [[-1n,1n],[0n,-1n],[7n,2n],[1n<<64n,0n]]) {{
          assert.throws(()=>writeBytes(memory,pointer,length,17,partial),e=>e.code==='R0007'&&e.line===17);
        }}
        writeBytes(memory,8n,0n,7,()=>{{throw Error('empty write touched host');}});
        assert.throws(()=>writeBytes(memory,0n,1n,8,()=>0),e=>e.code==='R0007');
        assert.throws(()=>writeBytes(memory,0n,1n,8,()=>{{throw Error('I/O');}}),e=>e.code==='R0007');
    "#,
        host.display()
    );
    let output = Command::new("node")
        .args(["--input-type=module", "-e", &script])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn host_output_error_is_reported_at_the_write() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/hello.rs");
    let sink = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .arg("run")
        .arg(source)
        .stdout(sink)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("R0007 host output failure"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn host_imports_require_the_checked_writer_capability() {
    use std::io::Write;
    use std::process::Stdio;
    for (text, expected) in [
        ("fn probe() -> i64 { 42 }", "[]"),
        (
            "fn main(mut world: World) { let mut out = world.stdout(); let bytes: [u8; 1] = [42u8]; out.write(&bytes); }",
            "[{\"module\":\"lkjscript\",\"name\":\"stdout\",\"kind\":\"function\"}]",
        ),
    ] {
        let parsed = lkjscript::syntax::parse(text).unwrap();
        let checked = compiler::check_source(&parsed).unwrap();
        let bytes = lkjscript::wasm::emit(&checked).unwrap();
        let mut child = Command::new("node")
            .args(["-e", "const chunks=[]; process.stdin.on('data', x=>chunks.push(x)); process.stdin.on('end',()=>{const m=new WebAssembly.Module(Buffer.concat(chunks)); const actual=JSON.stringify(WebAssembly.Module.imports(m)); if(actual!==process.argv[1]) throw Error(actual);});", expected])
            .stdin(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        child.stdin.take().unwrap().write_all(&bytes).unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
