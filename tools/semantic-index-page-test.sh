#!/usr/bin/env bash
# Test-only eager publisher is the byte oracle for the production lazy publisher.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
fixture=$(mktemp -d "$PWD/target/compiler-index-page-oracle.XXXXXX")
cleanup() { local status=$?; trap - EXIT; if [[ -n ${LKJSCRIPT_INDEX_EVIDENCE:-} ]]; then mkdir -p "$LKJSCRIPT_INDEX_EVIDENCE"; cp "$fixture"/*.out "$fixture"/*.err "$LKJSCRIPT_INDEX_EVIDENCE/" 2>/dev/null || :; fi; rm -rf -- "$fixture"; exit "$status"; }
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
cp -a src library "$fixture/"
mkdir "$fixture/store"
umask 077
head -c 32 /dev/urandom > "$fixture/key"
sed '/^pub fn main(world: World)/,$d' src/main.lkj > "$fixture/src/compiler_index_page_main.lkj"
cat >> "$fixture/src/compiler_index_page_main.lkj" <<'LKJ_MAIN'
pub fn main(mut world: World) {
    if let Err(problem) = semantic_store::index_page_oracle(&mut world) {
        world.stderr().write(format!("{problem}\n").as_bytes()); world.set_exit_status(1u8);
    }
}
LKJ_MAIN
cat >> "$fixture/src/semantic_store.lkj" <<'LKJ_ORACLE'

// Frozen eager publisher exists only in this generated test fixture.
struct OracleEntry { name: String, hash: String, value: Vec<u8> }
fn oracle_node(store: &mut store::Store, entries: Vec<OracleEntry>, depth: usize, pages: &mut Vec<String>) -> Result<String, String> {
    let mut size = 32usize;
    for entry in &entries { size += entry.name.len() + entry.value.len() + 88usize; }
    let mut bytes = Vec::new();
    store::push_word(&mut bytes, 1usize);
    if entries.len() <= 64usize && size <= 32768usize {
        store::push_word(&mut bytes, 0usize);
        store::push_word(&mut bytes, depth);
        store::push_word(&mut bytes, entries.len());
        let mut ordered: BTreeMap<String, OracleEntry> = BTreeMap::new();
        for entry in entries { ordered.insert(entry.hash.clone(), entry); }
        for (_, entry) in ordered {
            store::push_bytes(&mut bytes, entry.name.as_bytes());
            store::push_bytes(&mut bytes, entry.hash.as_bytes());
            store::push_bytes(&mut bytes, &entry.value);
        }
    } else {
        if depth >= 64usize { return Err("E_QUERY_INDEX a fact exceeds its page representation".to_string()); }
        store::push_word(&mut bytes, 1usize);
        store::push_word(&mut bytes, depth);
        let mut groups: BTreeMap<u8, Vec<OracleEntry>> = BTreeMap::new();
        for entry in entries {
            let digit = entry.hash.as_bytes()[depth];
            if !groups.contains_key(&digit) { groups.insert(digit, Vec::new()); }
            groups.get_mut(&digit).unwrap().push(entry);
        }
        store::push_word(&mut bytes, groups.len());
        for (digit, group) in groups {
            store::push_word(&mut bytes, digit as usize);
            let identity = oracle_node(store, group, depth + 1usize, pages)?;
            store::push_bytes(&mut bytes, identity.as_bytes());
        }
    }
    let identity = sha256::hex_digest(&bytes);
    if store.get("semantic-page", &identity).is_none() { store.put("semantic-page", &identity, &bytes); }
    pages.push(identity.clone());
    Ok(identity)
}

fn oracle_publish(store: &mut store::Store, selector: &str, catalog: &semantic::Catalog, checked: &ir::CheckedProgram, native_sizes: &BTreeMap<String, usize>) -> Result<Snapshot, String> {
    let facts = semantic_facts::build_emitted(catalog, checked, native_sizes)?;
    let mut entries = Vec::new();
    let mut count = 0usize;
    for (name, lines) in facts.records {
        count += lines.len();
        let mut at = 0usize; let mut number = 0usize;
        loop {
            let mut end = at; let mut size = 24usize;
            while end < lines.len() && end - at < 16usize {
                let addition = lines[end].len().checked_add(8usize).ok_or("E_QUERY_INDEX fact length overflow")?;
                if end != at && size + addition > 16384usize { break; }
                if addition > 16000usize { return Err("E_QUERY_INDEX one fact is too large; source text uses chunks".to_string()); }
                size += addition; end += 1usize;
            }
            let next = if end < lines.len() { Some(number + 1usize) } else { None };
            let key = format!("{name}\t{number}");
            entries.push(OracleEntry { hash: sha256::hex_digest(key.as_bytes()), name: key, value: page(&lines[at..end], next) });
            at = end; number += 1usize;
            if next.is_none() { break; }
        }
    }
    let mut pages = Vec::new();
    let root = oracle_node(store, entries, 0usize, &mut pages)?;
    let checked_ir = sha256::hex_digest(&ir_binary::write(checked.program()));
    let bytes = snapshot_bytes(&store.query_producer(), &catalog.input_digest, &checked_ir, &root, selector, count);
    let snapshot = decode_snapshot(&bytes).ok_or("E_QUERY_INDEX could not form snapshot")?;
    store.put("semantic-root", &snapshot.identity, &bytes);
    if !store.publish_query_locator(&snapshot.identity, selector, &root) {
        store.invalidate_query_snapshot(&snapshot.identity, selector);
        return Err("E_QUERY_INDEX store refused authenticated snapshot locator publication".to_string());
    }
    store.put("semantic-latest", selector, &bytes);
    // Publishing can be refused by the disposable store's budget or lock. Never
    // print a usable identity until its root, locator and all pages were retained.
    if store.get("semantic-root", &snapshot.identity).is_none() || !store.query_pages_retained(&pages)
        || !store.query_locator_retained(&snapshot.identity, selector, &root) {
        store.invalidate_query_snapshot(&snapshot.identity, selector);
        return Err("E_QUERY_INDEX store refused complete snapshot publication; increase --store-bytes or retry".to_string());
    }
    Ok(snapshot)
}


fn oracle_query(store: &mut store::Store, selector: &str, snapshot: &str, kind: &str, id: &str) -> Result<Vec<String>, String> {
    let mut options = crate::semantic_query::Options::new();
    options.kind = kind.to_string(); options.identity = id.to_string();
    options.depth = 2usize; options.limit = 5usize; options.bytes = 8192usize;
    options.snapshot = Some(snapshot.to_string());
    let mut replies: Vec<String> = Vec::new();
    for _ in 0usize..256usize {
        let reply = crate::semantic_query::run(store, selector, &options)?;
        let cursor: String = {
            let at = reply.rfind("cursor=").ok_or("oracle missing continuation")? + 7usize;
            let tail = &reply[at..];
            let end = tail.find(' ').ok_or("oracle malformed continuation")?;
            tail[..end].to_string()
        };
        replies.push(reply);
        if cursor == "none" { return Ok(replies); }
        options.cursor = Some(cursor);
    }
    Err("oracle did not finish continuation".to_string())
}
fn oracle_leaf(store: &mut store::Store, lines: Vec<String>, ordinal: usize) -> Result<(), String> {
    let key = format!("type\tboundary{ordinal}\t0");
    let value = page(&lines, None);
    let size = value.len();
    let old: Vec<OracleEntry> = vec![OracleEntry { name: key.clone(), hash: sha256::hex_digest(key.as_bytes()), value }];
    let entries: Vec<Entry> = vec![Entry { name: key.clone(), hash: sha256::hex_digest(key.as_bytes()), record: 0usize,
        start: 0usize, end: lines.len(), next: None, value_bytes: size }];
    let records: Vec<Vec<String>> = vec![lines];
    let mut old_pages: Vec<String> = Vec::new(); let mut new_pages: Vec<String> = Vec::new();
    let before = oracle_node(store, old, 0usize, &mut old_pages)?;
    let after = node(store, entries, &records, 0usize, &mut new_pages)?;
    assert_eq!(before, after); assert_eq!(old_pages, new_pages);
    Ok(())
}
pub fn index_page_oracle(world: &mut World) -> Result<(), String> {
    let mut args: Vec<String> = Vec::new();
    for bytes in world.args() { args.push(String::from_utf8(bytes).map_err(|_| -> String { "invalid argument".to_string() })?); }
    if args.len() != 2usize { return Err("oracle expects STORE KEY".to_string()); }
    let mut store = store::Store::open_bounded(world, &Some(args[0usize].clone()), &Some(args[1usize].clone()), 268435456usize)
        .ok_or("oracle store unavailable")?;
    let empty: Vec<String> = Vec::new(); oracle_leaf(&mut store, empty, 0usize)?;
    let singleton: Vec<String> = vec![String::new()]; oracle_leaf(&mut store, singleton, 1usize)?;
    let mut maximum = String::new(); for _ in 0usize..15992usize { maximum.push('x'); }
    oracle_leaf(&mut store, vec![maximum], 2usize)?;
    let mut boundary: Vec<String> = Vec::new();
    for _ in 0usize..16usize { let mut line = String::new(); for _ in 0usize..507usize { line.push_str("α"); } boundary.push(line); }
    oracle_leaf(&mut store, boundary, 3usize)?;
    let mut text = String::new();
    for index in 0usize..100usize { text.push_str(&format!("pub fn f{index}() -> i64 {{ {index}i64 }}\n")); }
    text.push_str("pub fn long_signature(/*");
    for _ in 0usize..8000usize { text.push_str("αβ"); }
    text.push_str("*/) -> i64 { 42i64 }\nfn main() -> i64 { f0() + f99() + long_signature() }\n");
    let path = "oracle.lkj".to_string();
    let bundle = crate::source::SourceBundle { root: path.clone(), library: None,
        files: vec![crate::source::SourceInput { path: path.clone(), text: text.clone() }] };
    let parsed = crate::source::parse_bundle(&bundle).map_err(|problem| problem.message)?;
    let resolved = crate::resolve::resolve(&parsed).map_err(|problem| problem.message)?;
    let definitions = crate::elaborate::infer_resolved_all(&resolved).map_err(|problems| -> String { problems[0usize].message.clone() })?;
    let raw = crate::elaborate::lower_definitions_recorded_semantic(definitions, &mut None, &mut None)
        .map_err(|problems| -> String { problems[0usize].message.clone() })?;
    let checked = crate::check::check(raw).map_err(|problem| problem.message)?;
    let inputs: Vec<(String, Vec<u8>)> = vec![(path.clone(), text.as_bytes().to_vec())];
    let catalog = semantic::catalog_items(&resolved, &inputs, &checked)?;
    let sizes: BTreeMap<String, usize> = BTreeMap::new();
    let selector = selector(&path, &None, &None, true);
    let before = oracle_publish(&mut store, &selector, &catalog, &checked, &sizes)?;
    let kinds: Vec<String> = vec!["outline".to_string(), "signature".to_string(), "text".to_string(), "slice".to_string(),
        "callees".to_string(), "type".to_string(), "layout".to_string(), "instances".to_string(), "authority".to_string()];
    let ids: Vec<String> = vec!["crate".to_string(), "long_signature".to_string(), "long_signature".to_string(), "main".to_string(),
        "main".to_string(), "ir:main".to_string(), "i64".to_string(), "main".to_string(), "main".to_string()];
    let mut old_replies: Vec<Vec<String>> = Vec::new();
    for index in 0usize..kinds.len() {
        old_replies.push(oracle_query(&mut store, &selector, &before.identity, &kinds[index], &ids[index])?);
    }
    let after = publish_emitted(&mut store, &selector, &catalog, &checked, &sizes)?;
    assert_eq!(before.identity, after.identity); assert_eq!(before.root, after.root);
    assert_eq!(before.facts, after.facts); assert_eq!(before.source, after.source);
    assert_eq!(before.checked_ir, after.checked_ir); assert_eq!(before.producer, after.producer);
    let mut replies = 0usize;
    for index in 0usize..kinds.len() {
        let new_replies: Vec<String> = oracle_query(&mut store, &selector, &after.identity, &kinds[index], &ids[index])?;
        assert_eq!(old_replies[index], new_replies); replies += new_replies.len();
    }
    store.finish();
    world.stdout().write(format!("ok semantic index pages: boundaries=4 kinds={} replies={replies} facts={} root={} snapshot={} exact-payload-root-and-stdout=true\n",
        kinds.len(), after.facts, after.root, after.identity).as_bytes());
    Ok(())
}

LKJ_ORACLE
"$compiler" build "$fixture/src/compiler_index_page_main.lkj" --library-dir "$PWD/library" --no-store -o "$fixture/oracle" > "$fixture/build.out" 2> "$fixture/build.err" || { cat "$fixture/build.err" >&2; exit 1; }
"$fixture/oracle" "$fixture/store" "$fixture/key" > "$fixture/run.out" 2> "$fixture/run.err" || { cat "$fixture/run.out" "$fixture/run.err" >&2; exit 1; }
cat "$fixture/run.out"
