//! Authenticated relocatable fragments. This module performs no host I/O.
use super::*;
use crate::reuse::{Keys, Records};
use crate::x86::{Fragment, FragmentReference};

pub(super) struct Context<'a> {
    pub namespace: &'a str,
    pub keys: &'a Keys,
    pub records: &'a mut Records,
    pub prefix: usize,
}
fn word(bytes: &mut Vec<u8>, value: usize) {
    for index in 0..8 {
        bytes.push(((value as u64) >> (index * 8)) as u8);
    }
}
fn span(bytes: &mut Vec<u8>, site: Span) {
    word(bytes, site.file);
    word(bytes, site.line);
    word(bytes, site.column);
}
fn take_word(bytes: &[u8], at: &mut usize) -> Option<usize> {
    if *at > bytes.len() || bytes.len() - *at < 8 {
        return None;
    }
    let mut value = 0u64;
    for index in 0..8 {
        value |= (bytes[*at + index] as u64) << (index * 8);
    }
    *at += 8;
    Some(value as usize)
}
fn take_span(bytes: &[u8], at: &mut usize) -> Option<Span> {
    Some(Span {
        file: take_word(bytes, at)?,
        line: take_word(bytes, at)?,
        column: take_word(bytes, at)?,
        start: 0,
        end: 0,
    })
}
fn encode(fragment: &Fragment) -> Vec<u8> {
    let mut out = "LKJCODE2".as_bytes().to_vec();
    word(&mut out, fragment.label_base);
    span(&mut out, fragment.site);
    word(&mut out, fragment.bytes.len());
    out.extend_from_slice(&fragment.bytes);
    word(&mut out, fragment.labels.len());
    for position in &fragment.labels {
        word(&mut out, position.map_or(0, |position| position + 1));
    }
    word(&mut out, fragment.references.len());
    for reference in &fragment.references {
        word(&mut out, reference.at);
        word(&mut out, reference.label);
        span(&mut out, reference.site);
    }
    word(&mut out, fragment.symbols.len());
    for (label, bytes) in &fragment.symbols {
        word(&mut out, *label);
        word(&mut out, bytes.len());
        out.extend_from_slice(bytes);
    }
    word(&mut out, fragment.kinds.len());
    for (offset, kind) in &fragment.kinds {
        word(&mut out, *offset);
        word(&mut out, *kind as usize);
    }
    out
}
fn decode(bytes: &[u8]) -> Option<Fragment> {
    if bytes.len() < 8 || &bytes[..8] != b"LKJCODE2" {
        return None;
    }
    let mut at = 8;
    let label_base = take_word(bytes, &mut at)?;
    let site = take_span(bytes, &mut at)?;
    let length = take_word(bytes, &mut at)?;
    if length > bytes.len() - at {
        return None;
    }
    let code = bytes[at..at + length].to_vec();
    at += length;
    let count = take_word(bytes, &mut at)?;
    if count > (bytes.len() - at) / 8 {
        return None;
    }
    let mut labels = Vec::new();
    for _ in 0..count {
        let position = take_word(bytes, &mut at)?;
        labels.push(if position == 0 { None } else { Some(position - 1) });
    }
    let count = take_word(bytes, &mut at)?;
    if count > (bytes.len() - at) / 40 {
        return None;
    }
    let mut references = Vec::new();
    for _ in 0..count {
        references.push(FragmentReference {
            at: take_word(bytes, &mut at)?,
            label: take_word(bytes, &mut at)?,
            site: take_span(bytes, &mut at)?,
        });
    }
    let count = take_word(bytes, &mut at)?;
    if count > (bytes.len() - at) / 16 {
        return None;
    }
    let mut symbols = Vec::new();
    for _ in 0..count {
        let label = take_word(bytes, &mut at)?;
        let length = take_word(bytes, &mut at)?;
        if length > bytes.len() - at {
            return None;
        }
        symbols.push((label, bytes[at..at + length].to_vec()));
        at += length;
    }
    let count = take_word(bytes, &mut at)?;
    if count > (bytes.len() - at) / 16 {
        return None;
    }
    let mut kinds = Vec::new();
    for _ in 0..count {
        let offset = take_word(bytes, &mut at)?;
        let kind = take_word(bytes, &mut at)?;
        if kind >= KIND_COUNT { return None; }
        kinds.push((offset, kind as u8));
    }
    if at != bytes.len() {
        return None;
    }
    Some(Fragment { bytes: code, label_base, labels, references, symbols, site, kinds })
}

pub(super) fn namespace(
    checked: &crate::reuse::Checked,
    runtime: &Runtime,
    grants: &[ir::EntryGrant],
    profile_path: Option<&str>,
) -> String {
    let mut inputs = "lkjscript-native-functions-2\0linux-x86_64\0".as_bytes().to_vec();
    // Grant values and the selected entry do not occur in function bodies, but
    // grant count and profiler presence change the external label prefix.
    word(&mut inputs, grants.len());
    word(&mut inputs, if profile_path.is_some() { 1 } else { 0 });
    let runtime_ir = crate::ir_text::write(runtime.checked_program());
    word(&mut inputs, runtime_ir.len());
    inputs.extend_from_slice(runtime_ir.as_bytes());
    let messages = super::messages(checked.program());
    word(&mut inputs, messages.len());
    for message in messages {
        word(&mut inputs, message.len());
        inputs.extend_from_slice(message.as_bytes());
    }
    format!("native-{}", crate::sha256::hex_digest(&inputs))
}

