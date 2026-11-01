//! Output-only custody/IR probes. These never supply host-derived facts to a compiler.
use super::*;

struct CustodyProbes {
    source: KernelSnapshot,
    program: NormalizedProgram,
    images: [Vec<u8>; 2],
    source_carrier_sha256: String,
}

fn probe_program() -> &'static CustodyProbes {
    static PROBE: OnceLock<CustodyProbes> = OnceLock::new();
    PROBE.get_or_init(|| {
        let changes = proposals::updates(include_str!(
            "../../../../tests/fixtures/selfhost-source-custody-probes.lkjc"
        ))
        .unwrap();
        let source = apply_proposals(&compiler().0, changes);
        let carrier_sha256 = sha256(&graph::packet(&source));
        let program = prepare_snapshot(&source);
        let images = ["custody-evidence", "custody-input"].map(|name| {
            let entry = declaration_reference(&source, name);
            crate::platform::compiler::elf_x86_64::compile_function(
                &program,
                program.function(entry).unwrap().0 as usize,
            )
            .unwrap()
            .0
        });
        CustodyProbes {
            source,
            program,
            images,
            source_carrier_sha256: carrier_sha256,
        }
    })
}

fn probe(case: &Case, name: &str) -> Vec<u8> {
    case.assert_e1();
    let probes = probe_program();
    let source = &probes.source;
    let program = &probes.program;
    let index = match name {
        "custody-evidence" => 0,
        "custody-input" => 1,
        _ => panic!("unknown custody probe"),
    };
    let entry = declaration_reference(source, name);
    let arguments = vec![NormalizedValue::bytes(case.request.as_slice())];
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::for_test(program, policy)
        .invoke(entry, arguments.clone(), None, &control)
        .unwrap()
        .0;
    let reference = NormalizedReferenceInterpreter::new(source, program, policy)
        .invoke(entry, arguments, None, &control)
        .unwrap()
        .0;
    assert_eq!(vm, reference, "{name}");
    assert_eq!(
        super::super::super::super::field(&vm, "diagnostic"),
        &NormalizedValue::I64(0)
    );
    let NormalizedValue::Bytes(expected) = super::super::super::super::field(&vm, "image") else {
        panic!("custody probe result");
    };
    let native =
        native_filter_tests::execute_filter(&probes.images[index], &case.request, &[], 127);
    assert_eq!(native.status.code(), Some(0), "{name}: {:?}", native.stderr);
    assert!(native.stderr.is_empty());
    assert_eq!(native.stdout.as_slice(), expected.as_ref());
    eprintln!(
        "source custody output probe={name} bootstrap=rust_elf compiler_carrier_sha256={} executable_sha256={} \
         request_sha256={} carrier_sha256={} output_sha256={} output_bytes={}",
        probes.source_carrier_sha256,
        sha256(&probes.images[index]),
        case.request_sha256,
        case.carrier_sha256,
        sha256(&native.stdout),
        native.stdout.len(),
    );
    native.stdout
}

fn word(bytes: &[u8], at: usize) -> u64 {
    let end = at.checked_add(8).unwrap();
    u64::from_le_bytes(bytes.get(at..end).unwrap().try_into().unwrap())
}

fn size_word(bytes: &[u8], at: usize) -> usize {
    usize::try_from(word(bytes, at)).unwrap()
}

fn source_row(carrier: &[u8], key: OwnerKey) -> u64 {
    let count = u32::from_le_bytes(carrier[12..16].try_into().unwrap()) as usize;
    let start = u32::from_le_bytes(carrier[80..84].try_into().unwrap()) as usize;
    let key = kernel::owner_map_key(key);
    let matches = (0..count)
        .filter(|row| {
            let at = start + *row * 50;
            carrier.get(at..at + 17) == Some(key.as_slice())
        })
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1);
    matches[0] as u64
}

