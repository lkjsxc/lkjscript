//! Backend text storage and UTF-8 validation exercise checked K directly.
use lkjscript::{check, k, wasm};
use std::io::Write;
use std::process::{Command, Stdio};

const STR: usize = 7;
const TEXT: usize = 8;
const SLICE: usize = 9;
const BYTES: usize = 10;
const ERROR: usize = 11;
const RESULT: usize = 12;
const VEC: usize = 13;
const VEC_MUT: usize = 14;
const BOX_VEC: usize = 15;
const HOLDER: usize = 16;

fn types() -> Vec<k::Type> {
    let mut types = k::primitives();
    for (name, kind) in [
        ("str", k::TypeKind::Str),
        (
            "&str",
            k::TypeKind::Ref {
                inner: STR,
                mutable: false,
            },
        ),
        (
            "[u8]",
            k::TypeKind::Slice {
                inner: k::U8,
                mutable: false,
            },
        ),
        (
            "&[u8]",
            k::TypeKind::Ref {
                inner: SLICE,
                mutable: false,
            },
        ),
        (
            "Utf8Error",
            k::TypeKind::Struct {
                fields: Vec::new(),
                copy: true,
            },
        ),
        (
            "Result",
            k::TypeKind::Enum {
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
                            ty: ERROR,
                        }],
                    },
                ],
                copy: false,
            },
        ),
        ("Vec<u8>", k::TypeKind::Vec { inner: k::U8 }),
        (
            "&mut Vec<u8>",
            k::TypeKind::Ref {
                inner: VEC,
                mutable: true,
            },
        ),
        ("Box<Vec<u8>>", k::TypeKind::Box { inner: VEC }),
        (
            "Holder",
            k::TypeKind::Struct {
                fields: vec![k::Field {
                    name: "text".to_string(),
                    ty: TEXT,
                }],
                copy: true,
            },
        ),
    ] {
        types.push(k::Type {
            name: name.to_string(),
            kind,
        });
    }
    types
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
        locals: locals
            .iter()
            .map(|ty| k::Local {
                name: String::new(),
                ty: *ty,
                mutable: true,
            })
            .collect(),
        body: body
            .into_iter()
            .map(|op| k::Instruction {
                column: 1,
                op,
                line: 1,
            })
            .collect(),
        test: false,
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

fn text(dst: usize, value: &str) -> k::Op {
    k::Op::Text {
        dst,
        bytes: value.as_bytes().to_vec(),
    }
}

fn literal(name: &str, value: &str) -> k::Function {
    function(
        name,
        0,
        TEXT,
        Some(k::ReturnSource::Static),
        &[TEXT],
        vec![text(0, value), k::Op::Return { src: 0 }],
    )
}

fn buffer_functions() -> Vec<k::Function> {
    let mut body = vec![intrinsic(0, "vec_new", &[])];
    // A live16-byte allocation can also hold a raw two-word view result.
    for _ in 0..16 {
        body.extend([
            k::Op::Borrow {
                dst: 1,
                place: k::Place::local(0),
                mutable: true,
            },
            k::Op::Int { dst: 2, value: 0 },
            intrinsic(3, "vec_push", &[1, 2]),
            drop_local(3),
        ]);
    }
    body.extend([intrinsic(4, "box_new", &[0]), k::Op::Return { src: 4 }]);
    vec![
        function(
            "buffer",
            0,
            BOX_VEC,
            None,
            &[VEC, VEC_MUT, k::U8, k::UNIT, BOX_VEC],
            body,
        ),
        function(
            "release",
            1,
            k::UNIT,
            None,
            &[BOX_VEC, k::UNIT],
            vec![
                drop_local(0),
                k::Op::Unit { dst: 1 },
                k::Op::Return { src: 1 },
            ],
        ),
    ]
}