pub(super) fn emit(
    a: &mut Assembler,
    p: platform::Platform,
    selection: &Selection<'_>,
    index: usize,
    needed: &mut Vec<bool>,
    needed_drops: &mut Vec<bool>,
    literals: &mut BTreeMap<Vec<u8>, Label>,
    cache: &mut Context<'_>,
) -> Result<(), Diagnostic> {
    let key = cache.keys.function(index);
    if let Some(payload) = cache.records.get(cache.namespace, key) {
        if let Some(fragment) = decode(&payload) {
            if a.append_fragment(&fragment, selection.functions[index], cache.prefix, literals) {
                for reference in &fragment.references {
                    if !selection.functions.is_empty() {
                        let start = selection.functions[0];
                        if reference.label >= start && reference.label - start < needed.len() {
                            needed[reference.label - start] = true;
                        }
                    }
                    if !selection.drops.is_empty() {
                        let start = selection.drops[0];
                        if reference.label >= start && reference.label - start < needed_drops.len() {
                            needed_drops[reference.label - start] = true;
                        }
                    }
                }
                return Ok(());
            }
        }
    }
    let checkpoint = a.checkpoint();
    super::emit_function(a, p, selection, index, needed, needed_drops, literals)?;
    let mut symbols: BTreeMap<Label, Vec<u8>> = BTreeMap::new();
    for instruction in &selection.program.functions[index].body {
        if let Op::Text { bytes, .. } = &instruction.op {
            if let Some(label) = literals.get(bytes) {
                symbols.insert(*label, bytes.clone());
            }
        }
    }
    let mut listed = Vec::new();
    for (label, bytes) in symbols {
        listed.push((label, bytes));
    }
    if let Some(fragment) = a.fragment(checkpoint, listed) {
        cache.records.put(cache.namespace, key, &encode(&fragment));
    }
    Ok(())
}


fn fixture(padding: usize, existing: bool) -> (Assembler, Label, Label, BTreeMap<Vec<u8>, Label>) {
    let mut a = Assembler::new();
    a.enable_profile();
    let target = a.label();
    let bound = a.label();
    a.bind(target);
    a.ret();
    for _ in 0..padding {
        a.label();
        a.raw(&[0x90]);
    }
    let mut symbols = BTreeMap::new();
    if existing {
        let label = a.label();
        symbols.insert("reused literal".as_bytes().to_vec(), label);
    }
    (a, target, bound, symbols)
}
fn fixture_body(
    a: &mut Assembler, target: Label, bound: Label,
    symbols: &mut BTreeMap<Vec<u8>, Label>,
) -> Fragment {
    let checkpoint = a.checkpoint();
    a.location(3, 17, 9);
    a.bind(bound);
    let first_bytes = "reused literal".as_bytes().to_vec();
    let first = match symbols.get(&first_bytes) {
        Some(label) => *label,
        None => {
            let label = a.label();
            symbols.insert(first_bytes.clone(), label);
            label
        }
    };
    let done = a.label();
    a.call(target);
    a.address(A, first);
    let second_bytes = "new literal".as_bytes().to_vec();
    let second = a.label();
    symbols.insert(second_bytes.clone(), second);
    a.address(SI, second);
    a.jump(done);
    a.immediate(A, 41);
    a.bind(done);
    a.ret();
    a.fragment(checkpoint, vec![(first, first_bytes), (second, second_bytes)]).unwrap()
}
fn fixture_finish(mut a: Assembler, symbols: &BTreeMap<Vec<u8>, Label>) -> (Vec<u8>, Vec<(usize, u8)>) {
    let kinds = a.take_kinds();
    for (bytes, label) in symbols {
        a.bind(*label);
        a.raw(bytes);
    }
    (a.finish().unwrap(), kinds)
}
pub(super) fn contracts() {
    let (mut original, target, bound, mut symbols) = fixture(0, true);
    let fragment = fixture_body(&mut original, target, bound, &mut symbols);
    let encoded = encode(&fragment);
    for choice in 0..2usize {
        let padding = if choice == 0 { 0usize } else { 19usize };
        for present in 0..2usize {
            let existing = present != 0;
            let (mut cold, target, bound, mut cold_symbols) = fixture(padding, existing);
            fixture_body(&mut cold, target, bound, &mut cold_symbols);
            let expected = fixture_finish(cold, &cold_symbols);
            let (mut warm, _, bound, mut warm_symbols) = fixture(padding, existing);
            let decoded = decode(&encoded).unwrap();
            assert!(warm.append_fragment(&decoded, bound, 2, &mut warm_symbols));
            assert!(fixture_finish(warm, &warm_symbols) == expected);
        }
    }
    assert!(decode(&encoded[..0]).is_none());
    assert!(decode(&encoded[..7]).is_none());
    assert!(decode(&encoded[..encoded.len() - 1]).is_none());
    let mut broken = decode(&encoded).unwrap();
    broken.references[0].label = broken.label_base + broken.labels.len();
    let (mut a, _, bound, mut symbols) = fixture(11, false);
    let before = a.bytes.clone();
    let labels = a.label_count();
    assert!(!a.append_fragment(&broken, bound, 2, &mut symbols));
    assert!(a.bytes == before && a.label_count() == labels && symbols.is_empty());
    let mut broken = decode(&encoded).unwrap();
    broken.kinds[0].0 = 1;
    assert!(!a.append_fragment(&broken, bound, 2, &mut symbols));
    assert!(a.bytes == before && a.label_count() == labels && symbols.is_empty());
}
