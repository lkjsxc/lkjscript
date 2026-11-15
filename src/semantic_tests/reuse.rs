//! Content identities, authenticated proof reuse, and untrusted entry attacks.
use crate::{check, ir, ir_text, reuse};

fn hex(bytes: &[u8]) -> String {
    let alphabet = b"0123456789abcdef";
    let mut result = String::new();
    for byte in bytes {
        result.push(alphabet[(*byte >> 4u8) as usize] as char);
        result.push(alphabet[(*byte & 15u8) as usize] as char);
    }
    result
}
fn small() -> ir::Program {
    let mut functions = Vec::new();
    for index in 0..2 {
        functions.push(ir::Function {
            file: 0, name: if index == 0 { "probe".to_string() } else { "other".to_string() },
            params: 0, result: ir::I64, return_source: None, test: false,
            locals: vec![ir::Local { name: "value".to_string(), ty: ir::I64, mutable: false }],
            body: vec![
                ir::Instruction { file: None, line: 2 + index * 3, column: 3, op: ir::Op::Int { dst: 0, value: index as i64 } },
                ir::Instruction { file: None, line: 3 + index * 3, column: 3, op: ir::Op::Return { src: 0 } },
            ],
        });
    }
    ir::Program { types: ir::primitives(), functions, destructors: Vec::new() }
}
fn records() -> reuse::Records { reuse::Records::new(&[0x11u8; 32], &[0x22u8; 32]) }
fn replay(previous: &reuse::Records) -> reuse::Records {
    let mut records = records();
    for pending in previous.pending() {
        assert!(records.load(&pending.namespace, &pending.key, &pending.bytes));
    }
    records
}
fn hmac_standard_vectors() {
    assert_eq!(
        hex(&reuse::hmac(&[0x0bu8; 20], b"Hi There")),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
    );
    assert_eq!(
        hex(&reuse::hmac(b"Jefe", b"what do ya want for nothing?")),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
    );
    assert_eq!(
        hex(&reuse::hmac(&vec![0xaau8; 131], b"Test Using Larger Than Block-Size Key - Hash Key First")),
        "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54",
    );
}
fn authentication_binds_every_input_and_rejects_damage() {
    let producer = [0x11u8; 32];
    let secret = [0x22u8; 32];
    let payload: &[u8] = b"native bytes or a checked verdict";
    let encoded = reuse::seal(&producer, &secret, "native-linux-x86_64-default", "input-closure", payload);
    assert_eq!(
        reuse::authenticate(&producer, &secret, "native-linux-x86_64-default", "input-closure", &encoded),
        Some(payload.to_vec()),
    );
    assert!(reuse::authenticate(&[0x12u8; 32], &secret, "native-linux-x86_64-default", "input-closure", &encoded).is_none());
    assert!(reuse::authenticate(&producer, &[0x23u8; 32], "native-linux-x86_64-default", "input-closure", &encoded).is_none());
    assert!(reuse::authenticate(&producer, &secret, "check", "input-closure", &encoded).is_none());
    assert!(reuse::authenticate(&producer, &secret, "native-another-target-default", "input-closure", &encoded).is_none());
    assert!(reuse::authenticate(&producer, &secret, "native-linux-x86_64-profile", "input-closure", &encoded).is_none());
    assert!(reuse::authenticate(&producer, &secret, "native-linux-x86_64-default", "different-input", &encoded).is_none());
    for end in 0..encoded.len() {
        assert!(reuse::authenticate(&producer, &secret, "native-linux-x86_64-default", "input-closure", &encoded[..end]).is_none());
    }
    for index in 0..encoded.len() {
        let mut changed = encoded.clone();
        changed[index] ^= 1u8;
        assert!(reuse::authenticate(&producer, &secret, "native-linux-x86_64-default", "input-closure", &changed).is_none());
    }
    let mut appended = encoded.clone();
    appended.push(0u8);
    assert!(reuse::authenticate(&producer, &secret, "native-linux-x86_64-default", "input-closure", &appended).is_none());
    let mut hostile_length = encoded.clone();
    for index in 18..26 { hostile_length[index] = 255u8; }
    assert!(reuse::authenticate(&producer, &secret, "native-linux-x86_64-default", "input-closure", &hostile_length).is_none());
    assert!(reuse::seal(&producer, &[1u8; 31], "check", "input", b"ok").is_empty());
    assert!(reuse::authenticate(&producer, &[1u8; 31], "check", "input", &encoded).is_none());
}
fn streaming_authentication_matches_materialized_records() {
    let producer = [0x11u8; 32];
    let secret = [0x22u8; 32];
    let payload: &[u8] = b"the whole payload, divided at every possible boundary";
    let encoded = reuse::seal(&producer, &secret, "test", "key", payload);
    let header = reuse::header_size();
    assert_eq!(reuse::payload_length(&encoded[..header]), Some(payload.len()));
    assert!(reuse::payload_length(&encoded[..header - 1]).is_none());
    assert!(reuse::payload_length(&encoded[..header + 1]).is_none());
    let supplied = &encoded[header + payload.len()..];
    for split in 0..payload.len() + 1 {
        let mut verifier = reuse::Verifier::new(&producer, &secret, "test", "key", payload.len()).unwrap();
        assert!(verifier.update(&payload[..split]));
        assert!(verifier.update(&payload[split..]));
        assert!(verifier.finish(supplied));
    }
    let mut incomplete = reuse::Verifier::new(&producer, &secret, "test", "key", payload.len()).unwrap();
    assert!(incomplete.update(&payload[..payload.len() - 1]));
    assert!(!incomplete.finish(supplied));
    let mut excess = reuse::Verifier::new(&producer, &secret, "test", "key", payload.len() - 1).unwrap();
    assert!(!excess.update(payload));
    assert!(!excess.finish(supplied));
    let mut wrong = reuse::Verifier::new(&producer, &secret, "test", "another key", payload.len()).unwrap();
    assert!(wrong.update(payload));
    assert!(!wrong.finish(supplied));
    let empty = reuse::seal(&producer, &secret, "test", "empty", b"");
    let verifier = reuse::Verifier::new(&producer, &secret, "test", "empty", 0usize).unwrap();
    assert!(verifier.finish(&empty[header..]));
}
fn identities_track_body_and_shared_contracts() {
    let original = small();
    let keys = reuse::Keys::new(&original);
    let text = ir_text::write(&original);
    assert_eq!(text, ir_text::write(&ir_text::read(&text).unwrap()));
    let mut changed = original.clone();
    changed.functions[1].body[0].op = ir::Op::Int { dst: 0, value: 7 };
    let body = reuse::Keys::new(&changed);
    assert_eq!(keys.graph(), body.graph());
    assert_eq!(keys.function(0), body.function(0));
    assert!(keys.function(1) != body.function(1));
    assert!(keys.program() != body.program());
    changed = original.clone();
    changed.functions[1].body[0].line += 1;
    let location = reuse::Keys::new(&changed);
    assert_eq!(keys.graph(), location.graph());
    assert!(keys.function(1) != location.function(1));
    changed = original.clone();
    changed.functions[1].locals[0].name = "renamed".to_string();
    let local = reuse::Keys::new(&changed);
    assert_eq!(keys.graph(), local.graph());
    assert!(keys.function(1) != local.function(1));
    changed = original.clone();
    changed.functions[1].result = ir::U64;
    let signature = reuse::Keys::new(&changed);
    assert!(keys.graph() != signature.graph());
    assert!(keys.function(0) != signature.function(0));
    changed = original.clone();
    changed.functions[1].return_source = Some(ir::ReturnSource::Static);
    assert!(keys.graph() != reuse::Keys::new(&changed).graph());
    changed = original.clone();
    changed.functions[1].return_source = Some(ir::ReturnSource::Leaves(Vec::new()));
    assert!(keys.graph() != reuse::Keys::new(&changed).graph());
    changed = original.clone();
    changed.types[ir::I64].name = "other spelling".to_string();
    assert!(keys.graph() != reuse::Keys::new(&changed).graph());
    changed = original.clone();
    changed.destructors.push((ir::I64, 1usize));
    assert!(keys.graph() != reuse::Keys::new(&changed).graph());
    changed = original.clone();
    changed.functions = vec![original.functions[1].clone(), original.functions[0].clone()];
    assert!(keys.graph() != reuse::Keys::new(&changed).graph());
}
fn cached_verdicts_reuse_only_unchanged_bodies() {
    let original = small();
    let mut cold = records();
    let first = check::check_all_cached(reuse::Prepared::new(original.clone()), &mut cold).unwrap();
    assert_eq!(first.program(), &original);
    assert_eq!(cold.hits, 0usize);
    assert_eq!(cold.misses, 2usize);
    assert_eq!(cold.writes, 2usize);
    let mut warm = replay(&cold);
    let second = check::check_all_cached(reuse::Prepared::new(original.clone()), &mut warm).unwrap();
    assert_eq!(second.program(), first.program());
    assert_eq!(second.keys().unwrap().program(), first.keys().unwrap().program());
    assert_eq!(warm.hits, 2usize);
    assert_eq!(warm.misses, 0usize);
    assert_eq!(warm.writes, 0usize);
    let mut changed = original;
    changed.functions[1].body[0].op = ir::Op::Int { dst: 0, value: 7 };
    let mut partial = replay(&cold);
    assert!(check::check_all_cached(reuse::Prepared::new(changed.clone()), &mut partial).is_ok());
    assert_eq!(partial.hits, 1usize);
    assert_eq!(partial.misses, 1usize);
    assert_eq!(partial.writes, 1usize);
    changed.functions[1].body[1].op = ir::Op::Return { src: 10 };
    let expected = check::check_all(changed.clone()).err().unwrap();
    let mut attacked = replay(&cold);
    let actual = check::check_all_cached(reuse::Prepared::new(changed), &mut attacked).err().unwrap();
    assert_eq!(actual, expected);
    assert_eq!(attacked.hits, 1usize);
    assert_eq!(attacked.misses, 1usize);
}
fn shared_shape_is_checked_before_any_body_reuse() {
    let mut malformed = small();
    malformed.functions[0].params = 2;
    let expected = check::check_all(malformed.clone()).err().unwrap();
    let prepared = reuse::Prepared::new(malformed);
    let mut known = records();
    for key in prepared.keys().all() { known.put("check", key, b"ok"); }
    let actual = check::check_all_cached(prepared, &mut known).err().unwrap();
    assert_eq!(actual, expected);
    assert_eq!(known.hits, 0usize);
    assert_eq!(known.misses, 0usize);
}
fn malformed_payloads_and_other_domains_do_not_admit_bodies() {
    let mut original = small();
    original.functions[0].body[1].op = ir::Op::Return { src: 4 };
    let expected = check::check_all(original.clone()).err().unwrap();
    let prepared = reuse::Prepared::new(original);
    let mut known = records();
    known.put("check", prepared.keys().function(0), b"not a successful verdict");
    known.put("native", prepared.keys().function(0), b"ok");
    let actual = check::check_all_cached(prepared, &mut known).err().unwrap();
    assert_eq!(actual, expected);
    let mut empty = reuse::Records::disabled();
    empty.put("check", "anything", b"ok");
    assert!(empty.get("check", "anything").is_none());
    assert!(!empty.load("check", "anything", b"forged"));
    assert!(empty.pending().is_empty());
    assert_eq!(empty.writes, 0usize);
    let mut damaged = records();
    assert!(!damaged.load("check", "anything", b"forged"));
    assert_eq!(damaged.rejected, 1usize);
    assert!(damaged.get("check", "anything").is_none());
}
fn authenticated_packs_bind_the_graph_and_reuse_changed_bodies() {
    let producer = [0x11u8; 32]; let secret = [0x22u8; 32];
    let program = small(); let prepared = reuse::Prepared::new(program.clone());
    let group = prepared.keys().graph().to_string();
    let mut cold = reuse::Records::packed(&producer, &secret, "check", &group);
    assert!(check::check_all_cached(prepared, &mut cold).is_ok());
    assert_eq!(cold.writes, 2usize);
    assert!(cold.pending().is_empty());
    let pack = cold.sealed_pack().unwrap();
    let mut warm = reuse::Records::packed(&producer, &secret, "check", &group);
    assert!(warm.load_pack(&pack.bytes));
    assert!(check::check_all_cached(reuse::Prepared::new(program.clone()), &mut warm).is_ok());
    assert_eq!(warm.hits, 2usize); assert!(warm.sealed_pack().is_none());
    let mut changed = program;
    changed.functions[0].body[0].op = ir::Op::Int { dst: 0, value: 9 };
    assert!(check::check_all_cached(reuse::Prepared::new(changed), &mut warm).is_ok());
    assert_eq!(warm.writes, 1usize); assert!(warm.sealed_pack().is_some());
    let mut wrong = reuse::Records::packed(&producer, &secret, "check", "other graph");
    assert!(!wrong.load_pack(&pack.bytes));
    let mut wrong = reuse::Records::packed(&producer, &secret, "native", &group);
    assert!(!wrong.load_pack(&pack.bytes));
    for end in 0..pack.bytes.len() {
        let mut reader = reuse::Records::packed(&producer, &secret, "check", &group);
        assert!(!reader.load_pack(&pack.bytes[..end]));
    }
    let mut damaged = pack.bytes.clone();
    damaged[reuse::header_size() + 8] ^= 255u8;
    assert!(!warm.load_pack(&damaged));
    let mut payload = reuse::authenticate(&producer, &secret, "check", &group, &pack.bytes).unwrap();
    for index in 8..16 { payload[index] = 255u8; }
    let forged_count = reuse::seal(&producer, &secret, "check", &group, &payload);
    assert!(!warm.load_pack(&forged_count));
}
pub fn run(world: &mut World, trace: bool) -> usize {
    if trace { world.stdout().write(b"reuse_hmac_standard_vectors\n"); }
    hmac_standard_vectors();
    if trace { world.stdout().write(b"reuse_authenticated_domains_and_damage\n"); }
    authentication_binds_every_input_and_rejects_damage();
    if trace { world.stdout().write(b"reuse_streamed_authentication\n"); }
    streaming_authentication_matches_materialized_records();
    if trace { world.stdout().write(b"reuse_body_and_shared_contract_identity\n"); }
    identities_track_body_and_shared_contracts();
    if trace { world.stdout().write(b"reuse_cached_verdicts_and_changed_body\n"); }
    cached_verdicts_reuse_only_unchanged_bodies();
    if trace { world.stdout().write(b"reuse_shared_shape_always_checked\n"); }
    shared_shape_is_checked_before_any_body_reuse();
    if trace { world.stdout().write(b"reuse_malformed_payload_and_domain\n"); }
    malformed_payloads_and_other_domains_do_not_admit_bodies();
    if trace { world.stdout().write(b"reuse_native_fragment_validation\n"); }
    crate::native::cache_contracts();
    if trace { world.stdout().write(b"reuse_authenticated_packs\n"); }
    authenticated_packs_bind_the_graph_and_reuse_changed_bodies();
    9usize
}
