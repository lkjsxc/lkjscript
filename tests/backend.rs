//! Test-only host bridge: existing Rust arithmetic and Node are the oracles.
use lkjscript::{check, elaborate, k, syntax, wasm};
use std::io::Write;
use std::process::{Command, Stdio};

fn run_node(bytes: &[u8], body: &str) {
    let script = format!(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));\
         process.stdin.on('end',async()=>{{try{{\
         const {{instance}}=await WebAssembly.instantiate(Buffer.concat(chunks),\
         {{lkjscript:{{trap(code,line){{throw Error('trap '+code+':'+line)}}}}}});\
         {body}\
         }}catch(e){{console.error(e);process.exitCode=1}}}});"
    );
    let mut child = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node is required for backend conformance");
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn instruction(op: k::Op) -> k::Instruction {
    k::Instruction { op, line: 1 }
}

#[test]
fn checked_integer_edges_match_rust() {
    let mut functions = Vec::new();
    let mut cases = Vec::new();
    let pairs: [(u64, u64); 13] = [
        (0, 0),
        (1, 0),
        (0, 1),
        (u64::MAX, 1),
        (i64::MAX as u64, 1),
        (i64::MIN as u64, u64::MAX),
        (u64::MAX, u64::MAX),
        (1_u64 << 63, 2),
        (1_u64 << 62, 4),
        (i32::MAX as u64, 2),
        (i32::MIN as u64, u64::MAX),
        (255, 255),
        (u64::MAX, 2),
    ];
    for (ty, prefix) in [(k::I64, "i64"), (k::I32, "i32"), (k::U64, "u64")] {
        for (op, label) in [
            ("+", "add"),
            ("-", "sub"),
            ("*", "mul"),
            ("/", "div"),
            ("%", "rem"),
        ] {
            let name = format!("{prefix}_{label}");
            functions.push(k::Function {
                name: name.clone(),
                params: 2,
                result: ty,
                test: false,
                return_source: None,
                locals: vec![
                    k::Local {
                        name: "a".to_string(),
                        ty,
                        mutable: false,
                    },
                    k::Local {
                        name: "b".to_string(),
                        ty,
                        mutable: false,
                    },
                    k::Local {
                        name: "result".to_string(),
                        ty,
                        mutable: false,
                    },
                ],
                body: vec![
                    instruction(k::Op::Binary {
                        dst: 2,
                        op: op.to_string(),
                        left: 0,
                        right: 1,
                    }),
                    instruction(k::Op::Return { src: 2 }),
                ],
            });
            for (a, b) in pairs {
                let (a, b, expected) = if ty == k::I64 {
                    let (x, y) = (a as i64, b as i64);
                    let answer = match op {
                        "+" => x.checked_add(y),
                        "-" => x.checked_sub(y),
                        "*" => x.checked_mul(y),
                        "/" => x.checked_div(y),
                        _ => x.checked_rem(y),
                    };
                    (x.to_string(), y.to_string(), answer.map(|x| x.to_string()))
                } else if ty == k::I32 {
                    let (x, y) = (a as i32, b as i32);
                    let answer = match op {
                        "+" => x.checked_add(y),
                        "-" => x.checked_sub(y),
                        "*" => x.checked_mul(y),
                        "/" => x.checked_div(y),
                        _ => x.checked_rem(y),
                    };
                    (x.to_string(), y.to_string(), answer.map(|x| x.to_string()))
                } else {
                    let answer = match op {
                        "+" => a.checked_add(b),
                        "-" => a.checked_sub(b),
                        "*" => a.checked_mul(b),
                        "/" => a.checked_div(b),
                        _ => a.checked_rem(b),
                    };
                    (
                        a.to_string(),
                        b.to_string(),
                        answer.map(|x| (x as i64).to_string()),
                    )
                };
                let expected = expected.unwrap_or_else(|| "trap".to_string());
                cases.push(format!(r#"["{name}","{a}","{b}","{expected}"]"#));
            }
        }
    }
    let program = check::check(k::Program {
        types: k::primitives(),
        functions,
        destructors: Vec::new(),
    })
    .unwrap();
    let bytes = wasm::emit(&program).unwrap();
    run_node(
        &bytes,
        &format!(
            "for(const [name,a,b,expected] of [{}]){{\
         let actual;try{{actual=instance.exports['fn:'+name](BigInt(a),BigInt(b)).toString()}}\
         catch{{actual='trap'}}\
         if(actual!==expected)throw Error([name,a,b,expected,actual].join(' '));\
         }}",
            cases.join(",")
        ),
    );
}

#[test]
fn aggregate_copy_and_released_storage_remain_independent() {
    let source = r#"
        #[derive(Copy, Clone)]
        struct Inner { x: i64 }
        #[derive(Copy, Clone)]
        struct Outer { inner: Inner }
        fn copy_probe() -> i64 {
            let mut a = Outer { inner: Inner { x: 7 } };
            let b = a;
            a.inner.x = 99;
            b.inner.x
        }
        fn vec_probe() -> i64 {
            let mut values: Vec<i64> = Vec::new();
            let mut i: i64 = 0;
            while i < 1000 {
                values.push(i);
                i = i + 1;
            }
            values[999] + values.len() as i64
        }
    "#;
    let ast = syntax::parse(source).unwrap();
    let program = check::check(elaborate::elaborate(&ast).unwrap()).unwrap();
    let bytes = wasm::emit(&program).unwrap();
    run_node(
        &bytes,
        "
        if(instance.exports['fn:copy_probe']()!==7n)throw Error('Copy aliases nested fields');
        if(instance.exports['fn:vec_probe']()!==1999n)throw Error('Vec growth result');
        const before=instance.exports.memory.buffer.byteLength;
        for(let i=0;i<1000;i++){
            if(instance.exports['fn:copy_probe']()!==7n||instance.exports['fn:vec_probe']()!==1999n)
                throw Error('released storage was reused incorrectly');
        }
        if(instance.exports.memory.buffer.byteLength!==before)
            throw Error('aggregate drops leaked linear memory');
    ",
    );
}

#[test]
fn narrow_complement_and_float_conversions_match_rust() {
    let mut functions = Vec::new();
    let mut checks = Vec::new();
    let conversions = [
        (
            "negative_nan",
            k::F64,
            k::F64,
            0x7ff8_0000_0000_0000_u64,
            "-",
        ),
        ("u8_not", k::U8, k::U8, 128, "!"),
        (
            "float_i32_max",
            k::F64,
            k::I32,
            f64::INFINITY.to_bits(),
            "cast",
        ),
        (
            "float_u8_negative",
            k::F64,
            k::U8,
            (-3.5_f64).to_bits(),
            "cast",
        ),
        (
            "float_i64_min",
            k::F64,
            k::I64,
            f64::NEG_INFINITY.to_bits(),
            "cast",
        ),
        (
            "float_nan_integer",
            k::F64,
            k::I64,
            f64::NAN.to_bits(),
            "cast",
        ),
    ];
    let expected = [
        0x7ff8_0000_0000_0000_i64,
        i64::from(!128_u8),
        i64::from(f64::INFINITY as i32),
        i64::from(-3.5_f64 as u8),
        f64::NEG_INFINITY as i64,
        0, // Rust defines a NaN-to-integer cast as zero.
    ];
    for (index, (name, from, to, bits, op)) in conversions.into_iter().enumerate() {
        let literal = if from == k::F64 {
            k::Op::Float { dst: 0, bits }
        } else {
            k::Op::Int {
                dst: 0,
                value: bits as i64,
            }
        };
        let operation = if op == "cast" {
            k::Op::Intrinsic {
                dst: 1,
                name: op.to_string(),
                args: vec![0],
            }
        } else {
            k::Op::Unary {
                dst: 1,
                op: op.to_string(),
                src: 0,
            }
        };
        functions.push(k::Function {
            name: name.to_string(),
            params: 0,
            result: to,
            test: false,
            return_source: None,
            locals: vec![
                k::Local {
                    name: "input".to_string(),
                    ty: from,
                    mutable: false,
                },
                k::Local {
                    name: "output".to_string(),
                    ty: to,
                    mutable: false,
                },
            ],
            body: vec![
                instruction(literal),
                instruction(operation),
                instruction(k::Op::Return { src: 1 }),
            ],
        });
        checks.push(format!(
            "if(instance.exports['fn:{name}']()!=={}n)throw Error('{name}');",
            expected[index]
        ));
    }
    let checked = check::check(k::Program {
        types: k::primitives(),
        functions,
        destructors: Vec::new(),
    })
    .unwrap();
    run_node(&wasm::emit(&checked).unwrap(), &checks.join(""));
}

#[test]
fn arrays_coerce_to_slices_and_keep_value_semantics() {
    let source = r#"
        fn sum(values: &[i64]) -> i64 {
            let mut i: usize = 0;
            let mut total: i64 = 0;
            while i < values.len() {
                total = total + values[i];
                i = i + 1;
            }
            total
        }
        fn array_probe() -> i64 {
            let mut original: [i64; 3] = [1, 2, 3];
            let copied = original;
            {
                let view: &mut [i64] = &mut original;
                view[1] = 99;
            }
            assert_eq!(copied[1], 2);
            sum(&original) + sum(&copied)
        }
    "#;
    let ast = syntax::parse(source).unwrap();
    let program = check::check(elaborate::elaborate(&ast).unwrap()).unwrap();
    run_node(
        &wasm::emit(&program).unwrap(),
        "
        if(instance.exports['fn:array_probe']()!==109n)throw Error('array/slice result');
        const size=instance.exports.memory.buffer.byteLength;
        for(let i=0;i<1000;i++)if(instance.exports['fn:array_probe']()!==109n)throw Error('array reuse');
        if(instance.exports.memory.buffer.byteLength!==size)throw Error('array storage leak');
    ",
    );
}

#[test]
fn registered_drop_runs_on_return_break_and_nested_release() {
    let source = r#"
        struct Counter<'a> { count: &'a mut i64 }
        impl<'a> Drop for Counter<'a> {
            fn drop(&mut self) { *self.count = *self.count + 1; }
        }
        struct Parent<'a> { child: Counter<'a> }
        fn returning(count: &mut i64) -> i64 {
            let counter = Counter { count };
            return 7;
        }
        fn breaking(count: &mut i64) {
            loop {
                let counter = Counter { count };
                break;
            }
        }
        fn drop_probe() -> i64 {
            let mut count: i64 = 0;
            let value = returning(&mut count);
            breaking(&mut count);
            { let parent = Parent { child: Counter { count: &mut count } }; }
            count + value
        }
    "#;
    let ast = syntax::parse(source).unwrap();
    let program = check::check(elaborate::elaborate(&ast).unwrap()).unwrap();
    run_node(&wasm::emit(&program).unwrap(), "
        if(instance.exports['fn:drop_probe']()!==10n)throw Error('destructor count');
        const size=instance.exports.memory.buffer.byteLength;
        for(let i=0;i<1000;i++)if(instance.exports['fn:drop_probe']()!==10n)throw Error('destructor reuse');
        if(instance.exports.memory.buffer.byteLength!==size)throw Error('destructor receiver cell leak');
    ");
}

#[test]
fn function_exports_have_a_separate_namespace() {
    let mut functions = Vec::new();
    for (name, value) in [("memory", 41_i64), ("fn:memory", 42_i64)] {
        functions.push(k::Function {
            name: name.to_string(),
            params: 0,
            result: k::I64,
            test: false,
            return_source: None,
            locals: vec![k::Local {
                name: "value".to_string(),
                ty: k::I64,
                mutable: false,
            }],
            body: vec![
                instruction(k::Op::Int { dst: 0, value }),
                instruction(k::Op::Return { src: 0 }),
            ],
        });
    }
    let checked = check::check(k::Program {
        types: k::primitives(),
        functions,
        destructors: Vec::new(),
    })
    .unwrap();
    run_node(
        &wasm::emit(&checked).unwrap(),
        "
        if(!(instance.exports.memory instanceof WebAssembly.Memory))throw Error('memory export');
        if(instance.exports['fn:memory']()!==41n)throw Error('source memory function');
        if(instance.exports['fn:fn:memory']()!==42n)throw Error('explicit K function name');
    ",
    );
}
