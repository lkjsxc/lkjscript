//! Raw profile-6 certificates are hostile input, even when their digest is valid.
use super::*;
use input::ValueType;

const fn owned(identity: u64) -> ValueType {
    ValueType::owned(identity)
}
const fn borrowed() -> ValueType {
    ValueType::borrowed_buffer()
}

fn node(tag: u64, a: u64, b: u64, initialized: &[u8], stack: &[input::ValueType]) -> input::Node {
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
            nodes: vec![node(1, 0, 0, &[], &[]), node(11, 0, 0, &[], &[owned(1)])],
            ..input::Function::default()
        },
        input::Function {
            result: 5,
            parameters: 2,
            locals: vec![owned(4), owned(5)],
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
        (5, vec![4, 6], 28, 8),
        // Bulk codes 29..=31 are current registered intrinsics. A forged
        // derive-key signature reaches their signature check, not unknown-code
        // admission. The next code remains explicitly unsupported.
        (5, vec![4, 5], 29, 8),
        (5, vec![4, 5], 30, 8),
        (5, vec![4, 5], 31, 8),
        (5, vec![4, 5], 32, 5),
    ] {
        let mut changed = functions.clone();
        changed[1].result = result;
        changed[1].parameters = locals.len() as u64;
        changed[1].locals = locals.into_iter().map(owned).collect();
        changed[1].external = external;
        let (code, image) = compile(&input::encode(&changed, 0).unwrap());
        assert_eq!(code, expected, "external code={external}, result={result}");
        assert!(image.is_empty());
    }
}

