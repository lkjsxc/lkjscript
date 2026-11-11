//! Immutable UTF-8 views retain static or borrowed provenance through K and source.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::ast::Diagnostic;
use lkjscript::{check, compiler, k, k_text, source, wasm};
use std::io::Write;
use std::process::{Command, Stdio};

const STR: usize = 7;
const TEXT: usize = 8;
const BYTE_SLICE: usize = 9;
const BYTES: usize = 10;

fn types() -> Vec<k::Type> {
    let mut result = k::primitives();
    result.extend([
        k::Type {
            name: "str".to_string(),
            kind: k::TypeKind::Str,
        },
        k::Type {
            name: "&str".to_string(),
            kind: k::TypeKind::Ref {
                inner: STR,
                mutable: false,
            },
        },
        k::Type {
            name: "[u8]".to_string(),
            kind: k::TypeKind::Slice {
                inner: k::U8,
                mutable: false,
            },
        },
        k::Type {
            name: "&[u8]".to_string(),
            kind: k::TypeKind::Ref {
                inner: BYTE_SLICE,
                mutable: false,
            },
        },
    ]);
    result
}

fn local(ty: usize) -> k::Local {
    k::Local {
        name: String::new(),
        ty,
        mutable: false,
    }
}

fn function(
    name: &str,
    params: usize,
    result: usize,
    return_source: Option<k::ReturnSource>,
    locals: &[usize],
    body: Vec<k::Op>,
) -> k::Function {
    k::Function {
        file: 0,
        name: name.to_string(),
        params,
        result,
        return_source,
        locals: locals.iter().map(|ty| local(*ty)).collect(),
        body: body
            .into_iter()
            .enumerate()
            .map(|(index, op)| k::Instruction {
                op,
                line: index + 1,
            })
            .collect(),
        test: false,
    }
}

fn program(types: Vec<k::Type>, functions: Vec<k::Function>) -> k::Program {
    k::Program {
        types,
        functions,
        destructors: Vec::new(),
    }
}

fn text(dst: usize, value: &str) -> k::Op {
    k::Op::Text {
        dst,
        bytes: value.as_bytes().to_vec(),
    }
}

fn intrinsic(dst: usize, name: &str, args: &[usize]) -> k::Op {
    k::Op::Intrinsic {
        dst,
        name: name.to_string(),
        args: args.to_vec(),
    }
}

fn drop_local(local: usize) -> k::Op {
    k::Op::Drop {
        place: k::Place::local(local),
    }
}

fn accepted(program: k::Program) -> k::CheckedProgram {
    check::check(program).unwrap_or_else(|error| panic!("valid text K rejected: {error:?}"))
}

fn rejected(program: k::Program, code: &str) -> Diagnostic {
    match check::check(program) {
        Ok(_) => panic!("invalid text K was accepted"),
        Err(error) => {
            assert_eq!(error.code, code, "{error:?}");
            error
        }
    }
}

fn literal_program(bytes: Vec<u8>) -> k::Program {
    program(
        types(),
        vec![function(
            "literal",
            0,
            TEXT,
            Some(k::ReturnSource::Static),
            &[TEXT],
            vec![k::Op::Text { dst: 0, bytes }, k::Op::Return { src: 0 }],
        )],
    )
}

#[test]
fn direct_k_static_literals_have_a_checked_text_round_trip() {
    for value in ["", "ASCII", "nul\0byte", "é水😀", "\u{10ffff}"] {
        let original = literal_program(value.as_bytes().to_vec());
        let encoded = k_text::write(&original);
        let decoded = k_text::read(&encoded).expect("text K must round trip");
        assert_eq!(original, decoded);
        accepted(decoded);
    }
}

