//! Evaluator-hosted structural compiler reader and dependency graph.
//! Observe constructors, ordered child occurrences, reverse multiplicities, and
//! dependency-before-parent order. Hash/authority admission stays upstream.
use super::*;
use std::time::Instant;

#[path = "typed_compiler_input_fixture_tests.rs"]
mod fixture;

const TYPED_SOURCE: &str = concat!(
    include_str!("../../../../tools/selfhost/typed-input.lkjc"),
    "\n",
    include_str!("../../../../tools/selfhost/typed-dag.lkjc"),
);
fn prototype() -> &'static (KernelSnapshot, NormalizedProgram) {
    static PROTOTYPE: OnceLock<(KernelSnapshot, NormalizedProgram)> = OnceLock::new();
    PROTOTYPE.get_or_init(|| {
        let source = byte_buffer_tests::author_only(TYPED_SOURCE).unwrap();
        let program = prepare_snapshot(&source);
        (source, program)
    })
}

fn typed_function(module: &str, name: &str) -> DeclarationReference {
    let source = &prototype().0;
    let module = source
        .owners
        .iter()
        .find_map(|(key, value)| match (key, value) {
            (OwnerKey::Module(id), OwnerRecord::Module(record))
                if record.name.as_str() == module =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap();
    let declaration = source
        .owners
        .iter()
        .find_map(|(key, value)| match (key, value) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.module == module && record.name.as_str() == name =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap();
    DeclarationReference {
        package: source.root.package_id,
        declaration,
    }
}

fn invoke_observed(
    module: &str,
    name: &str,
    values: Vec<NormalizedValue>,
) -> (
    NormalizedValue,
    crate::platform::execution::normalized::NormalizedRunObservation,
) {
    let (_, program) = prototype();
    let (result, work) = NormalizedVm::for_test(program, NormalizedRunPolicy::foreground())
        .invoke(
            typed_function(module, name),
            values,
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    assert_eq!(work.live_call_frames_after, 0);
    assert_eq!(work.live_operands_after, 0);
    (result, work)
}

fn invoke_typed(name: &str, values: Vec<NormalizedValue>) -> NormalizedValue {
    invoke_observed("typed-input", name, values).0
}

fn branch(value: &NormalizedValue) -> (&str, Option<&NormalizedValue>) {
    let NormalizedValue::Variant {
        layout,
        case,
        payload,
    } = value
    else {
        panic!("expected typed reader variant");
    };
    let program = &prototype().1;
    (
        program.variants[layout.0 as usize].cases[*case as usize]
            .name
            .as_str(),
        payload.as_deref(),
    )
}

fn member<'a>(value: &'a NormalizedValue, name: &str) -> &'a NormalizedValue {
    let NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }) = value else {
        panic!("expected typed reader record");
    };
    let index = prototype().1.records[layout.0 as usize]
        .fields
        .iter()
        .position(|field| field.name.as_str() == name)
        .unwrap();
    &fields[index]
}

fn integer(value: &NormalizedValue) -> i64 {
    let NormalizedValue::I64(value) = value else {
        panic!("expected I64");
    };
    *value
}

fn octets(value: &NormalizedValue) -> &[u8] {
    let NormalizedValue::Bytes(value) = value else {
        panic!("expected Bytes");
    };
    value
}

fn table(value: &NormalizedValue) -> &crate::platform::execution::normalized::list::List {
    let (case, payload) = branch(value);
    assert_eq!(case, "ready");
    let NormalizedValue::List(entries) = payload.unwrap() else {
        panic!("expected type table");
    };
    entries
}

fn assert_error(bytes: &[u8], at: i64) {
    let result = invoke_typed("read", vec![NormalizedValue::bytes(bytes)]);
    let (case, payload) = branch(&result);
    assert_eq!(case, "error");
    let error = payload.unwrap();
    assert_eq!(integer(member(error, "at")), at);
    assert!(
        matches!(member(error, "message"), NormalizedValue::Text(message) if !message.is_empty())
    );
}

fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

fn values(value: &NormalizedValue) -> &crate::platform::execution::normalized::list::List {
    let NormalizedValue::List(values) = value else {
        panic!("expected list");
    };
    values
}

