//! Raw profile-4 certificates are hostile input, even when their digest is valid.
use super::*;

fn node(tag: u32, a: u32, b: u32, initialized: &[u8], stack: &[u8]) -> input::Node {
    input::Node {
        tag,
        a,
        b,
        initialized: initialized.to_vec(),
        stack: stack.to_vec(),
        ..input::Node::default()
    }
}

#[test]
fn selfhost_admission_checks_derived_hash_signatures_and_unknown_codes() {
    let functions = vec![
        input::Function {
            result: 1,
            nodes: vec![node(1, 0, 0, &[], &[]), node(11, 0, 0, &[], &[1])],
            ..input::Function::default()
        },
        input::Function {
            result: 5,
            parameters: 2,
            locals: vec![4, 5],
            external: 28,
            ..input::Function::default()
        },
    ];
    let (code, image) = compile(&input::encode(&functions, 0).unwrap());
    assert_eq!(code, 0);
    assert_eq!(execute(&image, &[]).stdout, b"0");
    for (result, locals, external, expected) in [
        (1, vec![4, 5], 28, 8),
        (5, vec![5, 4], 28, 8),
        (5, vec![4], 28, 8),
        (5, vec![6, 5], 28, 8),
        (5, vec![4, 134], 28, 8),
        (5, vec![4, 5], 29, 5),
    ] {
        let mut changed = functions.clone();
        changed[1].result = result;
        changed[1].parameters = locals.len() as u32;
        changed[1].locals = locals;
        changed[1].external = external;
        let (code, image) = compile(&input::encode(&changed, 0).unwrap());
        assert_eq!(code, expected);
        assert!(image.is_empty());
    }
}

fn buffer_program() -> Vec<input::Function> {
    let mut nodes = vec![
        node(9, 1, 0, &[0], &[]),
        node(5, 0, 0, &[0], &[6]),
        node(16, 0, 0, &[1], &[]),
        node(9, 2, 1, &[0], &[6]),
        node(6, 0, 0, &[0], &[5]),
        node(1, 0, 0, &[0], &[]),
        node(11, 0, 0, &[0], &[1]),
    ];
    nodes[5].immediate = 42;
    vec![
        input::Function {
            result: 1,
            locals: vec![6],
            nodes,
            ..input::Function::default()
        },
        input::Function {
            result: 6,
            external: 20,
            ..input::Function::default()
        },
        input::Function {
            result: 5,
            parameters: 1,
            locals: vec![6],
            external: 24,
            ..input::Function::default()
        },
    ]
}

fn forgotten_buffer_program() -> Vec<input::Function> {
    let mut functions = buffer_program();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/selfhost-forgotten-buffer.json"
    ))
    .unwrap();
    functions[0].nodes = fixture["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let values = |i| {
                row[i]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_u64().unwrap() as u8)
                    .collect::<Vec<_>>()
            };
            node(
                row[0].as_u64().unwrap() as u32,
                row[1].as_u64().unwrap() as u32,
                row[2].as_u64().unwrap() as u32,
                &values(3),
                &values(4),
            )
        })
        .collect();
    functions
}

pub(super) fn forgotten_buffer_input() -> Vec<u8> {
    input::encode(&forgotten_buffer_program(), 0).unwrap()
}