#[test]
fn direct_k_invalid_utf8_is_rejected_before_emission() {
    for bytes in [
        vec![128],
        vec![192, 128],
        vec![224, 128, 128],
        vec![237, 160, 128],
        vec![240, 128, 128, 128],
        vec![244, 144, 128, 128],
        vec![245, 128, 128, 128],
        vec![194],
        vec![226, 130],
        vec![240, 159, 152],
        vec![255],
    ] {
        let error = match check::check(literal_program(bytes)) {
            Ok(_) => panic!("invalid UTF-8 literal was accepted"),
            Err(error) => error,
        };
        assert!(error.message.contains("UTF-8"), "{error:?}");
    }
}

#[test]
fn direct_k_static_reference_returns_require_the_declared_contract() {
    let mut missing = literal_program(b"stable".to_vec());
    missing.functions[0].return_source = None;
    rejected(missing, "E_BORROW");

    accepted(program(
        types(),
        vec![function(
            "shortened",
            1,
            TEXT,
            Some(k::ReturnSource::Parameter(0)),
            &[TEXT, TEXT],
            vec![text(1, "stable"), drop_local(0), k::Op::Return { src: 1 }],
        )],
    ));
    rejected(
        program(
            types(),
            vec![function(
                "forged_static",
                1,
                TEXT,
                Some(k::ReturnSource::Static),
                &[TEXT],
                vec![k::Op::Return { src: 0 }],
            )],
        ),
        "E_BORROW",
    );
}

#[test]
fn direct_k_no_argument_calls_transport_static_reference_results() {
    let mut module = literal_program("é".as_bytes().to_vec());
    module.functions.push(function(
        "forward",
        0,
        TEXT,
        Some(k::ReturnSource::Static),
        &[TEXT],
        vec![
            k::Op::Call {
                dst: 0,
                function: 0,
                args: Vec::new(),
            },
            k::Op::Return { src: 0 },
        ],
    ));
    accepted(module);
}

#[test]
fn direct_k_aggregate_static_leaves_can_return_without_a_parameter() {
    let mut declarations = types();
    let pair = declarations.len();
    declarations.push(k::Type {
        name: "Pair".to_string(),
        kind: k::TypeKind::Struct {
            copy: true,
            fields: vec![
                k::Field {
                    name: "left".to_string(),
                    ty: TEXT,
                },
                k::Field {
                    name: "right".to_string(),
                    ty: TEXT,
                },
            ],
        },
    });
    accepted(program(
        declarations,
        vec![function(
            "pair",
            0,
            pair,
            Some(k::ReturnSource::Static),
            &[TEXT, TEXT, pair],
            vec![
                text(0, "left"),
                text(1, "right"),
                k::Op::Struct {
                    dst: 2,
                    fields: vec![0, 1],
                },
                k::Op::Return { src: 2 },
            ],
        )],
    ));
}

#[test]
fn direct_k_static_and_parameter_leaves_keep_the_dynamic_lifetime() {
    let mut declarations = types();
    let pointer = declarations.len();
    declarations.push(k::Type {
        name: "&i64".to_string(),
        kind: k::TypeKind::Ref {
            inner: k::I64,
            mutable: false,
        },
    });
    let view = declarations.len();
    declarations.push(k::Type {
        name: "View".to_string(),
        kind: k::TypeKind::Struct {
            copy: false,
            fields: vec![
                k::Field {
                    name: "label".to_string(),
                    ty: TEXT,
                },
                k::Field {
                    name: "value".to_string(),
                    ty: pointer,
                },
            ],
        },
    });
    accepted(program(
        declarations.clone(),
        vec![function(
            "mixed",
            1,
            view,
            Some(k::ReturnSource::Parameter(0)),
            &[pointer, TEXT, pointer, view],
            vec![
                text(1, "label"),
                k::Op::Read {
                    dst: 2,
                    place: k::Place::local(0),
                    moving: false,
                },
                k::Op::Struct {
                    dst: 3,
                    fields: vec![1, 2],
                },
                drop_local(0),
                k::Op::Return { src: 3 },
            ],
        )],
    ));
    rejected(
        program(
            declarations,
            vec![function(
                "mixed_escape",
                1,
                view,
                Some(k::ReturnSource::Parameter(0)),
                &[pointer, TEXT, k::I64, pointer, view],
                vec![
                    text(1, "label"),
                    k::Op::Int { dst: 2, value: 7 },
                    k::Op::Borrow {
                        dst: 3,
                        place: k::Place::local(2),
                        mutable: false,
                    },
                    k::Op::Struct {
                        dst: 4,
                        fields: vec![1, 3],
                    },
                    drop_local(0),
                    k::Op::Return { src: 4 },
                ],
            )],
        ),
        "E_BORROW",
    );
}

