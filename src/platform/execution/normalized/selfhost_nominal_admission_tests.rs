//! Hostile profile-9 nominal certificates and independent literal execution.
//! Register under selfhost_tests only with the complete adopted nominal batch.
use super::*;
use crate::platform::diagnostic::Diagnostic;
use crate::platform::kernel::{CaseReference, DeclarationReference, FieldReference, PackageId};
use crate::platform::semantic_id::{CaseId, DeclarationId, FieldId};

const fn owned(identity: u64) -> input::ValueType {
    input::ValueType::owned(identity)
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

fn integer(value: i64, initialized: &[u8]) -> input::Node {
    let mut result = node(1, 0, 0, initialized, &[]);
    result.immediate = value;
    result
}

#[derive(Clone, Copy, Default)]
struct Options {
    buffer: bool,
    preinitialized_payload: bool,
    tail: bool,
}

#[derive(Clone)]
struct Fixture {
    functions: Vec<input::Function>,
    descriptors: Vec<input::Descriptor>,
    constructor: usize,
    switch: usize,
    arms: Vec<usize>,
    terminal: Vec<usize>,
}

impl Fixture {
    fn encoded(&self) -> Result<Vec<u8>, Diagnostic> {
        input::encode_descriptors(&self.functions, &self.descriptors, 0)
    }

    fn arm_word(&mut self, arm: usize, column: usize, value: u64) {
        let at = arm * 32 + column * 8;
        self.functions[0].nodes[self.switch].data[at..at + 8].copy_from_slice(&value.to_le_bytes());
    }
}

fn fixture(selected: usize, options: Options) -> Fixture {
    let package = PackageId::migrate(b"nominal-flow-controls", 0);
    let record = input::NominalRecord {
        declaration: DeclarationReference {
            package,
            declaration: DeclarationId::migrate(b"nominal-flow-controls", 0),
        },
        fields: vec![(
            FieldReference {
                package,
                field: FieldId::migrate(b"nominal-flow-controls", 0),
            },
            1,
        )],
    };
    let mut case_ids: Vec<_> = (0..5)
        .map(|index| CaseId::migrate(b"nominal-flow-controls", index))
        .collect();
    case_ids.sort();
    let variant = input::NominalVariant {
        declaration: DeclarationReference {
            package,
            declaration: DeclarationId::migrate(b"nominal-flow-controls", 1),
        },
        cases: case_ids
            .into_iter()
            .zip([None, Some(3), Some(1), Some(1), Some(8)])
            .map(|(case, payload)| (CaseReference { package, case }, payload))
            .collect(),
    };
    let mut locals = vec![owned(3), owned(1), owned(1), owned(8)];
    if options.buffer {
        locals.push(owned(6));
    }
    let mut incoming = vec![0; locals.len()];
    let mut nodes = Vec::new();
    if options.buffer {
        nodes.push(node(9, 1, 0, &incoming, &[]));
        nodes.push(node(5, 4, 0, &incoming, &[owned(6)]));
        incoming[4] = 1;
    }
    if options.preinitialized_payload {
        nodes.push(integer(99, &incoming));
        nodes.push(node(5, 1, 0, &incoming, &[owned(1)]));
        incoming[1] = 1;
    }
    let payload = match selected {
        0 => None,
        1 => {
            nodes.push(node(3, 0, 0, &incoming, &[]));
            Some(3)
        }
        2 | 3 => {
            nodes.push(integer(if selected == 2 { 42 } else { 19 }, &incoming));
            Some(1)
        }
        _ => {
            nodes.push(integer(43, &incoming));
            let mut value = node(14, 8, 1, &incoming, &[owned(1)]);
            value.data.extend_from_slice(&0_u64.to_le_bytes());
            nodes.push(value);
            Some(8)
        }
    };
    let constructor = nodes.len();
    let stack: Vec<_> = payload.into_iter().map(owned).collect();
    let mut value = node(22, 9, selected as u64, &incoming, &stack);
    value.c = u64::from(payload.is_some());
    nodes.push(value);
    let switch = nodes.len();
    nodes.push(node(23, 9, 5, &incoming, &[owned(9)]));
    let mut arms = Vec::new();
    let mut terminal = Vec::new();
    let mut jumps = Vec::new();
    for case in 0..5 {
        arms.push(nodes.len());
        let mut flags = vec![0; locals.len()];
        if options.buffer {
            flags[4] = 1;
        }
        if case != 0 {
            flags[case - 1] = 1;
        }
        match case {
            0 => nodes.push(integer(7, &flags)),
            1 => {
                nodes.push(node(4, 0, 0, &flags, &[]));
                nodes.push(node(6, 0, 0, &flags, &[owned(3)]));
                nodes.push(integer(11, &flags));
            }
            2 | 3 => nodes.push(node(4, (case - 1) as u64, 0, &flags, &[])),
            _ => {
                nodes.push(node(4, 3, 0, &flags, &[]));
                nodes.push(node(15, 8, 0, &flags, &[owned(8)]));
            }
        }
        if options.tail && case == 0 {
            terminal.push(nodes.len());
            nodes.push(node(
                10,
                if options.buffer { 2 } else { 1 },
                1,
                &flags,
                &[owned(1)],
            ));
        } else {
            if options.buffer {
                nodes.push(node(16, 4, 0, &flags, &[owned(1)]));
                flags[4] = 0;
                nodes.push(node(6, 0, 0, &flags, &[owned(1), owned(6)]));
            }
            terminal.push(nodes.len());
            jumps.push(nodes.len());
            nodes.push(node(7, 0, 0, &flags, &[owned(1)]));
        }
    }
    let common = nodes.len();
    nodes.push(node(11, 0, 0, &vec![0; locals.len()], &[owned(1)]));
    for jump in jumps {
        nodes[jump].a = common as u64;
    }
    for (case, target) in arms.iter().enumerate() {
        for word in [
            case as u64,
            *target as u64,
            u64::from(case != 0),
            if case == 0 { 0 } else { (case - 1) as u64 },
        ] {
            nodes[switch].data.extend_from_slice(&word.to_le_bytes());
        }
    }
    let mut functions = vec![input::Function {
        result: 1,
        locals,
        nodes,
        ..input::Function::default()
    }];
    if options.buffer {
        functions.push(input::Function {
            result: 6,
            external: 20,
            ..input::Function::default()
        });
    }
    if options.tail {
        functions.push(input::Function {
            result: 1,
            parameters: 1,
            locals: vec![owned(1)],
            nodes: vec![node(4, 0, 0, &[1], &[]), node(11, 0, 0, &[1], &[owned(1)])],
            ..input::Function::default()
        });
    }
    Fixture {
        functions,
        descriptors: vec![
            input::Descriptor::NominalRecord(record),
            input::Descriptor::NominalVariant(variant),
        ],
        constructor,
        switch,
        arms,
        terminal,
    }
}

fn accepted(fixture: &Fixture, literal: &[u8]) -> Result<Vec<u8>, Diagnostic> {
    let (code, image) = compile(&fixture.encoded()?);
    assert_eq!(code, 0);
    let output = execute(&image, &[]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, literal);
    assert!(output.stderr.is_empty());
    Ok(image)
}

fn rejected(label: &str, fixture: &Fixture, expected: i64) -> Result<(), Diagnostic> {
    let (code, image) = compile(&fixture.encoded()?);
    assert_eq!(code, expected, "{label}");
    assert!(image.is_empty(), "{label}");
    Ok(())
}

#[test]
fn selfhost_nominal_flow_executes_every_case_against_independent_literals() -> Result<(), Diagnostic>
{
    for (case, expected) in [b"7".as_slice(), b"11", b"42", b"19", b"43"]
        .into_iter()
        .enumerate()
    {
        accepted(&fixture(case, Options::default()), expected)?;
    }
    Ok(())
}

#[test]
fn selfhost_nominal_flow_rejects_missing_sibling_reused_and_incompatible_states()
-> Result<(), Diagnostic> {
    let valid = fixture(2, Options::default());
    let mut cases = Vec::new();
    let mut changed = valid.clone();
    changed.functions[0].nodes[changed.arms[2]].initialized[1] = 0;
    cases.push(("selected payload local is missing", changed));
    let mut changed = valid.clone();
    changed.functions[0].nodes[changed.arms[2]].initialized[2] = 1;
    cases.push(("sibling payload local leaks into selected arm", changed));
    let mut changed = valid.clone();
    changed.arm_word(3, 3, 1);
    for index in changed.arms[3]..=changed.terminal[3] {
        changed.functions[0].nodes[index].initialized = vec![0, 1, 0, 0];
    }
    changed.functions[0].nodes[changed.arms[3]].a = 1;
    cases.push(("two equal-typed cases reuse one payload local", changed));
    cases.push((
        "payload local was already initialized before Switch",
        fixture(
            2,
            Options {
                preinitialized_payload: true,
                ..Options::default()
            },
        ),
    ));
    let mut changed = valid.clone();
    changed.functions[0].locals[1] = owned(2);
    cases.push(("payload local has the wrong exact type", changed));
    let mut changed = valid.clone();
    changed.functions[0].locals[1] = owned(6);
    cases.push(("copyable case cannot initialize an affine local", changed));
    let mut changed = valid.clone();
    changed.arm_word(2, 3, 4);
    cases.push(("payload local is outside the frame", changed));
    let mut changed = valid.clone();
    changed.functions[0].nodes[changed.arms[0]].tag = 2;
    changed.functions[0].nodes[changed.arms[0]].immediate = 1;
    changed.functions[0].nodes[changed.terminal[0]].stack = vec![owned(2)];
    cases.push(("untaken arm cannot join a different result stack", changed));
    let mut changed = valid.clone();
    changed.functions[0].nodes[changed.switch].b = 4;
    changed.functions[0].nodes[changed.switch]
        .data
        .truncate(4 * 32);
    cases.push(("the complete case vector is required", changed));
    let mut changed = valid.clone();
    changed.arm_word(1, 0, 0);
    cases.push((
        "case ordinals must be exactly canonical and exhaustive",
        changed,
    ));
    let mut changed = valid.clone();
    changed.arm_word(0, 1, 0);
    cases.push(("backward target with an otherwise exact edge", changed));
    let mut changed = valid.clone();
    changed.arm_word(1, 1, changed.arms[0] as u64);
    cases.push(("arm targets cannot repeat", changed));
    let mut changed = valid.clone();
    changed.arm_word(4, 1, changed.functions[0].nodes.len() as u64);
    cases.push(("arm target is outside the function", changed));
    let mut changed = valid.clone();
    changed.functions[0].parameters = 1;
    // The first ordinary edge may forget this copyable parameter. Its flag is
    // then 0 at Switch, so freshness alone cannot hide the parameter-bound rule.
    changed.functions[0].nodes[0].initialized[0] = 1;
    changed.functions.insert(
        0,
        input::Function {
            result: 1,
            nodes: vec![integer(42, &[]), node(11, 0, 0, &[], &[owned(1)])],
            ..input::Function::default()
        },
    );
    cases.push((
        "an uncalled function cannot bind a case payload into a parameter",
        changed,
    ));
    for (label, changed) in cases {
        rejected(label, &changed, 8)?;
    }
    Ok(())
}

#[test]
fn selfhost_nominal_flow_preserves_none_unit_and_qualified_identity() -> Result<(), Diagnostic> {
    let mut absent = fixture(0, Options::default());
    absent.functions[0].nodes[absent.constructor].c = 1;
    rejected("absent case cannot claim a payload", &absent, 8)?;
    let mut unit = fixture(1, Options::default());
    unit.functions[0].nodes[unit.constructor].c = 0;
    rejected("Unit is a present payload", &unit, 8)?;
    let mut unit = fixture(1, Options::default());
    unit.functions[0].nodes[0].tag = 1;
    unit.functions[0].nodes[0].immediate = 3;
    unit.functions[0].nodes[unit.constructor].stack = vec![owned(1)];
    rejected("an I64 is not a Unit payload", &unit, 8)?;
    let mut absent = fixture(2, Options::default());
    absent.arm_word(0, 3, 1);
    rejected("absent binder has the exact zero encoding", &absent, 8)?;
    let mut unit = fixture(2, Options::default());
    unit.arm_word(1, 2, 0);
    rejected("Unit case must bind its present payload", &unit, 8)?;
    let mut wrong = fixture(2, Options::default());
    let mut second = wrong.descriptors[1].clone();
    if let input::Descriptor::NominalVariant(value) = &mut second {
        value.declaration.declaration = DeclarationId::migrate(b"nominal-flow-controls", 2);
    }
    wrong.descriptors.push(second);
    wrong.functions[0].nodes[wrong.switch].a = 10;
    rejected(
        "equal shapes with different nominal identity do not match",
        &wrong,
        8,
    )?;
    Ok(())
}

#[test]
fn selfhost_nominal_flow_keeps_buffer_custody_and_tail_release_exact() -> Result<(), Diagnostic> {
    let valid = fixture(
        2,
        Options {
            buffer: true,
            ..Options::default()
        },
    );
    accepted(&valid, b"42")?;
    let mut missing = valid.clone();
    missing.functions[0].nodes[missing.arms[0]].initialized[4] = 0;
    rejected(
        "an untaken arm cannot forget a live ByteBuffer",
        &missing,
        8,
    )?;
    let live_tail = fixture(
        0,
        Options {
            buffer: true,
            tail: true,
            ..Options::default()
        },
    );
    rejected(
        "tail transfer cannot abandon a live ByteBuffer",
        &live_tail,
        8,
    )?;
    accepted(
        &fixture(
            0,
            Options {
                tail: true,
                ..Options::default()
            },
        ),
        b"7",
    )?;
    Ok(())
}

#[test]
fn selfhost_nominal_flow_keeps_shape_diagnostics_and_optional_policy_exact()
-> Result<(), Diagnostic> {
    let valid = fixture(2, Options::default());
    let mut truncated = valid.clone();
    truncated.functions[0].nodes[truncated.switch]
        .data
        .truncate(5 * 32 - 8);
    rejected("truncated arm row framing", &truncated, 5)?;
    let mut invalid_flag = valid.clone();
    invalid_flag.functions[0].nodes[invalid_flag.arms[2]].initialized[1] = 2;
    rejected("nonboolean initialized flag", &invalid_flag, 5)?;
    let mut invalid_presence = valid.clone();
    invalid_presence.functions[0].nodes[invalid_presence.constructor].c = 2;
    rejected("nonboolean constructor presence", &invalid_presence, 7)?;
    let mut unknown = valid.clone();
    unknown.functions[0].nodes[unknown.constructor].tag = 24;
    rejected(
        "the next unassigned instruction remains unsupported",
        &unknown,
        7,
    )?;
    let bytes = valid.encoded()?;
    let required =
        4096 + wide_at(&bytes, 96) + 64 * wide_at(&bytes, 120) + 256 * wide_at(&bytes, 136);
    let (baseline_code, baseline_image) = compile(&bytes);
    assert_eq!(baseline_code, 0);
    for (allowance, expected) in [
        (None, 0),
        (Some(0), 6),
        (Some(required - 1), 6),
        (Some(required), 0),
        (Some(u64::MAX), 0),
    ] {
        let mut checked = bytes.clone();
        input::set_allowance(&mut checked, allowance);
        let (code, image) = compile(&checked);
        assert_eq!(code, expected, "allowance {allowance:?}");
        if expected == 0 {
            assert_eq!(image, baseline_image, "allowance {allowance:?}");
        } else {
            assert!(image.is_empty());
        }
    }
    Ok(())
}