fn unique_binding(source: &KernelSnapshot, name: &str) -> OwnerKey {
    let matches = source
        .owners
        .iter()
        .filter_map(|(key, record)| match record {
            OwnerRecord::Binding(binding) if binding.name.as_str() == name => Some(*key),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1, "binding {name}");
    matches[0]
}

#[derive(Debug)]
struct Slot {
    owner: u64,
    category: u64,
    live: u64,
    loans: u64,
}

#[derive(Debug)]
struct Event {
    tag: u64,
    anchor: u64,
    local: u64,
    detail: u64,
    state: Vec<Slot>,
}

fn events(bytes: &[u8], owners: u64) -> Vec<Event> {
    let mut at = 0usize;
    let mut result = Vec::new();
    while at < bytes.len() {
        let state_bytes = size_word(bytes, at.checked_add(32).unwrap());
        assert_eq!(state_bytes % 32, 0);
        let start = at.checked_add(40).unwrap();
        let end = start.checked_add(state_bytes).unwrap();
        let state_bytes = bytes.get(start..end).unwrap();
        let state = state_bytes
            .as_chunks::<32>()
            .0
            .iter()
            .map(|row| {
                let slot = Slot {
                    owner: word(row, 0),
                    category: word(row, 8),
                    live: word(row, 16),
                    loans: word(row, 24),
                };
                assert!(slot.owner < owners);
                assert!(matches!(slot.category, 1 | 2));
                assert!(slot.live <= 1);
                assert!(slot.loans <= i64::MAX as u64);
                slot
            })
            .collect::<Vec<_>>();
        let distinct = state
            .iter()
            .map(|slot| slot.owner)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(distinct.len(), state.len());
        let event = Event {
            tag: word(bytes, at),
            anchor: word(bytes, at + 8),
            local: word(bytes, at + 16),
            detail: word(bytes, at + 24),
            state,
        };
        assert!(event.anchor < owners);
        if matches!(event.tag, 3 | 4 | 5 | 6 | 10 | 13) {
            assert!(event.local < owners);
        }
        result.push(event);
        at = end;
    }
    assert_eq!(at, bytes.len());
    result
}

#[test]
fn selfhost_source_custody_evidence_derives_reverse_cleanup_and_branch_repairs() {
    let source = custody_fixture();
    let case = &custody_cases()[0];
    let output = probe(case, "custody-evidence");
    let owners = u32::from_le_bytes(case.carrier[12..16].try_into().unwrap()) as u64;
    let events = events(&output, owners);
    let expected_cleanup = ["inner-second", "inner-first", "outer-second", "outer-first"]
        .map(|name| source_row(&case.carrier, unique_binding(source, name)));
    let actual = events
        .iter()
        .filter(|event| event.tag == 6 && expected_cleanup.contains(&event.local))
        .map(|event| {
            assert_eq!(event.detail, 1);
            assert!(event.state.iter().all(|slot| slot.owner != event.local));
            event.local
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected_cleanup);

    let selected = fixture_function(source, "pick");
    let E::If {
        when_true,
        when_false,
        ..
    } = &expression(source, selected.body).operation
    else {
        panic!("owner choice If");
    };
    let left = source_row(&case.carrier, OwnerKey::Parameter(selected.parameters[1]));
    let right = source_row(&case.carrier, OwnerKey::Parameter(selected.parameters[2]));
    let expected = [
        (
            source_row(&case.carrier, OwnerKey::Expression(*when_true)),
            right,
            1,
        ),
        (
            source_row(&case.carrier, OwnerKey::Expression(*when_false)),
            left,
            2,
        ),
    ]
    .into_iter()
    .collect::<std::collections::BTreeSet<_>>();
    let repairs = events
        .iter()
        .filter(|event| event.tag == 10 && [left, right].contains(&event.local))
        .map(|event| {
            let slot = event
                .state
                .iter()
                .find(|slot| slot.owner == event.local)
                .unwrap();
            assert_eq!((slot.live, slot.loans), (0, 0));
            (event.anchor, event.local, event.detail)
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(repairs, expected);

    let returns = fixture_function(source, "return-cleanup");
    let keep = source_row(&case.carrier, OwnerKey::Parameter(returns.parameters[0]));
    let spare = source_row(&case.carrier, OwnerKey::Parameter(returns.parameters[1]));
    assert!(
        events
            .iter()
            .any(|event| event.tag == 3 && event.local == keep)
    );
    assert!(
        !events
            .iter()
            .any(|event| event.tag == 6 && event.local == keep)
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.tag == 6 && event.local == spare)
            .map(|event| event.detail)
            .collect::<Vec<_>>(),
        [2]
    );
}

struct Ir<'a> {
    bytes: &'a [u8],
    functions: usize,
    function_count: usize,
    nodes: usize,
    node_count: usize,
    entry: usize,
}

impl<'a> Ir<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        assert!(bytes.len() >= 160);
        assert_eq!(&bytes[..8], b"LKJCI001");
        assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 6);
        assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 0);
        assert_eq!(size_word(bytes, 16), bytes.len());
        let mut committed = bytes[..40].to_vec();
        committed.extend_from_slice(&bytes[72..]);
        assert_eq!(&bytes[40..72], blake3::hash(&committed).as_bytes());
        let ir = Self {
            bytes,
            functions: size_word(bytes, 88),
            function_count: size_word(bytes, 96),
            nodes: size_word(bytes, 104),
            node_count: size_word(bytes, 120),
            entry: size_word(bytes, 144),
        };
        assert!(ir.entry < ir.function_count);
        let functions_end = ir
            .functions
            .checked_add(ir.function_count.checked_mul(80).unwrap())
            .unwrap();
        let nodes_end = ir
            .nodes
            .checked_add(ir.node_count.checked_mul(64).unwrap())
            .unwrap();
        assert!(functions_end <= bytes.len());
        assert!(nodes_end <= bytes.len());
        ir
    }

    fn function(&self, index: usize) -> &'a [u8] {
        assert!(index < self.function_count);
        let at = self.functions + index * 80;
        &self.bytes[at..at + 80]
    }

    fn instructions(&self, index: usize) -> Vec<&'a [u8]> {
        let function = self.function(index);
        let first = size_word(function, 32);
        let count = size_word(function, 40);
        assert!(first.checked_add(count).unwrap() <= self.node_count);
        (first..first + count)
            .map(|index| {
                let at = self.nodes + index * 64;
                &self.bytes[at..at + 64]
            })
            .collect()
    }

    fn local_types(&self, index: usize) -> Vec<(u64, u64)> {
        let function = self.function(index);
        let count = size_word(function, 24);
        let at = size_word(function, 56);
        let end = at.checked_add(count.checked_mul(16).unwrap()).unwrap();
        self.bytes
            .get(at..end)
            .unwrap()
            .as_chunks::<16>()
            .0
            .iter()
            .map(|row| (word(row, 0), word(row, 8)))
            .collect()
    }

    fn external(&self, call: &[u8]) -> u64 {
        assert!(matches!(word(call, 0), 9 | 10));
        word(self.function(size_word(call, 8)), 48)
    }
}