#[test]
fn direct_k_borrowed_bytes_retain_the_static_text_origin() {
    accepted(program(
        types(),
        vec![function(
            "bytes",
            0,
            BYTES,
            Some(k::ReturnSource::Static),
            &[TEXT, BYTES],
            vec![
                text(0, "é"),
                intrinsic(1, "str_as_bytes", &[0]),
                k::Op::Return { src: 1 },
            ],
        )],
    ));
}

#[test]
fn direct_k_static_bytes_cannot_be_mutated_or_freed() {
    rejected(
        program(
            types(),
            vec![function(
                "mutate",
                0,
                k::UNIT,
                None,
                &[TEXT, BYTES, k::U64, k::U8, k::UNIT],
                vec![
                    text(0, "A"),
                    intrinsic(1, "str_as_bytes", &[0]),
                    k::Op::Int { dst: 2, value: 0 },
                    k::Op::Int { dst: 3, value: 255 },
                    k::Op::Assign {
                        place: k::Place {
                            local: 1,
                            projections: vec![k::Projection::Deref, k::Projection::Index(2)],
                        },
                        src: 3,
                    },
                    drop_local(1),
                    drop_local(2),
                    k::Op::Unit { dst: 4 },
                    k::Op::Return { src: 4 },
                ],
            )],
        ),
        "E_BORROW",
    );
    rejected(
        program(
            types(),
            vec![function(
                "free",
                0,
                k::UNIT,
                None,
                &[TEXT, k::UNIT],
                vec![
                    text(0, "A"),
                    k::Op::Drop {
                        place: k::Place {
                            local: 0,
                            projections: vec![k::Projection::Deref],
                        },
                    },
                    k::Op::Unit { dst: 1 },
                    k::Op::Return { src: 1 },
                ],
            )],
        ),
        "E_BORROW",
    );
}

#[test]
fn direct_k_mutable_text_references_are_explicitly_unsupported() {
    let mut invalid = literal_program(b"immutable".to_vec());
    invalid.types[TEXT].kind = k::TypeKind::Ref {
        inner: STR,
        mutable: true,
    };
    let error = rejected(invalid, "E_UNSUPPORTED");
    assert!(error.message.contains("mutable text"), "{error:?}");
}

#[test]
fn direct_k_invalid_utf8_is_checked_even_after_a_trap() {
    let mut invalid = literal_program(vec![255]);
    invalid.functions[0].body.insert(
        0,
        k::Instruction {
            op: k::Op::Trap {
                message: "unreachable payload".to_string(),
            },
            line: 1,
        },
    );
    let error = rejected(invalid, "E_TYPE");
    assert!(error.message.contains("UTF-8"), "{error:?}");
}

#[test]
fn direct_k_a_scalar_cannot_forge_a_text_reference() {
    rejected(
        program(
            types(),
            vec![function(
                "forged",
                0,
                TEXT,
                Some(k::ReturnSource::Static),
                &[TEXT],
                vec![k::Op::Int { dst: 0, value: 0 }, k::Op::Return { src: 0 }],
            )],
        ),
        "E_TYPE",
    );
}

#[test]
fn direct_k_borrowing_a_local_text_reference_does_not_make_its_storage_static() {
    let mut declarations = types();
    let double = declarations.len();
    declarations.push(k::Type {
        name: "&&str".to_string(),
        kind: k::TypeKind::Ref {
            inner: TEXT,
            mutable: false,
        },
    });
    rejected(
        program(
            declarations,
            vec![function(
                "local_cell",
                0,
                double,
                Some(k::ReturnSource::Static),
                &[TEXT, double],
                vec![
                    text(0, "static bytes"),
                    k::Op::Borrow {
                        dst: 1,
                        place: k::Place::local(0),
                        mutable: false,
                    },
                    k::Op::Return { src: 1 },
                ],
            )],
        ),
        "E_BORROW",
    );
}