#[test]
fn selfhost_admission_rejects_buffer_aliases_loans_and_forgotten_custody() {
    let valid = buffer_program();
    let (code, image) = compile(&input::encode(&valid, 0).unwrap());
    assert_eq!(code, 0);
    assert_eq!(execute(&image, &[]).stdout, b"42");
    let mut cases = Vec::new();
    // Copying an owned buffer would alias its mutable custody.
    let mut changed = valid.clone();
    changed[0].nodes[2].tag = 4;
    cases.push(("copy-buffer", changed, 8));
    // A move must clear custody on the very next edge.
    let mut changed = valid.clone();
    changed[0].nodes[3].initialized[0] = 1;
    cases.push(("move-retains-owner", changed, 8));
    // A borrow cannot satisfy a consuming parameter.
    let mut changed = valid.clone();
    changed[0].nodes[2] = node(4, 0, 1, &[1], &[]);
    changed[0].nodes[3] = node(9, 2, 1, &[1], &[134]);
    cases.push(("consume-borrow", changed, 8));
    // A live loan cannot coexist with a subsequent move.
    let mut changed = valid.clone();
    changed[0].nodes = vec![
        node(9, 1, 0, &[0], &[]),
        node(5, 0, 0, &[0], &[6]),
        node(4, 0, 1, &[1], &[]),
        node(16, 0, 0, &[1], &[134]),
        node(6, 0, 0, &[0], &[134, 6]),
        node(6, 0, 0, &[0], &[134]),
        node(1, 0, 0, &[0], &[]),
        node(11, 0, 0, &[0], &[1]),
    ];
    cases.push(("move-with-loan", changed, 8));
    // Explicit lexical cleanup must also respect a live loan.
    let mut changed = valid.clone();
    changed[0].nodes = vec![
        node(9, 1, 0, &[0], &[]),
        node(5, 0, 0, &[0], &[6]),
        node(4, 0, 1, &[1], &[]),
        node(3, 0, 0, &[1], &[134]),
        node(5, 0, 0, &[1], &[134, 3]),
        node(6, 0, 0, &[0], &[134]),
        node(1, 0, 0, &[0], &[]),
        node(11, 0, 0, &[0], &[1]),
    ];
    cases.push(("cleanup-with-loan", changed, 8));
    // An ordinary edge cannot silently forget a live owner.
    let mut changed = valid.clone();
    changed[0].nodes = forgotten_buffer_program()[0].nodes.clone();
    cases.push(("branch-forgets-owner", changed, 8));
    // A result cannot abandon a still-owned local, even with honest flags.
    let mut changed = valid.clone();
    changed[0].nodes = vec![
        node(9, 1, 0, &[0], &[]),
        node(5, 0, 0, &[0], &[6]),
        node(1, 0, 0, &[1], &[]),
        node(11, 0, 0, &[1], &[1]),
    ];
    cases.push(("return-abandons-owner", changed, 8));
    // Borrow storage outside parameters, and borrowed results, fail layout admission.
    let mut changed = valid.clone();
    changed[0].locals[0] = 134;
    cases.push(("borrow-local", changed, 5));
    let mut changed = valid.clone();
    changed[1].result = 134;
    cases.push(("borrow-result", changed, 5));
    for (name, functions, code) in cases {
        let (actual, image) = compile(&input::encode(&functions, 0).unwrap());
        assert_eq!(actual, code, "{name}: output_bytes={}", image.len());
        assert!(image.is_empty(), "{name}");
    }
    // Explicit Unit cleanup is a supported owner release, not a silent forgotten flag.
    let mut cleared = valid;
    cleared[0].nodes = vec![
        node(9, 1, 0, &[0], &[]),
        node(5, 0, 0, &[0], &[6]),
        node(3, 0, 0, &[1], &[]),
        node(5, 0, 0, &[1], &[3]),
        node(1, 0, 0, &[0], &[]),
        node(11, 0, 0, &[0], &[1]),
    ];
    cleared[0].nodes[4].immediate = 42;
    let (code, image) = compile(&input::encode(&cleared, 0).unwrap());
    assert_eq!(code, 0);
    assert_eq!(execute(&image, &[]).stdout, b"42");
}

fn record_program() -> (Vec<input::Function>, Vec<input::Record>) {
    let mut nodes = vec![
        node(1, 0, 0, &[], &[]),
        node(1, 0, 0, &[], &[1]),
        node(14, 8, 2, &[], &[1, 1]),
        node(15, 8, 0, &[], &[8]),
        node(11, 0, 0, &[], &[1]),
    ];
    nodes[0].immediate = 17;
    nodes[1].immediate = 42;
    nodes[2].data = vec![1, 0];
    (
        vec![input::Function {
            result: 1,
            nodes,
            ..input::Function::default()
        }],
        vec![input::Record {
            fields: vec![("alpha".into(), 1), ("omega".into(), 1)],
        }],
    )
}

