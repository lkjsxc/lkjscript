//! Producer controls for the current Mode-0 callable projection.
//! These inspect normalized export without invoking the source compiler.
use super::*;
use crate::platform::execution::normalized::prepare::{
    NormalizedFieldSelector, NormalizedFunctionBody, NormalizedInstruction as I,
};
use crate::platform::kernel::{KernelSnapshot, Name};
use std::sync::Arc;

#[path = "selfhost_callable_admission_tests.rs"]
mod consumer;

fn fixture() -> &'static (KernelSnapshot, NormalizedProgram) {
    static FIXTURE: OnceLock<(KernelSnapshot, NormalizedProgram)> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let source = byte_buffer_tests::author_only(include_str!(
            "../../../../tests/fixtures/selfhost-callable-projection.lkjc"
        ))
        .unwrap();
        let program = prepare_snapshot(&source);
        (source, program)
    })
}

fn named_index(source: &KernelSnapshot, program: &NormalizedProgram, name: &str) -> usize {
    program.function(declaration_named(source, name)).unwrap().0 as usize
}

fn row(bytes: &[u8], function: usize) -> usize {
    wide_at(bytes, 88) as usize + function * input::FUNCTION_BYTES
}

fn root_row(bytes: &[u8]) -> usize {
    row(bytes, wide_at(bytes, 144) as usize)
}

fn node_row(bytes: &[u8], function_row: usize, pc: usize) -> usize {
    wide_at(bytes, 104) as usize
        + (wide_at(bytes, function_row + 32) as usize + pc) * input::NODE_BYTES
}

fn signature(bytes: &[u8], identity: u64) -> (Vec<u64>, u64) {
    assert!(identity > 7 && identity <= wide_at(bytes, 80));
    let descriptor = input::DESCRIPTORS + (identity as usize - 1) * input::DESCRIPTOR_BYTES;
    assert_eq!(wide_at(bytes, descriptor), 9);
    let arity = wide_at(bytes, descriptor + 8) as usize;
    let offset = wide_at(bytes, descriptor + 16) as usize;
    let parameters = (0..arity).map(|i| wide_at(bytes, offset + i * 8)).collect();
    (parameters, wide_at(bytes, descriptor + 24))
}

fn code(program: &NormalizedProgram, function: usize) -> &[I] {
    let NormalizedFunctionBody::Code(code) = &program.functions[function].body else {
        panic!("fixture function must have graph code")
    };
    &code.instructions
}

fn edited(
    program: &NormalizedProgram,
    function: usize,
    edit: impl FnOnce(&mut [I]),
) -> NormalizedProgram {
    let mut changed = program.clone();
    let NormalizedFunctionBody::Code(code) =
        &mut Arc::make_mut(&mut changed.functions)[function].body
    else {
        panic!("fixture function must have graph code")
    };
    edit(Arc::make_mut(&mut code.instructions));
    changed
}