fn result_types() -> (Vec<k::Type>, usize) {
    let mut declarations = types();
    let error = declarations.len();
    declarations.push(k::Type {
        name: "Utf8Error".to_string(),
        kind: k::TypeKind::Struct {
            fields: Vec::new(),
            copy: true,
        },
    });
    let result = declarations.len();
    declarations.push(k::Type {
        name: "Result".to_string(),
        kind: k::TypeKind::Enum {
            copy: true,
            variants: vec![
                k::Variant {
                    name: "Ok".to_string(),
                    fields: vec![k::Field {
                        name: "0".to_string(),
                        ty: TEXT,
                    }],
                },
                k::Variant {
                    name: "Err".to_string(),
                    fields: vec![k::Field {
                        name: "0".to_string(),
                        ty: error,
                    }],
                },
            ],
        },
    });
    (declarations, result)
}

#[test]
fn direct_k_utf8_validation_preserves_borrowed_or_static_provenance() {
    let (declarations, result) = result_types();
    accepted(program(
        declarations.clone(),
        vec![function(
            "validate",
            1,
            result,
            Some(k::ReturnSource::Parameter(0)),
            &[BYTES, result],
            vec![
                intrinsic(1, "str_from_utf8", &[0]),
                k::Op::Return { src: 1 },
            ],
        )],
    ));
    accepted(program(
        declarations,
        vec![function(
            "validate_static",
            0,
            result,
            Some(k::ReturnSource::Static),
            &[TEXT, BYTES, result],
            vec![
                text(0, "checked"),
                intrinsic(1, "str_as_bytes", &[0]),
                intrinsic(2, "str_from_utf8", &[1]),
                k::Op::Return { src: 2 },
            ],
        )],
    ));
}

#[test]
fn direct_k_utf8_validation_cannot_promote_local_bytes_to_static_storage() {
    let (mut declarations, result) = result_types();
    let array = declarations.len();
    declarations.push(k::Type {
        name: "[u8;1]".to_string(),
        kind: k::TypeKind::Array {
            inner: k::U8,
            len: 1,
        },
    });
    rejected(
        program(
            declarations,
            vec![function(
                "local_bytes",
                0,
                result,
                Some(k::ReturnSource::Static),
                &[k::U8, array, BYTES, result],
                vec![
                    k::Op::Int { dst: 0, value: 65 },
                    k::Op::Array {
                        dst: 1,
                        elements: vec![0],
                    },
                    k::Op::SliceBorrow {
                        dst: 2,
                        place: k::Place::local(1),
                        mutable: false,
                    },
                    intrinsic(3, "str_from_utf8", &[2]),
                    k::Op::Return { src: 3 },
                ],
            )],
        ),
        "E_BORROW",
    );
}

fn source_bundle(text: &str) -> source::SourceBundle {
    let mut files = bootstrap_library::standard_library();
    files.push(source::SourceInput {
        path: "text_probe.rs".to_string(),
        text: text.to_string(),
    });
    source::SourceBundle {
        root: "text_probe.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}

fn checked_source(text: &str) -> k::CheckedProgram {
    compiler::check_bundle(&source_bundle(text))
        .unwrap_or_else(|error| panic!("valid source rejected: {error:?}"))
}

fn source_rejected(text: &str, code: &str) {
    match compiler::check_bundle(&source_bundle(text)) {
        Ok(_) => panic!("invalid source was accepted"),
        Err(error) => assert_eq!(error.code, code, "{error:?}"),
    }
}

fn run(checked: k::CheckedProgram, observations: &str) {
    let bytes = wasm::emit(&checked).expect("checked text K must emit");
    let script = format!(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));process.stdin.on('end',async()=>{{try{{const {{instance}}=await WebAssembly.instantiate(Buffer.concat(chunks),{{lkjscript:{{trap(c,l){{throw Error('trap '+c+':'+l)}}}}}});{observations}}}catch(e){{console.error(e);process.exitCode=1}}}})"
    );
    let mut child = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&bytes).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn source_literal_returns_and_byte_views_survive_local_reference_cleanup() {
    let source = r#"
        fn literal() -> &'static str { "Aé水😀" }
        fn bytes() -> &'static [u8] { let text = literal(); text.as_bytes() }
        pub fn probe() -> i64 {
            let view = bytes();
            assert_eq!(view.len(), 10);
            view[0] as i64 + view[9] as i64
        }
    "#;
    run(
        checked_source(source),
        "if(instance.exports['fn:probe']()!==193n)throw Error('static byte view lost its data');",
    );
}