#[test]
fn selfhost_admission_rejects_cyclic_affine_or_noncanonical_record_metadata() {
    let (functions, records) = record_program();
    let valid = input::encode_records(&functions, &records, 0).unwrap();
    let (code, image) = compile(&valid);
    assert_eq!(code, 0);
    assert_eq!(execute(&image, &[]).stdout, b"42");
    for fields in [
        vec![("alpha".into(), 8), ("omega".into(), 1)],
        vec![("alpha".into(), 6), ("omega".into(), 1)],
        vec![("alpha".into(), 134), ("omega".into(), 1)],
        vec![("alpha".into(), 0), ("omega".into(), 1)],
        vec![("omega".into(), 1), ("alpha".into(), 1)],
        vec![("alpha".into(), 1), ("alpha".into(), 1)],
        vec![("".into(), 1), ("omega".into(), 1)],
    ] {
        let malformed = input::encode_records(&functions, &[input::Record { fields }], 0).unwrap();
        assert_eq!(compile(&malformed), (5, vec![]));
    }
    // A type's metadata extent is validated before any referenced byte is read.
    let mut bad_offset = valid.clone();
    bad_offset[248..252].copy_from_slice(&u32::MAX.to_le_bytes());
    input::rechecksum(&mut bad_offset);
    assert_eq!(compile(&bad_offset), (5, vec![]));
    let metadata = u32::from_le_bytes(valid[248..252].try_into().unwrap()) as usize;
    let name = u32::from_le_bytes(valid[metadata + 4..metadata + 8].try_into().unwrap()) as usize;
    let mut invalid_name = valid.clone();
    invalid_name[name] = 0xff;
    input::rechecksum(&mut invalid_name);
    assert_eq!(compile(&invalid_name), (5, vec![]));
    for permutation in [vec![0, 0], vec![0, 2], vec![255, 0]] {
        let mut changed = functions.clone();
        changed[0].nodes[2].data = permutation;
        assert_eq!(
            compile(&input::encode_records(&changed, &records, 0).unwrap()),
            (8, vec![])
        );
    }
    let mut changed = functions;
    changed[0].nodes[3].b = 2;
    assert_eq!(
        compile(&input::encode_records(&changed, &records, 0).unwrap()),
        (8, vec![])
    );
}

#[test]
fn selfhost_admission_checks_text_utf8_before_native_emission() {
    let text_program = |data: &[u8]| {
        let mut text = node(12, 0, data.len() as u32, &[], &[]);
        text.data = data.to_vec();
        input::encode(
            &[input::Function {
                result: 1,
                nodes: vec![
                    text,
                    node(6, 0, 0, &[], &[4]),
                    node(1, 0, 0, &[], &[]),
                    node(11, 0, 0, &[], &[1]),
                ],
                ..input::Function::default()
            }],
            0,
        )
        .unwrap()
    };
    for data in [b"".as_slice(), b"\0", "猫𐀀".as_bytes()] {
        assert_eq!(compile(&text_program(data)).0, 0);
    }
    for data in [
        &[0x80][..],
        &[0xc0, 0x80],
        &[0xc1, 0xbf],
        &[0xc2],
        &[0xe0, 0x9f, 0xbf],
        &[0xed, 0xa0, 0x80],
        &[0xe1, 0x80],
        &[0xf0, 0x8f, 0xbf, 0xbf],
        &[0xf4, 0x90, 0x80, 0x80],
        &[0xf5, 0x80, 0x80, 0x80],
    ] {
        assert_eq!(compile(&text_program(data)), (8, vec![]), "{data:?}");
    }
}

#[test]
fn selfhost_admission_accepts_empty_and_maximum_width_records() {
    for count in [0, 128] {
        let mut nodes = Vec::new();
        for value in 0..count {
            let mut literal = node(1, 0, 0, &[], &vec![1; value]);
            literal.immediate = value as i64;
            nodes.push(literal);
        }
        let mut record = node(14, 8, count as u32, &[], &vec![1; count]);
        record.data = (0..count).map(|i| i as u8).collect();
        nodes.push(record);
        if count == 0 {
            nodes.push(node(6, 0, 0, &[], &[8]));
            nodes.push(node(1, 0, 0, &[], &[]));
        } else {
            nodes.push(node(15, 8, 127, &[], &[8]));
        }
        nodes.push(node(11, 0, 0, &[], &[1]));
        let payload = input::encode_records(
            &[input::Function {
                result: 1,
                nodes,
                ..input::Function::default()
            }],
            &[input::Record {
                fields: (0..count).map(|i| (format!("f{i:03}"), 1)).collect(),
            }],
            0,
        )
        .unwrap();
        let (source, program) = compiler();
        let (diagnostic, image) =
            compile_with_policy(source, program, &payload, NormalizedRunPolicy::foreground());
        assert_eq!(diagnostic, 0);
        let output = execute(&image, &[]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty());
        assert_eq!(
            output.stdout,
            if count == 0 { b"0".as_slice() } else { b"127" }
        );
    }
}