fn run(functions: Vec<k::Function>, script: &str) {
    let checked = check::check(k::Program {
        types: types(),
        functions,
        destructors: Vec::new(),
    })
    .unwrap_or_else(|problem| panic!("invalid test K: {problem:?}"));
    let bytes = wasm::emit(&checked).expect("text emission");
    let script = format!(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));process.stdin.on('end',async()=>{{try{{\
         const {{instance}}=await WebAssembly.instantiate(Buffer.concat(chunks),{{lkjscript:{{trap(c,l){{throw Error('trap '+c+':'+l)}}}}}});\
         const e=instance.exports;{script}\
         }}catch(error){{console.error(error);process.exitCode=1}}}});"
    );
    let mut child = Command::new("node")
        .args(["-e", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node is required for backend validation");
    child.stdin.take().unwrap().write_all(&bytes).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn literal_pool_is_immutable_deduplicated_and_separate_from_heap() {
    let unicode = "nul\0é水😀";
    let large = "λ".repeat(40_000);
    let mut functions = vec![
        literal("empty", ""),
        literal("unicode", unicode),
        literal("duplicate", unicode),
        literal("large", &large),
        function(
            "copy_holder",
            0,
            TEXT,
            Some(k::ReturnSource::Static),
            &[TEXT, HOLDER, HOLDER, TEXT],
            vec![
                text(0, unicode),
                k::Op::Struct {
                    dst: 1,
                    fields: vec![0],
                },
                k::Op::Read {
                    dst: 2,
                    place: k::Place::local(1),
                    moving: false,
                },
                drop_local(1),
                k::Op::Read {
                    dst: 3,
                    place: k::Place {
                        local: 2,
                        projections: vec![k::Projection::Field(0)],
                    },
                    moving: false,
                },
                drop_local(2),
                k::Op::Return { src: 3 },
            ],
        ),
        function(
            "text_length",
            0,
            k::U64,
            None,
            &[TEXT, k::U64],
            vec![
                text(0, unicode),
                intrinsic(1, "str_len", &[0]),
                k::Op::Return { src: 1 },
            ],
        ),
        function(
            "bytes_length",
            0,
            k::U64,
            None,
            &[TEXT, TEXT, BYTES, k::U64],
            vec![
                text(0, unicode),
                k::Op::Read {
                    dst: 1,
                    place: k::Place::local(0),
                    moving: false,
                },
                drop_local(0),
                intrinsic(2, "str_as_bytes", &[1]),
                intrinsic(3, "vec_len", &[2]),
                k::Op::Return { src: 3 },
            ],
        ),
    ];
    functions.extend(buffer_functions());
    let script = format!(
        r#"
        let end=0;
        // The hidden output address is backed by a live source allocation;
        // neither address0 nor the immutable text region is writable scratch.
        const outputOwner=e['fn:buffer']();
        const initial=new DataView(e.memory.buffer);
        const outputHeader=Number(initial.getBigUint64(Number(outputOwner),true));
        const output=Number(initial.getBigUint64(outputHeader,true));
        if(initial.getBigUint64(outputHeader+8,true)<16n)throw Error('output extent');
        function inspect(name,expected){{
            if(e['fn:'+name](BigInt(output))!==0n)throw Error('indirect result marker');
            const view=new DataView(e.memory.buffer);
            const data=Number(view.getBigUint64(output,true));
            const length=Number(view.getBigUint64(output+8,true));
            if(length!==expected.length)throw Error(name+' length '+length);
            for(let i=0;i<length;i++)if(view.getUint8(data+i)!==expected[i])throw Error(name+' byte '+i);
            end=Math.max(end,data+length);
            return BigInt(data);
        }}
        inspect('empty',[]);
        const text=inspect('unicode',{unicode_bytes:?});
        if(inspect('duplicate',{unicode_bytes:?})!==text)throw Error('literal not pooled');
        inspect('large',Array.from(Buffer.from('λ'.repeat(40000))));
        if(e.memory.buffer.byteLength<131072)throw Error('static data did not initialize extra page');
        if(output<end)throw Error('output slot overlaps immutable bytes');
        const saved=Buffer.from(new Uint8Array(e.memory.buffer,0,end));
        for(let i=0;i<1000;i++){{
            if(inspect('copy_holder',{unicode_bytes:?})!==text)throw Error('copy changed text identity');
            if(e['fn:text_length']()!=={unicode_len}n||e['fn:bytes_length']()!=={unicode_len}n)throw Error('view length');
            const buffer=e['fn:buffer']();
            if(buffer<BigInt(end))throw Error('heap overlaps static data');
            e['fn:release'](buffer);
        }}
        if(!saved.equals(Buffer.from(new Uint8Array(e.memory.buffer,0,end))))throw Error('literal storage was modified or released');
        e['fn:release'](outputOwner);
        "#,
        unicode_bytes = unicode.as_bytes(),
        unicode_len = unicode.len(),
    );
    run(functions, &script);
}

#[test]
fn runtime_utf8_validation_matches_rust_at_encoding_boundaries() {
    // Exhaust every one/two-byte input and the boundary continuations for each
    // three/four-byte lead. These are byte values, not source literal spellings.
    let mut expected = Vec::new();
    let mut count = 0;
    let mut observe = |bytes: &[u8]| {
        if count % 8 == 0 {
            expected.push(0_u8);
        }
        if std::str::from_utf8(bytes).is_ok() {
            *expected.last_mut().unwrap() |= 1 << (count % 8);
        }
        count += 1;
    };
    observe(&[]);
    for first in 0..=255 {
        observe(&[first]);
        for second in 0..=255 {
            observe(&[first, second]);
        }
    }
    for first in 224..=239 {
        for second in 128..=191 {
            for third in [127, 128, 191, 192] {
                observe(&[first, second, third]);
            }
        }
    }
    for first in 240..=244 {
        for second in 128..=191 {
            for third in [128, 191] {
                for fourth in [127, 128, 191, 192] {
                    observe(&[first, second, third, fourth]);
                }
            }
        }
    }
    let extra = [
        vec![0xf0, 0x9f, 0x98],
        vec![0xf4, 0x8f, 0xbf],
        vec![0xe0, 0xa0],
        vec![0x61, 0xc2, 0x80],
        vec![0xc2, 0x80, 0x61],
        vec![0xe2, 0x82, 0xac, 0x61],
        vec![0x61, 0xe2, 0x82, 0xac],
        vec![0xc2, 0x80, 0xc2, 0x80],
        vec![0x61, 0x62, 0x63, 0x64],
    ];
    for bytes in &extra {
        observe(bytes);
    }
    let hex: String = expected.iter().map(|byte| format!("{byte:02x}")).collect();
    let mut functions = buffer_functions();
    functions.push(function(
        "validate",
        1,
        k::BOOL,
        None,
        &[BYTES, RESULT, k::BOOL],
        vec![
            intrinsic(1, "str_from_utf8", &[0]),
            k::Op::TagTest {
                dst: 2,
                place: k::Place::local(1),
                variant: 0,
            },
            drop_local(1),
            k::Op::Return { src: 2 },
        ],
    ));
    let script = format!(
        r#"
        const expected=Buffer.from('{hex}','hex');
        const slot=e['fn:buffer']();
        const view=new DataView(e.memory.buffer);
        const header=Number(view.getBigUint64(Number(slot),true));
        const data=Number(view.getBigUint64(header,true));
        let index=0;
        function verify(bytes){{
            view.setBigUint64(header+8,BigInt(bytes.length),true);
            for(let i=0;i<bytes.length;i++)view.setUint8(data+i,bytes[i]);
            const actual=e['fn:validate'](BigInt(data),BigInt(bytes.length));
            const answer=BigInt((expected[index>>3]>>(index&7))&1);
            if(actual!==answer)throw Error('UTF8 '+bytes+' expected '+answer+' got '+actual);
            index++;
        }}
        verify([]);
        for(let first=0;first<=255;first++){{
            verify([first]);
            for(let second=0;second<=255;second++)verify([first,second]);
        }}
        for(let first=224;first<=239;first++)
            for(let second=128;second<=191;second++)
                for(const third of [127,128,191,192])verify([first,second,third]);
        for(let first=240;first<=244;first++)
            for(let second=128;second<=191;second++)
                for(const third of [128,191])
                    for(const fourth of [127,128,191,192])verify([first,second,third,fourth]);
        for(const bytes of {extra:?})verify(bytes);
        if(index!=={count})throw Error('case count '+index);
        e['fn:release'](slot);
        "#,
    );
    run(functions, &script);
}