#[test]
fn source_owned_string_views_cannot_escape_their_owner() {
    source_rejected(
        r#"fn leak()->&'static str {let value=String::from_str("local");value.as_str()}"#,
        "E_BORROW",
    );
    source_rejected(
        r#"fn leak()->&'static [u8] {let value=String::from_str("local");value.as_str().as_bytes()}"#,
        "E_BORROW",
    );
    source_rejected(
        r#"fn leak()->&'static &'static str {let value="static";&value}"#,
        "E_BORROW",
    );
}

#[test]
fn source_string_copying_and_append_preserve_literals_and_borrowed_views() {
    let source = r#"
        fn literal() -> &'static str { "é" }
        fn borrowed(value: &String) -> &str { value.as_str() }
        pub fn probe() -> i64 {
            let mut i: i64 = 0;
            while i < 256 {
                let mut owned = String::from_str(literal());
                owned.push_str("::水");
                let view = borrowed(&owned);
                assert_eq!(view.len(), 7);
                assert_eq!(view.as_bytes()[2], 58);
                i = i + 1;
            }
            let text = literal();
            text.len() as i64 + text.as_bytes()[0] as i64
        }
    "#;
    run(
        checked_source(source),
        "for(let i=0;i<10;i++){if(instance.exports['fn:probe']()!==197n)throw Error('owned text changed literal storage');}",
    );
}

#[test]
fn source_aggregate_static_references_return_through_a_checked_call() {
    let source = r#"
        struct Labels<'a> { left: &'a str, right: &'a str }
        fn labels() -> Labels<'static> { Labels { left: "left", right: "right" } }
        pub fn probe() -> i64 {
            let pair = labels();
            pair.left.len() as i64 + pair.right.len() as i64
        }
    "#;
    run(
        checked_source(source),
        "if(instance.exports['fn:probe']()!==9n)throw Error('static aggregate call lost its views');",
    );
}

#[test]
fn source_borrowed_text_cannot_claim_a_static_return_lifetime() {
    source_rejected("fn widen(value:&str)->&'static str {value}", "E_BORROW");
}

#[test]
fn source_static_constraints_without_leaf_contracts_are_named_unsupported() {
    // These annotations need parameter/type leaf contracts. A single return
    // lifetime must not silently erase their stricter meaning.
    for source in [
        "fn takes_static(value:&'static str)->usize{value.len()} fn probe()->usize{let owned=String::from_str(\"local\");takes_static(owned.as_str())}",
        "fn unused<T>(value:&'static str){}",
        "fn mixed<'a>(value:&'a str)->(&'static str,&'a str){(value,value)}",
        "fn mixed(value:&str)->(&'static str,&str){(value,value)}",
        "struct Fixed{value:&'static str}",
        "enum Fixed{Value(&'static str),Empty}",
        "fn local(value:&str){let claimed:&'static str=value;}",
        "fn dead<T>(value:T){return;let claimed:&'static str=\"literal\";}",
        "type Fixed=&'static str; fn unused(value:Fixed){}",
        "struct View<'a>{value:&'a str} fn unused(value:View<'static>){}",
        "trait Source{type Item;} struct Value{} impl Source for Value{type Item=&'static str;}",
        "fn bound<T:'static>(value:T){}",
        "fn lifetime_bound<'a:'static>(value:&'a str){}",
    ] {
        let error = match compiler::check_bundle(&source_bundle(source)) {
            Ok(_) => panic!("unchecked static constraint accepted: {source}"),
            Err(error) => error,
        };
        assert_eq!(error.code, "E_UNSUPPORTED", "{source}: {error:?}");
        assert!(error.message.contains("static"), "{source}: {error:?}");
    }
}