fn indices(value: &NormalizedValue) -> Vec<usize> {
    values(value)
        .iter()
        .map(|value| usize::try_from(integer(value)).unwrap())
        .collect()
}

fn edges(value: &NormalizedValue) -> fixture::Graph {
    values(value).iter().map(indices).collect()
}

fn graph_result(bytes: &[u8]) -> NormalizedValue {
    invoke_observed(
        "typed-dag",
        "read-graph",
        vec![NormalizedValue::bytes(bytes)],
    )
    .0
}

fn graph_difference(graph: &fixture::Graph, result: &NormalizedValue) -> Option<String> {
    let (case, payload) = branch(result);
    let expected = fixture::class(graph);
    if case == "error" {
        let (error, details) = branch(payload.unwrap());
        let expected_error = match expected {
            fixture::Class::Missing => "missing-child",
            fixture::Class::Cycle => "cycle",
            fixture::Class::Ready => return Some(format!("valid graph rejected: {error}")),
        };
        if error != expected_error {
            return Some(format!("expected {expected_error}, got {error}"));
        }
        if error == "missing-child" {
            let details = details.unwrap();
            let parent = usize::try_from(integer(member(details, "parent"))).unwrap();
            let position = usize::try_from(integer(member(details, "position"))).unwrap();
            if parent >= graph.len() || position >= graph[parent].len() {
                return Some("missing-child identifies a nonexistent edge occurrence".into());
            }
            let child = graph[parent][position];
            if child < graph.len()
                || octets(member(details, "digest")) != fixture::alias(child).bytes()
            {
                return Some("missing-child identifies an existing or different endpoint".into());
            }
        } else {
            let mut remaining = indices(details.unwrap());
            remaining.sort_unstable();
            if remaining != fixture::unresolved(graph) {
                return Some(format!(
                    "cycle reports different unresolved ordinals: {remaining:?}"
                ));
            }
        }
        return None;
    }
    if expected != fixture::Class::Ready {
        return Some(format!("{expected:?} graph admitted"));
    }
    let dag = payload.unwrap();
    let children = edges(member(dag, "children"));
    let mut parents = edges(member(dag, "parents"));
    for occurrences in &mut parents {
        occurrences.sort_unstable();
    }
    let order = indices(member(dag, "order"));
    if children != *graph {
        return Some(format!("children differ: {children:?}"));
    }
    if parents != fixture::parents(graph) {
        return Some(format!("reverse occurrences differ: {parents:?}"));
    }
    if order.len() != graph.len() {
        return Some(format!("order length differs: {}", order.len()));
    }
    let mut rank = vec![None; graph.len()];
    for (at, row) in order.iter().copied().enumerate() {
        if row >= graph.len() || rank[row].replace(at).is_some() {
            return Some("order repeats or invents an ordinal".into());
        }
    }
    for (parent, children) in graph.iter().enumerate() {
        for child in children {
            if rank[*child] >= rank[parent] {
                return Some(format!(
                    "dependency {child} did not precede parent {parent}"
                ));
            }
        }
    }
    None
}

fn assert_graph(graph: fixture::Graph) {
    let result = graph_result(&fixture::carrier(&fixture::rows(&graph)));
    if let Some(problem) = graph_difference(&graph, &result) {
        let reduced = fixture::reduce(graph, |candidate| {
            graph_difference(
                candidate,
                &graph_result(&fixture::carrier(&fixture::rows(candidate))),
            )
            .is_some()
        });
        panic!("{problem}; minimized structural catalogue: {reduced:?}");
    }
}

fn assert_shape(form: &kernel::TypeForm) {
    let (kind, children) = fixture::shape(form);
    let image = fixture::image(form.clone());
    let result = invoke_typed(
        "decode-image",
        vec![NormalizedValue::bytes(image.as_slice())],
    );
    let (case, payload) = branch(&result);
    assert_eq!(case, "ready", "{kind}: {image:?}");
    let decoded = payload.unwrap();
    assert_eq!(branch(member(decoded, "kind")).0, kind);
    let actual: Vec<_> = values(member(decoded, "children"))
        .iter()
        .map(|v| octets(v).to_vec())
        .collect();
    let expected: Vec<_> = children
        .iter()
        .map(|digest| digest.bytes().to_vec())
        .collect();
    assert_eq!(actual, expected, "{kind}");
}