fn buffer_program() -> Vec<input::Function> {
    let mut nodes = vec![
        node(9, 1, 0, &[0], &[]),
        node(5, 0, 0, &[0], &[owned(6)]),
        node(16, 0, 0, &[1], &[]),
        node(9, 2, 1, &[0], &[owned(6)]),
        node(6, 0, 0, &[0], &[owned(5)]),
        node(1, 0, 0, &[0], &[]),
        node(11, 0, 0, &[0], &[owned(1)]),
    ];
    nodes[5].immediate = 42;
    vec![
        input::Function {
            result: 1,
            locals: vec![owned(6)],
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
            locals: vec![owned(6)],
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
                row[0].as_u64().unwrap(),
                row[1].as_u64().unwrap(),
                row[2].as_u64().unwrap(),
                &values(3),
                &row[4]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| owned(x.as_u64().unwrap()))
                    .collect::<Vec<_>>(),
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
    changed[0].nodes[3] = node(9, 2, 1, &[1], &[borrowed()]);
    cases.push(("consume-borrow", changed, 8));
    // A live loan cannot coexist with a subsequent move.
    let mut changed = valid.clone();
    changed[0].nodes = vec![
        node(9, 1, 0, &[0], &[]),
        node(5, 0, 0, &[0], &[owned(6)]),
        node(4, 0, 1, &[1], &[]),
        node(16, 0, 0, &[1], &[borrowed()]),
        node(6, 0, 0, &[0], &[borrowed(), owned(6)]),
        node(6, 0, 0, &[0], &[borrowed()]),
        node(1, 0, 0, &[0], &[]),
        node(11, 0, 0, &[0], &[owned(1)]),
    ];
    cases.push(("move-with-loan", changed, 8));
    // Explicit lexical cleanup must also respect a live loan.
    let mut changed = valid.clone();
    changed[0].nodes = vec![
        node(9, 1, 0, &[0], &[]),
        node(5, 0, 0, &[0], &[owned(6)]),
        node(4, 0, 1, &[1], &[]),
        node(3, 0, 0, &[1], &[borrowed()]),
        node(5, 0, 0, &[1], &[borrowed(), owned(3)]),
        node(6, 0, 0, &[0], &[borrowed()]),
        node(1, 0, 0, &[0], &[]),
        node(11, 0, 0, &[0], &[owned(1)]),
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
        node(5, 0, 0, &[0], &[owned(6)]),
        node(1, 0, 0, &[1], &[]),
        node(11, 0, 0, &[1], &[owned(1)]),
    ];
    cases.push(("return-abandons-owner", changed, 8));
    // Borrow storage outside parameters and undeclared result identities fail admission.
    let mut changed = valid.clone();
    changed[0].locals[0] = borrowed();
    cases.push(("borrow-local", changed, 5));
    let mut changed = valid.clone();
    changed[1].result = u64::MAX;
    cases.push(("undeclared-result", changed, 5));
    for (name, functions, code) in cases {
        let (actual, image) = compile(&input::encode(&functions, 0).unwrap());
        assert_eq!(actual, code, "{name}: output_bytes={}", image.len());
        assert!(image.is_empty(), "{name}");
    }
    // Explicit Unit cleanup is a supported owner release, not a silent forgotten flag.
    let mut cleared = valid;
    cleared[0].nodes = vec![
        node(9, 1, 0, &[0], &[]),
        node(5, 0, 0, &[0], &[owned(6)]),
        node(3, 0, 0, &[1], &[]),
        node(5, 0, 0, &[1], &[owned(3)]),
        node(1, 0, 0, &[0], &[]),
        node(11, 0, 0, &[0], &[owned(1)]),
    ];
    cleared[0].nodes[4].immediate = 42;
    let (code, image) = compile(&input::encode(&cleared, 0).unwrap());
    assert_eq!(code, 0);
    assert_eq!(execute(&image, &[]).stdout, b"42");
}

fn record_program() -> (Vec<input::Function>, Vec<input::Record>) {
    let mut nodes = vec![
        node(1, 0, 0, &[], &[]),
        node(1, 0, 0, &[], &[owned(1)]),
        node(14, 8, 2, &[], &[owned(1), owned(1)]),
        node(15, 8, 0, &[], &[owned(8)]),
        node(11, 0, 0, &[], &[owned(1)]),
    ];
    nodes[0].immediate = 17;
    nodes[1].immediate = 42;
    nodes[2].data = [1u64, 0].into_iter().flat_map(u64::to_le_bytes).collect();
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
    bad_offset[400..408].copy_from_slice(&u64::MAX.to_le_bytes());
    input::rechecksum(&mut bad_offset);
    assert_eq!(compile(&bad_offset), (5, vec![]));
    let metadata = u64::from_le_bytes(valid[400..408].try_into().unwrap()) as usize;
    let name = u64::from_le_bytes(valid[metadata + 8..metadata + 16].try_into().unwrap()) as usize;
    let mut invalid_name = valid.clone();
    invalid_name[name] = 0xff;
    input::rechecksum(&mut invalid_name);
    assert_eq!(compile(&invalid_name), (5, vec![]));
    for permutation in [vec![0u64, 0], vec![0, 2], vec![u64::MAX, 0]] {
        let mut changed = functions.clone();
        changed[0].nodes[2].data = permutation.into_iter().flat_map(u64::to_le_bytes).collect();
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
        let mut text = node(12, 0, data.len() as u64, &[], &[]);
        text.data = data.to_vec();
        input::encode(
            &[input::Function {
                result: 1,
                nodes: vec![
                    text,
                    node(6, 0, 0, &[], &[owned(4)]),
                    node(1, 0, 0, &[], &[]),
                    node(11, 0, 0, &[], &[owned(1)]),
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
fn selfhost_admission_accepts_empty_and_records_above_former_stack_and_field_widths() {
    for count in [0, 129, 257] {
        let mut nodes = Vec::new();
        for value in 0..count {
            let mut literal = node(1, 0, 0, &[], &vec![owned(1); value]);
            literal.immediate = value as i64;
            nodes.push(literal);
        }
        let mut record = node(14, 8, count as u64, &[], &vec![owned(1); count]);
        record.data = (0..count).flat_map(|i| (i as u64).to_le_bytes()).collect();
        nodes.push(record);
        if count == 0 {
            nodes.push(node(6, 0, 0, &[], &[owned(8)]));
            nodes.push(node(1, 0, 0, &[], &[]));
        } else {
            nodes.push(node(15, 8, (count - 1) as u64, &[], &[owned(8)]));
        }
        nodes.push(node(11, 0, 0, &[], &[owned(1)]));
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
            if count == 0 {
                "0".to_owned()
            } else {
                (count - 1).to_string()
            }
            .as_bytes()
        );
    }
}

#[test]
fn selfhost_admission_separates_wide_descriptor_identity_from_buffer_borrows() {
    const RECORDS: usize = 300;
    let mut nodes = vec![node(1, 0, 0, &[], &[])];
    nodes[0].immediate = 42;
    let mut records = Vec::new();
    let mut value_type = 1;
    for index in 0..RECORDS {
        let identity = 8 + index as u64;
        records.push(input::Record {
            fields: vec![("item".into(), value_type)],
        });
        let mut construct = node(14, identity, 1, &[], &[owned(value_type)]);
        construct.data = 0u64.to_le_bytes().to_vec();
        nodes.push(construct);
        value_type = identity;
    }
    for identity in (8..8 + RECORDS as u64).rev() {
        nodes.push(node(15, identity, 0, &[], &[owned(identity)]));
    }
    nodes.push(node(11, 0, 0, &[], &[owned(1)]));
    let functions = vec![input::Function {
        result: 1,
        nodes,
        ..input::Function::default()
    }];
    let bytes = input::encode_records(&functions, &records, 0).unwrap();
    assert_eq!(wide_at(&bytes, 80), 7 + RECORDS as u64);
    assert_eq!(wide_at(&bytes, 88), (160 + (7 + RECORDS) * 32) as u64);
    let (code, image) = compile(&bytes);
    assert_eq!(code, 0);
    let output = execute(&image, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"42");
    assert!(output.stderr.is_empty());

    // Identity 134 is admitted as an ordinary record. Setting its separate
    // qualifier to borrowed is rejected; only ByteBuffer may carry that qualifier.
    let mut invalid_borrow = functions.clone();
    let state = invalid_borrow[0]
        .nodes
        .iter_mut()
        .flat_map(|node| node.stack.iter_mut())
        .find(|ty| ty.identity == 134)
        .unwrap();
    state.borrowed = true;
    assert_eq!(
        compile(&input::encode_records(&invalid_borrow, &records, 0).unwrap()),
        (5, vec![])
    );

    // Every descriptor is checked, including a never-used trailing record.
    let mut unused_cycle = records;
    unused_cycle.push(input::Record {
        fields: vec![("cycle".into(), 8 + RECORDS as u64)],
    });
    assert_eq!(
        compile(&input::encode_records(&functions, &unused_cycle, 0).unwrap()),
        (5, vec![])
    );
}

#[test]
fn selfhost_admission_accepts_native_input_above_historical_eight_mebibytes() {
    let mut text = node(12, 0, 8 * 1024 * 1024 + 1, &[], &[]);
    text.data = vec![b'x'; text.b as usize];
    let mut answer = node(1, 0, 0, &[], &[]);
    answer.immediate = 42;
    let bytes = input::encode(
        &[input::Function {
            result: 1,
            nodes: vec![
                text,
                node(6, 0, 0, &[], &[owned(4)]),
                answer,
                node(11, 0, 0, &[], &[owned(1)]),
            ],
            ..input::Function::default()
        }],
        0,
    )
    .unwrap();
    assert!(bytes.len() > 8 * 1024 * 1024);
    assert_eq!(wide_at(&bytes, 32), 0);
    let (source, program) = compiler();
    let entry = program
        .function(declaration_named(source, "compile-elf"))
        .unwrap()
        .0 as usize;
    let compiler = crate::platform::compiler::elf_x86_64::compile_function(program, entry)
        .unwrap()
        .0;
    // Large-pool admission runs only natively; the independent expected result
    // avoids repeated multi-megabyte VM/reference compiler execution.
    let compiled = native_filter_tests::execute_filter(&compiler, &bytes, &[], 4093);
    assert_eq!(compiled.status.code(), Some(0), "{:?}", compiled.stderr);
    assert!(compiled.stderr.is_empty());
    crate::release_container::inspect_static_elf_bytes(&compiled.stdout).unwrap();
    let output = execute(&compiled.stdout, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"42");
    assert!(output.stderr.is_empty());
}

#[test]
fn selfhost_admission_checks_every_function_above_historical_1024_rows() {
    let mut functions = vec![
        input::Function {
            result: 1,
            nodes: vec![node(1, 0, 0, &[], &[]), node(11, 0, 0, &[], &[owned(1)])],
            ..input::Function::default()
        };
        1025
    ];
    functions[0].nodes[0].immediate = 42;
    let bytes = input::encode(&functions, 0).unwrap();
    assert_eq!(wide_at(&bytes, 96), 1025);
    let (code, image) = compile(&bytes);
    assert_eq!(code, 0);
    let output = execute(&image, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"42");
    assert!(output.stderr.is_empty());
    // The last uncalled function still reaches semantic instruction admission.
    functions[1024].nodes[0].tag = 17;
    assert_eq!(compile(&input::encode(&functions, 0).unwrap()), (7, vec![]));
}