#[test]
fn selfhost_profile8_producer_interns_named_closure_and_preserves_instruction_pcs() {
    let (source, program) = fixture();
    let entry = named_index(source, program, "entry");
    let bytes = input::export(program, entry).unwrap();
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 8);
    assert_eq!(wide_at(&bytes, 112), 0);
    let root = root_row(&bytes);
    assert_eq!(
        wide_at(&bytes, root + 40) as usize,
        code(program, entry).len()
    );

    let mut markers = 0;
    let mut empty_binds = 0;
    let mut partial_binds = 0;
    for (pc, instruction) in code(program, entry).iter().enumerate() {
        let at = node_row(&bytes, root, pc);
        let tag = wide_at(&bytes, at);
        match instruction {
            I::BeginBind { .. } | I::Capture { .. } => {
                markers += 1;
                assert_eq!(tag, 21);
                assert!(bytes[at + 8..at + 48].iter().all(|byte| *byte == 0));
            }
            I::FunctionValue { .. } => {
                assert_eq!(tag, 17);
                let target = row(&bytes, wide_at(&bytes, at + 8) as usize);
                assert_eq!(
                    signature(&bytes, wide_at(&bytes, target + 64)),
                    (vec![1, 1], 1)
                );
            }
            I::Bind { arguments } => {
                assert_eq!(tag, 18);
                let incoming = wide_at(&bytes, at + 8);
                let result = wide_at(&bytes, at + 24);
                if *arguments == 0 {
                    empty_binds += 1;
                    assert_eq!(result, incoming);
                } else {
                    partial_binds += 1;
                    assert_eq!(*arguments, 1);
                    assert_eq!(signature(&bytes, incoming), (vec![1, 1], 1));
                    assert_eq!(signature(&bytes, result), (vec![1], 1));
                }
            }
            I::Invoke { arguments } | I::TailInvoke { arguments } => {
                assert_eq!(
                    tag,
                    if matches!(instruction, I::TailInvoke { .. }) {
                        20
                    } else {
                        19
                    }
                );
                assert_eq!(wide_at(&bytes, at + 16), u64::from(*arguments));
                assert_eq!(signature(&bytes, wide_at(&bytes, at + 8)).1, 1);
            }
            I::Jump(target) | I::JumpIfFalse(target) => {
                assert_eq!(wide_at(&bytes, at + 8), u64::from(*target));
            }
            _ => {}
        }
    }
    assert!(markers > 0);
    assert_eq!(empty_binds, 1);
    assert_eq!(partial_binds, 1);

    // Named graph values add their reachable declarations and direct callees.
    // Every eligible declaration has exact signature metadata, not just targets
    // of FunctionValue instructions.
    for function in 0..wide_at(&bytes, 96) as usize {
        let at = row(&bytes, function);
        let (parameters, result) = signature(&bytes, wide_at(&bytes, at + 64));
        assert_eq!(result, wide_at(&bytes, at + 8));
        assert_eq!(parameters.len(), wide_at(&bytes, at + 16) as usize);
        let locals = wide_at(&bytes, at + 56) as usize;
        for (i, parameter) in parameters.iter().enumerate() {
            assert_eq!(
                *parameter,
                wide_at(&bytes, locals + i * input::VALUE_TYPE_BYTES)
            );
            assert_eq!(wide_at(&bytes, locals + i * input::VALUE_TYPE_BYTES + 8), 0);
        }
    }
}

#[test]
fn selfhost_profile8_producer_exports_function_fields_results_and_closed_generic_values() {
    let (source, program) = fixture();
    for name in ["higher", "recorded", "generic"] {
        let bytes = input::export(program, named_index(source, program, name)).unwrap();
        let descriptors = wide_at(&bytes, 80) as usize;
        let mut callable_count = 0;
        let mut record_with_callable = false;
        for identity in 8..=descriptors {
            let at = input::DESCRIPTORS + (identity - 1) * input::DESCRIPTOR_BYTES;
            match wide_at(&bytes, at) {
                9 => {
                    callable_count += 1;
                    let (parameters, result) = signature(&bytes, identity as u64);
                    assert!(result > 0 && result < identity as u64 && result != 6);
                    assert!(
                        parameters
                            .iter()
                            .all(|ty| *ty > 0 && *ty < identity as u64 && *ty != 6)
                    );
                }
                8 => {
                    let fields = wide_at(&bytes, at + 8) as usize;
                    let metadata = wide_at(&bytes, at + 16) as usize;
                    for field in 0..fields {
                        let child = wide_at(&bytes, metadata + field * 32);
                        if child > 7 {
                            let child_at =
                                input::DESCRIPTORS + (child as usize - 1) * input::DESCRIPTOR_BYTES;
                            record_with_callable |= wide_at(&bytes, child_at) == 9;
                        }
                    }
                }
                kind => panic!("unexpected compound kind {kind}"),
            }
        }
        assert!(callable_count > 0, "{name}");
        if name == "recorded" {
            assert!(record_with_callable);
        }
        if name == "generic" {
            let get = (0..wide_at(&bytes, 96) as usize)
                .map(|function| row(&bytes, function))
                .find(|at| wide_at(&bytes, at + 48) == 27)
                .unwrap();
            assert_eq!(
                signature(&bytes, wide_at(&bytes, get + 64)),
                (vec![7, 1], 1)
            );
        }
    }
    // A Function descriptor cannot be interpreted as a binary-filter record.
    let error = input::export(program, named_index(source, program, "factory")).unwrap_err();
    assert_eq!(error.code, "compiler_input_unsupported");
}