#[test]
fn source_unused_generic_bodies_cannot_return_local_owned_text() {
    source_rejected(
        r#"fn leak<T>(unused:T)->&'static str{let owned=String::from_str("local");owned.as_str()}"#,
        "E_BORROW",
    );
    source_rejected(
        r#"type View<'a>=&'a str;fn leak<T>(unused:T)->View<'static>{let owned=String::from_str("local");owned.as_str()}"#,
        "E_BORROW",
    );
}

#[test]
fn source_static_literals_can_share_a_shorter_dynamic_view_lifetime() {
    let source = r#"
        struct View<'a> { prefix: &'a str, value: &'a str }
        fn view<'a>(value: &'a str) -> View<'a> { View { prefix: "label", value } }
        pub fn probe() -> i64 {
            let owned = String::from_str("data");
            let pair = view(owned.as_str());
            pair.prefix.len() as i64 + pair.value.len() as i64
        }
    "#;
    run(
        checked_source(source),
        "if(instance.exports['fn:probe']()!==9n)throw Error('mixed static/dynamic source lifetime');",
    );
}

#[test]
fn primitive_text_methods_reject_spurious_generic_type_arguments() {
    source_rejected(
        "fn bad(){let bytes:Vec<u8>=Vec::new();std::str::from_utf8::<i64>(bytes.as_slice());}",
        "E_TYPE",
    );
    for method in ["len", "is_empty", "as_bytes", "to_string"] {
        source_rejected(
            &format!("fn bad(){{let value=\"text\".{method}::<bool>();}}"),
            "E_TYPE",
        );
    }
}

#[test]
fn a_user_utf8_function_is_selected_by_its_resolved_declaration() {
    let source = r#"
        mod std {
            pub mod str {
                pub fn from_utf8(value: i64) -> i64 { value + 1 }
            }
        }
        pub fn probe() -> i64 { std::str::from_utf8(41) }
    "#;
    run(
        checked_source(source),
        "if(instance.exports['fn:probe']()!==42n)throw Error('user from_utf8 declaration was hijacked');",
    );
}

#[test]
fn source_an_outer_static_reference_is_not_weakened_by_inner_lifetimes() {
    source_rejected(
        "struct View<'a>{value:&'a str} fn widen<'a>(value:&'a View<'a>)->&'static View<'a>{value}",
        "E_BORROW",
    );
}

#[test]
fn source_impl_methods_cannot_weaken_a_static_trait_result_contract() {
    let source = "trait Text{fn get(&self)->&'static str;} impl Text for String{fn get(&self)->&str{self.as_str()}}";
    let error = match compiler::check_bundle(&source_bundle(source)) {
        Ok(_) => panic!("implementation weakened the declared static trait result"),
        Err(error) => error,
    };
    assert_eq!(error.code, "E_TRAIT", "{error:?}");
    assert!(error.message.contains("static"), "{error:?}");
}

#[test]
fn source_generic_trait_calls_preserve_static_result_contracts() {
    let source = r#"
        trait Text { fn get(&self) -> &'static str; }
        impl Text for i64 { fn get(&self) -> &'static str { "checked" } }
        fn via<T:Text>(value: &T) -> &'static str { value.get() }
        pub fn probe() -> i64 { via(&1i64).len() as i64 }
    "#;
    run(
        checked_source(source),
        "if(instance.exports['fn:probe']()!==7n)throw Error('static generic trait result lost its contract');",
    );
}