fn source_ir(target: &str, name: &'static str, expected: i64) -> Vec<u8> {
    let source = custody_fixture();
    let case = Case::new(name, source.clone(), 0, Some(expected)).select(target_id(source, target));
    let bytes = probe(&case, "custody-input");
    let ir = Ir::new(&bytes);
    assert!(ir.function_count > 0);
    // Run the maintained independent Profile6 validator/emitter on the output.
    // Its inputs were derived inside lkjscript from source, never by Rust.
    let (code, image) = compile(&bytes);
    assert_eq!(code, 0);
    let direct = run_filter(native_compiler(), &case, &case.request, "LKJSRQ01/profile2");
    assert_eq!(image, direct);
    bytes
}

#[test]
fn selfhost_source_custody_ir_emits_reverse_drops_and_borrow_tail_fallback() {
    let bytes = source_ir("custody-scopes", "custody-ir-scopes", 42);
    let ir = Ir::new(&bytes);
    let instructions = ir.instructions(ir.entry);
    let stores = instructions
        .iter()
        .filter(|node| word(node, 0) == 5)
        .map(|node| word(node, 8))
        .collect::<Vec<_>>();
    assert_eq!(stores.len(), 4);
    let drops = instructions
        .windows(2)
        .filter(|nodes| word(nodes[0], 0) == 16 && word(nodes[1], 0) == 6)
        .map(|nodes| word(nodes[0], 8))
        .collect::<Vec<_>>();
    assert_eq!(drops, stores.iter().rev().copied().collect::<Vec<_>>());
    let calls = instructions
        .iter()
        .filter(|node| matches!(word(node, 0), 9 | 10))
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 5);
    for call in &calls[..4] {
        assert_eq!(ir.external(call), 20);
    }
    assert_eq!(word(calls[4], 0), 9);
    assert_eq!(ir.external(calls[4]), 0);
    assert_eq!(word(instructions.last().unwrap(), 0), 11);

    let bytes = source_ir("custody-borrowed", "custody-ir-borrowed", 1);
    let ir = Ir::new(&bytes);
    let instructions = ir.instructions(ir.entry);
    let position = instructions
        .iter()
        .position(|node| matches!(word(node, 0), 9 | 10) && ir.external(node) == 22)
        .unwrap();
    let call = instructions[position];
    assert_eq!(
        word(call, 0),
        9,
        "borrow source still requires cleanup after this call"
    );
    assert!(position > 0);
    let borrowed = instructions[position - 1];
    assert_eq!((word(borrowed, 0), word(borrowed, 16)), (4, 1));
    let slot = word(borrowed, 8);
    let state = size_word(call, 48);
    let locals = size_word(ir.function(ir.entry), 24);
    assert_eq!(word(call, 56), 1);
    assert_eq!(
        (
            word(ir.bytes, state + locals),
            word(ir.bytes, state + locals + 8)
        ),
        (6, 1)
    );
    let drops = instructions[position + 1..]
        .windows(2)
        .filter(|nodes| word(nodes[0], 0) == 16 && word(nodes[1], 0) == 6)
        .map(|nodes| word(nodes[0], 8))
        .collect::<Vec<_>>();
    assert_eq!(drops, [slot]);
    assert_eq!(word(instructions.last().unwrap(), 0), 11);

    // Identify the independently authored recursive (I64, consume ByteBuffer)
    // -> ByteBuffer helper by its signature and self-targeted transfer.
    let bytes = source_ir("custody-tail", "custody-ir-tail", 42);
    let ir = Ir::new(&bytes);
    let recursive = (0..ir.function_count)
        .filter(|index| {
            let function = ir.function(*index);
            word(function, 8) == 6
                && word(function, 16) == 2
                && word(function, 48) == 0
                && ir.local_types(*index).get(..2) == Some(&[(1, 0), (6, 0)][..])
        })
        .collect::<Vec<_>>();
    assert_eq!(recursive.len(), 1);
    let index = recursive[0];
    let local_types = ir.local_types(index);
    let recursive_nodes = ir.instructions(index);
    let tails = recursive_nodes
        .iter()
        .filter(|node| word(node, 0) == 10 && size_word(node, 8) == index)
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(tails.len(), 1);
    let tail = tails[0];
    assert_eq!(word(tail, 16), 2);
    let state = size_word(tail, 48);
    for (slot, ty) in local_types.iter().enumerate() {
        if *ty == (6, 0) {
            assert_eq!(
                ir.bytes[state + slot],
                0,
                "owned slot {slot} remained live at TailCall"
            );
        }
    }
}