#[test]
fn selfhost_profile8_producer_rejects_bad_captures_calls_and_direct_only_values() {
    let (source, program) = fixture();
    let entry = named_index(source, program, "entry");
    let empty = program
        .function(declaration_named(source, "empty"))
        .unwrap();
    let changes = [
        edited(program, entry, |instructions| {
            let value = instructions
                .iter_mut()
                .find(|i| matches!(i, I::I64(20)))
                .unwrap();
            *value = I::Bool(true);
        }),
        edited(program, entry, |instructions| {
            let call = instructions
                .iter_mut()
                .find(|i| matches!(i, I::Invoke { .. } | I::TailInvoke { .. }))
                .unwrap();
            *call = I::Invoke { arguments: 0 };
        }),
        edited(program, entry, |instructions| {
            let target = instructions
                .iter_mut()
                .find_map(|i| match i {
                    I::FunctionValue { function, .. } => Some(function),
                    _ => None,
                })
                .unwrap();
            *target = empty;
        }),
        edited(program, entry, |instructions| {
            let bind = instructions
                .iter_mut()
                .find(|i| matches!(i, I::Bind { arguments: 0 }))
                .unwrap();
            *bind = I::Field(NormalizedFieldSelector::Structural(
                Name::new("chosen").unwrap(),
            ));
        }),
    ];
    for changed in changes {
        assert_eq!(
            input::export(&changed, entry).unwrap_err().code,
            "compiler_input_unsupported"
        );
    }
}

#[test]
fn selfhost_profile8_producer_retains_cleanup_after_a_tail_invoke_hint() {
    let (source, program) = fixture();
    let entry = named_index(source, program, "held");
    let mut invoke_pc = None;
    let changed = edited(program, entry, |instructions| {
        let (pc, arguments) = instructions
            .iter()
            .enumerate()
            .find_map(|(pc, instruction)| match instruction {
                I::Invoke { arguments } | I::TailInvoke { arguments } => Some((pc, *arguments)),
                _ => None,
            })
            .unwrap();
        instructions[pc] = I::TailInvoke { arguments };
        invoke_pc = Some(pc);
    });
    let bytes = input::export(&changed, entry).unwrap();
    let root = root_row(&bytes);
    let pc = invoke_pc.unwrap();
    let at = node_row(&bytes, root, pc);
    assert_eq!(wide_at(&bytes, at), 19);
    assert_eq!(
        wide_at(&bytes, root + 40) as usize,
        code(&changed, entry).len()
    );

    let locals = wide_at(&bytes, root + 56) as usize;
    let state = wide_at(&bytes, at + 48) as usize;
    let local_count = wide_at(&bytes, root + 24) as usize;
    let owned: Vec<_> = (0..local_count)
        .filter(|local| {
            wide_at(&bytes, locals + local * input::VALUE_TYPE_BYTES) == 6
                && wide_at(&bytes, locals + local * input::VALUE_TYPE_BYTES + 8) == 0
                && bytes[state + local] == 1
        })
        .collect();
    assert!(!owned.is_empty());
    let returned = (pc + 1..wide_at(&bytes, root + 40) as usize)
        .find(|next| wide_at(&bytes, node_row(&bytes, root, *next)) == 11)
        .unwrap();
    let return_state = wide_at(&bytes, node_row(&bytes, root, returned) + 48) as usize;
    assert!(owned.iter().all(|local| bytes[return_state + local] == 0));
    // Canonical lexical cleanup may clear a slot with Unit+StoreLocal rather
    // than emit a separate Drop. Both paths must remain after the Invoke.
    assert!((pc + 1..returned).any(|next| {
        let cleanup = node_row(&bytes, root, next);
        let tag = wide_at(&bytes, cleanup);
        let local = wide_at(&bytes, cleanup + 8) as usize;
        if !owned.contains(&local) {
            return false;
        }
        if tag == 16 {
            return true;
        }
        let depth = wide_at(&bytes, cleanup + 56) as usize;
        let before = wide_at(&bytes, cleanup + 48) as usize;
        tag == 5
            && depth > 0
            && wide_at(
                &bytes,
                before + local_count + (depth - 1) * input::VALUE_TYPE_BYTES,
            ) == 3
    }));
}