fn assert_image_error(image: &[u8]) {
    let result = invoke_typed("decode-image", vec![NormalizedValue::bytes(image)]);
    let (case, payload) = branch(&result);
    assert_eq!(case, "error", "malformed image admitted: {image:?}");
    let error = payload.unwrap();
    assert!(integer(member(error, "at")) >= 0);
    assert!(matches!(member(error, "message"), NormalizedValue::Text(text) if !text.is_empty()));
}

#[test]
fn typed_compiler_input_structured_errors_release_partial_tables() {
    let (source, program) = prototype();
    let carrier = source_fixture::raw_object_packet(source, None, None);
    let result = invoke_typed("read", vec![NormalizedValue::bytes(carrier.as_slice())]);
    assert_eq!(table(&result).len(), source.types.len());
    let reference =
        NormalizedReferenceInterpreter::new(source, program, NormalizedRunPolicy::foreground())
            .invoke(
                typed_function("typed-input", "read"),
                vec![NormalizedValue::bytes(carrier.as_slice())],
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap();
    assert_eq!(raw_value_equal(&result, &reference.0), Ok(true));
    assert_eq!(reference.1.live_call_frames_after, 0);
    for length in [0, 1, 2, 4, 8, 16, 32, 64, 127] {
        assert_error(&carrier[..length], 0);
    }
    for (at, replacement, expected) in [
        (0, 0, 0),
        (8, 0, 8),
        (112, 9, 112),
        (84, u32::MAX, 84),
        (92, u32::MAX, 92),
        (96, u32::MAX, 96),
    ] {
        let mut broken = carrier.clone();
        broken[at..at + 4].copy_from_slice(&replacement.to_le_bytes());
        assert_error(&broken, expected);
    }
    // The final row fails after every earlier typed record has been accumulated.
    let span = u32_at(&carrier, 92) + (u32_at(&carrier, 12) + source.types.len() - 1) * 8;
    let mut broken = carrier.clone();
    broken[span + 4..span + 8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_error(&broken, span as i64);
    drop(result);
    drop(reference);
    assert_eq!(
        table(&invoke_typed("read", vec![NormalizedValue::bytes(carrier)])).len(),
        source.types.len()
    );
}

#[test]
#[ignore = "slow: maintained compiler type table and cursor comparison"]
fn typed_compiler_input_matches_maintained_cursor_and_type_objects() {
    let (source, old_program) = compiler();
    let carrier = source_fixture::packet(source);
    let argument = NormalizedValue::bytes(carrier.as_slice());
    prototype();
    let start = Instant::now();
    let typed = invoke_typed("read", vec![argument.clone()]);
    let typed_time = start.elapsed();
    let entries = table(&typed);
    assert_eq!(entries.len(), source.types.len());
    let control = ExecutionControl::uncancelled();
    let run_cursor = |module: &str, name: &str, values| {
        NormalizedVm::for_test(old_program, NormalizedRunPolicy::foreground())
            .invoke(function(module, name), values, None, &control)
            .unwrap()
            .0
    };
    let run_old = |name: &str, values| run_cursor("source-type-table", name, values);
    let start = Instant::now();
    assert_eq!(
        integer(&run_old("count", vec![argument.clone()])),
        entries.len() as i64
    );
    let old_images: Vec<_> = (0..entries.len())
        .map(|row| {
            run_old(
                "data",
                vec![argument.clone(), NormalizedValue::I64(row as i64)],
            )
        })
        .collect();
    let cursor_time = start.elapsed();
    let ordinals: BTreeMap<_, _> = source
        .types
        .keys()
        .copied()
        .enumerate()
        .map(|(row, digest)| (digest, row))
        .collect();
    let graph: fixture::Graph = source
        .types
        .values()
        .map(|object| {
            fixture::shape(&object.form)
                .1
                .iter()
                .map(|digest| ordinals[digest])
                .collect()
        })
        .collect();
    let graph_start = Instant::now();
    let typed_graph = graph_result(&carrier);
    let typed_graph_time = graph_start.elapsed();
    assert_eq!(graph_difference(&graph, &typed_graph), None);
    let cursor_work = run_cursor("source-budget", "unbounded", vec![]);
    let graph_start = Instant::now();
    let cursor_graph = run_cursor(
        "source-type-dag",
        "check",
        vec![argument.clone(), cursor_work],
    );
    let cursor_graph_time = graph_start.elapsed();
    assert_eq!(integer(field(&cursor_graph, "code")), 0);
    for (row, ((digest, object), entry)) in source.types.iter().zip(entries.iter()).enumerate() {
        let (encoded_digest, encoded) = kernel::encode_type_object(object).unwrap();
        assert_eq!(*digest, encoded_digest);
        assert_eq!(integer(member(entry, "index")), row as i64);
        assert_eq!(octets(member(entry, "digest")), digest.bytes());
        assert_eq!(octets(member(entry, "image")), encoded);
        assert_eq!(octets(member(entry, "image")), octets(&old_images[row]));
        let arguments = vec![argument.clone(), NormalizedValue::I64(row as i64)];
        assert_eq!(
            integer(&run_old("child-count", arguments.clone())),
            graph[row].len() as i64
        );
        let children = run_old("children", arguments);
        assert_eq!(octets(&children).len() % 4, 0);
        let children: Vec<_> = octets(&children)
            .chunks_exact(4)
            .map(|word| u32::from_le_bytes(word.try_into().unwrap()) as usize)
            .collect();
        assert_eq!(children, graph[row]);
    }
    let list_value = branch(&typed).1.unwrap().clone();
    for (row, (digest, _)) in source.types.iter().enumerate() {
        let result = invoke_typed(
            "lookup",
            vec![
                list_value.clone(),
                NormalizedValue::bytes(digest.bytes().as_slice()),
            ],
        );
        assert_eq!(branch(&result).0, "found");
        assert_eq!(
            integer(member(branch(&result).1.unwrap(), "index")),
            row as i64
        );
    }
    let absent = invoke_typed(
        "lookup",
        vec![list_value, NormalizedValue::bytes(&[0_u8; 32][..])],
    );
    assert_eq!(branch(&absent).0, "missing");
    eprintln!(
        "typed compiler input carrier_bytes={} types={} typed_read_ms={} cursor_read_ms={} typed_graph_ms={} cursor_graph_ms={} typed_source_bytes={} typed_source_words={}",
        carrier.len(),
        entries.len(),
        typed_time.as_millis(),
        cursor_time.as_millis(),
        typed_graph_time.as_millis(),
        cursor_graph_time.as_millis(),
        TYPED_SOURCE.len(),
        TYPED_SOURCE.split_whitespace().count()
    );
}

#[test]
fn typed_compiler_input_every_constructor_preserves_structural_child_occurrences() {
    let forms = fixture::constructors();
    let kinds: BTreeSet<_> = forms.iter().map(|form| fixture::shape(form).0).collect();
    assert_eq!(kinds.len(), 35);
    let mut rows: Vec<_> = [3, 7, 11]
        .into_iter()
        .map(|index| {
            (
                fixture::alias(index),
                fixture::image(kernel::TypeForm::Unit),
            )
        })
        .collect();
    let mut graph = vec![vec![]; rows.len()];
    let leaves: BTreeMap<_, _> = rows
        .iter()
        .enumerate()
        .map(|(row, (digest, _))| (*digest, row))
        .collect();
    for (index, form) in forms.iter().enumerate() {
        assert_shape(form);
        rows.push((fixture::alias(100 + index), fixture::image(form.clone())));
        graph.push(
            fixture::shape(form)
                .1
                .iter()
                .map(|digest| leaves[digest])
                .collect(),
        );
    }
    let result = graph_result(&fixture::carrier(&rows));
    assert_eq!(graph_difference(&graph, &result), None);
    // Cross the canonical varint-width change with repeated child occurrences.
    for count in [0, 1, 250, 251] {
        assert_shape(&kernel::TypeForm::Function {
            parameters: vec![fixture::alias(7); count],
            result: fixture::alias(3),
        });
    }
}

#[test]
fn typed_compiler_input_malformed_images_return_structured_errors() {
    let forms = fixture::constructors();
    for form in &forms {
        let image = fixture::image(form.clone());
        let payload = &image[18..image.len() - 32];
        assert_image_error(&envelope(&payload[..payload.len() - 1]));
    }
    let record = fixture::image(forms[10].clone());
    // Reframe every prefix with a correct checksum so payload rejection cannot
    // be accidentally supplied by the envelope checksum validator.
    for end in 0..record.len() - 50 {
        assert_image_error(&envelope(&record[18..18 + end]));
    }
    let valid = fixture::image(kernel::TypeForm::Unit);
    for end in 0..valid.len() {
        assert_image_error(&valid[..end]);
    }
    for at in [0, 8, 9, 10, 17] {
        let mut broken = valid.clone();
        broken[at] ^= 1;
        assert_image_error(&broken);
    }
    let digest = [vec![2], fixture::alias(0).bytes().to_vec()].concat();
    let malformed = vec![
        vec![],
        vec![1],
        vec![2, 0],
        vec![1, 35],
        vec![1, 0, 0],
        vec![251, 1, 0, 0],
        vec![1, 251, 0, 0],
        vec![1, 16, 251],
        vec![1, 16, 251, 1, 0],
        vec![1, 16, 252, 0, 0, 1, 0],
        [vec![1, 11, 1], fixture::alias(0).bytes().to_vec()].concat(),
        [vec![1, 34], digest, vec![4]].concat(),
    ];
    for payload in malformed {
        let image = envelope(&payload);
        assert!(kernel::decode_type_object(&image, TypeObjectDigest::of(&image)).is_err());
        assert_image_error(&image);
    }
    // Both metadata scope tags and payload exhaustion have exact parser obligations.
    let view = forms.iter().find(|form| matches!(form, kernel::TypeForm::ReadViewApplied { arguments, .. } if arguments.is_empty())).unwrap();
    let image = fixture::image(view.clone());
    let mut payload = image[18..image.len() - 32].to_vec();
    payload[37] = 2; // contract, form, 34-byte declaration, empty argument array, scope tag.
    assert_image_error(&envelope(&payload));
    assert_shape(&kernel::TypeForm::Unit); // Error cleanup leaves the evaluator usable.
}

#[test]
fn typed_compiler_input_graph_oracle_and_reducer_cover_cycles_missing_and_duplicates() {
    for graph in [
        vec![],
        vec![vec![]],
        vec![vec![], vec![0, 0]],
        vec![vec![], vec![0], vec![0], vec![1, 2]],
        vec![vec![0]],
        vec![vec![1], vec![0]],
        vec![vec![], vec![2], vec![1]],
        vec![vec![1]],
    ] {
        assert_graph(graph);
    }
    for seed in 0_usize..24 {
        let count = 1 + seed % 8;
        let mut graph: fixture::Graph = (0..count)
            .map(|row| {
                if row == 0 {
                    vec![]
                } else {
                    (0..1 + (seed + row) % 3)
                        .map(|edge| (seed * 17 + row * 13 + edge * 7) % row)
                        .collect()
                }
            })
            .collect();
        assert_graph(graph.clone());
        // A catalogue permutation changes ordinals, never dependency meaning.
        let permuted = graph
            .iter()
            .rev()
            .map(|children| children.iter().map(|child| count - child - 1).collect())
            .collect();
        assert_graph(permuted);
        if count > 1 {
            let child = graph[count - 1][0];
            graph[count - 1].push(child);
        }
        graph.push(vec![]); // Isolated vertices and duplicate edges preserve acceptance.
        assert_graph(graph.clone());
        let endpoint = if seed % 2 == 0 { 0 } else { graph.len() };
        graph[0].push(endpoint);
        assert_graph(graph);
    }
    // The reducer preserves successful admission while isolating the smallest
    // multigraph exposing a hypothetical implementation that drops duplicates.
    let duplicated = |graph: &fixture::Graph| {
        graph.iter().any(|children| {
            children
                .iter()
                .enumerate()
                .any(|(at, child)| children[..at].contains(child))
        })
    };
    let minimal = fixture::reduce(vec![vec![], vec![], vec![0, 1, 1], vec![0]], duplicated);
    assert_eq!(minimal, vec![vec![], vec![0, 0]]);
    assert_eq!(fixture::class(&minimal), fixture::Class::Ready);
}

#[test]
fn typed_compiler_input_graph_errors_preserve_catalogue_locations() {
    let input = graph_result(&[]);
    let (case, payload) = branch(branch(&input).1.unwrap());
    assert_eq!(case, "input");
    assert_eq!(integer(member(payload.unwrap(), "at")), 0);
    let valid = fixture::image(kernel::TypeForm::Unit);
    let duplicate = graph_result(&fixture::carrier(&[
        (fixture::alias(0), valid.clone()),
        (fixture::alias(0), valid.clone()),
    ]));
    let (case, payload) = branch(branch(&duplicate).1.unwrap());
    assert_eq!(case, "duplicate-digest");
    let details = payload.unwrap();
    assert_eq!(integer(member(details, "first")), 0);
    assert_eq!(integer(member(details, "second")), 1);
    assert_eq!(octets(member(details, "digest")), fixture::alias(0).bytes());

    let malformed = graph_result(&fixture::carrier(&[
        (fixture::alias(0), valid.clone()),
        (fixture::alias(1), envelope(&[1, 35])),
    ]));
    let (case, payload) = branch(branch(&malformed).1.unwrap());
    assert_eq!(case, "malformed");
    assert_eq!(integer(member(payload.unwrap(), "ordinal")), 1);
    assert!(
        matches!(member(member(payload.unwrap(), "problem"), "message"), NormalizedValue::Text(text) if !text.is_empty())
    );

    let table = invoke_typed(
        "read",
        vec![NormalizedValue::bytes(fixture::carrier(&[(
            fixture::alias(0),
            valid,
        )]))],
    );
    let mut entries: Vec<_> = values(branch(&table).1.unwrap()).iter().cloned().collect();
    let NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }) = &entries[0] else {
        panic!("expected TypeEntry");
    };
    let layout = *layout;
    let at = prototype().1.records[layout.0 as usize]
        .fields
        .iter()
        .position(|f| f.name.as_str() == "index")
        .unwrap();
    let mut fields: Vec<_> = fields.iter().cloned().collect();
    fields[at] = NormalizedValue::I64(7);
    entries[0] = NormalizedValue::Record(NormalizedRecord::Nominal {
        layout,
        fields: crate::platform::execution::normalized::child_storage::Values::try_new(fields)
            .unwrap(),
    });
    let ordinal = invoke_observed(
        "typed-dag",
        "analyze",
        vec![NormalizedValue::list(entries).unwrap()],
    )
    .0;
    let (case, payload) = branch(branch(&ordinal).1.unwrap());
    assert_eq!(case, "ordinal");
    assert_eq!(integer(member(payload.unwrap(), "ordinal")), 0);
    assert_eq!(integer(member(payload.unwrap(), "index")), 7);
    assert_graph(vec![vec![], vec![0, 0]]);
}

#[test]
#[ignore = "slow: geometric structural reader and dependency graph scale"]
fn typed_compiler_input_structural_dag_scale() {
    prototype();
    for size in [256, 1024, 4096] {
        for (name, graph) in [
            ("chain", fixture::chain(size)),
            ("wide", fixture::wide(size)),
        ] {
            let bytes = fixture::carrier(&fixture::rows(&graph));
            let argument = NormalizedValue::bytes(bytes.as_slice());
            let start = Instant::now();
            let (result, work) = invoke_observed("typed-dag", "read-graph", vec![argument]);
            let read_time = start.elapsed();
            assert_eq!(graph_difference(&graph, &result), None);
            let retained = result.clone();
            drop(result);
            assert_eq!(graph_difference(&graph, &retained), None);
            drop(retained);
            eprintln!(
                "typed structural scale shape={name} types={size} edges={} bytes={} read_ms={} lifecycle_ms={} instructions={} allocation_charges={}",
                graph.iter().map(Vec::len).sum::<usize>(),
                bytes.len(),
                read_time.as_millis(),
                start.elapsed().as_millis(),
                work.instructions,
                work.allocation_charges
            );
        }
    }
}