#[test]
fn selfhost_profile8_producer_keeps_wide_signatures_and_finite_policy_tags_exact() {
    let records: Vec<_> = (0..260)
        .map(|index| input::Record {
            fields: vec![(format!("field-{index}"), 1)],
        })
        .collect();
    let make_nodes = |locals: usize| {
        vec![
            input::Node {
                tag: 1,
                immediate: 42,
                initialized: vec![1; locals],
                ..input::Node::default()
            },
            input::Node {
                tag: 11,
                initialized: vec![1; locals],
                stack: vec![input::ValueType::owned(1)],
                ..input::Node::default()
            },
        ]
    };
    let functions = vec![
        input::Function {
            result: 1,
            nodes: make_nodes(0),
            ..input::Function::default()
        },
        input::Function {
            result: 1,
            parameters: 300,
            locals: vec![input::ValueType::owned(134); 300],
            nodes: make_nodes(300),
            ..input::Function::default()
        },
        input::Function {
            result: 6,
            external: 20,
            ..input::Function::default()
        },
        input::Function {
            result: 1,
            parameters: 1,
            locals: vec![input::ValueType::borrowed_buffer()],
            external: 22,
            ..input::Function::default()
        },
    ];
    let mut bytes = input::encode_records(&functions, &records, 0).unwrap();
    let zero = wide_at(&bytes, row(&bytes, 0) + 64);
    let wide = wide_at(&bytes, row(&bytes, 1) + 64);
    assert!(zero > 255 && wide > zero);
    assert_eq!(signature(&bytes, zero), (vec![], 1));
    assert_eq!(signature(&bytes, wide), (vec![134; 300], 1));
    let zero_at = input::DESCRIPTORS + (zero as usize - 1) * input::DESCRIPTOR_BYTES;
    let wide_at_offset = input::DESCRIPTORS + (wide as usize - 1) * input::DESCRIPTOR_BYTES;
    assert_eq!(
        wide_at(&bytes, zero_at + 16),
        wide_at(&bytes, wide_at_offset + 16)
    );
    assert_eq!(wide_at(&bytes, row(&bytes, 2) + 64), 0);
    assert_eq!(wide_at(&bytes, row(&bytes, 3) + 64), 0);
    for allowance in [None, Some(0), Some(u64::MAX)] {
        input::set_allowance(&mut bytes, allowance);
        assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 8);
        assert_eq!(
            u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
            u32::from(allowance.is_some())
        );
        assert_eq!(wide_at(&bytes, 32), allowance.unwrap_or(0));
        assert_eq!(wide_at(&bytes, 112), 0);
    }

    // Hostile semantic descriptors still reach the consumer fixture boundary.
    let malformed = input::encode_descriptors(
        &functions[..1],
        &[input::Descriptor::Function(input::FunctionType {
            parameters: vec![8],
            result: 0,
        })],
        0,
    )
    .unwrap();
    let descriptor = input::DESCRIPTORS + 7 * input::DESCRIPTOR_BYTES;
    assert_eq!(wide_at(&malformed, descriptor + 24), 0);
    assert_eq!(
        wide_at(&malformed, wide_at(&malformed, descriptor + 16) as usize),
        8
    );
}
