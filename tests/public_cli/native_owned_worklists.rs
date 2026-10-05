//! Parameterized owned contracts through fresh packages and independent native results.
use super::native_byte_buffer::{Export, author, dependency, export, stage};
use super::*;
use serde_json::json;

const LIBRARY: &str = include_str!("../../examples/owned-worklists/library.lkjc");
const CARRIERS: &str = include_str!("../../examples/owned-worklists/carriers.lkjc");
const APPLICATION: &str = include_str!("../../examples/owned-worklists/application.lkjc");
const REACHABILITY: &str = include_str!("../../examples/owned-worklists/reachability.lkjc");

struct Packages {
    library: Native,
    carriers: Native,
    consumer: Native,
    generic: Export,
    concrete: Export,
    standards: [PathBuf; 2],
}

fn standard(public: &Native) -> Export {
    let output = public.root.path().join("standard.lkjp");
    let records = public.cli(
        &[
            "package",
            "builtin",
            "export",
            "--kind",
            "transport",
            "--output",
            path(&output),
        ],
        true,
    );
    let builtin = compact_record(&records, "package");
    let package = Export {
        path: output,
        package: compact_field(builtin, "id").into(),
        semantic: compact_field(builtin, "revision").into(),
        revision: compact_field(builtin, "package-revision").into(),
        transport: compact_field(builtin, "transport").into(),
    };
    stage(public, &package);
    package
}

impl Packages {
    fn stage() -> Self {
        let library = Native::new();
        let library_standard = standard(&library);
        author(
            &library,
            &format!("{}{LIBRARY}", dependency(&library_standard)),
        );
        let draft = unchanged(&library, "owned-worklists");
        for absent in ["OwnedI64Cell", "ByteBuffer", "owned-implementation"] {
            assert!(!draft.contains(absent), "generic library: {absent}");
        }
        for present in [
            "(type-parameter ",
            "(constraint owned)",
            "(owned-choice ",
            "(owned-product ",
        ] {
            assert!(draft.contains(present), "generic library: {present}");
        }
        let contract = declaration(&library, "owned-worklists", "Worklist");
        let inspected = library.cli(&["inspect", "owner", "owned_contract", &contract], true);
        assert_eq!(
            compact_field(
                compact_record(&inspected, "owned.contract"),
                "type-parameters"
            ),
            "1"
        );
        let build = declaration(&library, "owned-worklists", "build");
        let inspected = library.cli(&["inspect", "owner", "pure_function", &build], true);
        let witnesses: Vec<_> = inspected
            .iter()
            .filter(|record| record.operation == "owned.implementation-parameter")
            .collect();
        assert_eq!(witnesses.len(), 2);
        assert_eq!(compact_field(witnesses[0], "type-arguments"), "0");
        assert_eq!(compact_field(witnesses[1], "type-arguments"), "1");
        let arguments: Vec<_> = inspected
            .iter()
            .filter(|record| record.operation == "owned.witness-type-argument")
            .collect();
        assert_eq!(arguments.len(), 1);
        assert_eq!(compact_field(arguments[0], "index"), "0");
        assert_eq!(
            compact_field(arguments[0], "witness"),
            compact_field(witnesses[1], "id")
        );
        assert_eq!(
            compact_field(arguments[0], "type"),
            compact_field(witnesses[0], "self")
        );
        assert_ne!(
            compact_field(arguments[0], "type"),
            compact_field(witnesses[1], "self")
        );
        // Admission and export happen before either concrete element/storage witness exists.
        let generic = export(&library);
        let carriers = Native::new();
        stage(&carriers, &generic);
        let carrier_standard = standard(&carriers);
        author(
            &carriers,
            &format!(
                "{}{}declarations.begin\n(units (use owned-worklists {} {}))\ndeclarations.end\n{CARRIERS}",
                dependency(&generic),
                dependency(&carrier_standard),
                generic.package,
                generic.revision,
            ),
        );
        unchanged(&carriers, "worklist-carriers");
        let concrete = export(&carriers);
        let consumer = Native::template("command");
        stage(&consumer, &generic);
        stage(&consumer, &concrete);
        Self {
            library,
            carriers,
            consumer,
            generic,
            concrete,
            standards: [library_standard.path, carrier_standard.path],
        }
    }

    fn source(&self, body: &str) -> String {
        format!(
            "{}{}declarations.begin\n(units (use owned-worklists {} {}) (use worklist-carriers {} {}))\ndeclarations.end\n{body}",
            dependency(&self.generic),
            dependency(&self.concrete),
            self.generic.package,
            self.generic.revision,
            self.concrete.package,
            self.concrete.revision,
        )
    }

    fn detach(&self) {
        for project in [
            &self.library.project,
            &self.carriers.project,
            &self.consumer.project,
        ] {
            std::fs::remove_dir_all(project).unwrap();
        }
        for package in [
            &self.generic.path,
            &self.concrete.path,
            &self.standards[0],
            &self.standards[1],
        ] {
            std::fs::remove_file(package).unwrap();
        }
    }

    fn retain(self, label: &str) {
        if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
            for public in [self.library, self.carriers, self.consumer] {
                println!(
                    "retained owned-worklist {label} public evidence: {}",
                    public.root.keep().display()
                );
            }
        }
    }
}

fn unchanged(public: &Native, module: &str) -> String {
    let draft = public.root.path().join(format!("{module}-draft.lkjc"));
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            module,
            "--output",
            path(&draft),
        ],
        true,
    );
    let plan = public.plan(&draft, true);
    assert_eq!(
        compact_field(compact_record(&plan, "result"), "outcome"),
        "unchanged"
    );
    std::fs::read_to_string(draft).unwrap()
}

fn declaration(public: &Native, module: &str, name: &str) -> String {
    let module = public.cli(&["query", "find", "module", module], true);
    let parent = compact_field(compact_record(&module, "owner"), "id");
    let owner = public.cli(
        &["query", "find", "declaration", name, "--parent", parent],
        true,
    );
    compact_field(compact_record(&owner, "owner"), "id").into()
}

fn deployment(public: &Native, artifact: &str, target: &str) -> PathBuf {
    public.input(
        &format!("{target}.deployment.json"),
        &json!({
            "artifact":artifact,"target":target,"listen":null,
            "http":null,"session":null,"worker":null,
            "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,
                "maximum_total_bytes":1048576,"maximum_live_streams":1024},
            "grants":[],"secrets":[],"configuration":{},
        })
        .to_string(),
    )
}

fn clean_execution(records: &[CompactRecord], detached: bool) {
    let execution = compact_record(records, "execution");
    if !detached {
        assert_eq!(compact_field(execution, "differential"), "equal");
        for field in ["production-instructions", "reference-expressions"] {
            assert!(compact_field(execution, field).parse::<u64>().unwrap() > 0);
        }
        return;
    }
    let observation: Value =
        serde_json::from_str(compact_field(execution, "production-observation")).unwrap();
    assert_eq!(observation["capability_calls"], json!(0));
    assert_eq!(observation["live_handles_after"], json!(0));
    let cleanup: Value = serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
    assert_eq!(cleanup["remaining_tasks"], json!(0));
    assert_eq!(cleanup["cleanup_failures"], json!([]));
}

fn run_expected(
    public: &Native,
    target: &str,
    deployment: &Path,
    detached: bool,
    label: &str,
    arguments: Value,
    expected: &Value,
) {
    let input = public.input(
        &format!("{target}-{detached}-{label}-arguments.json"),
        &arguments.to_string(),
    );
    let result = public
        .root
        .path()
        .join(format!("{target}-{detached}-{label}-result.json"));
    let mut args = if detached {
        vec!["run", "--deployment", path(deployment)]
    } else {
        vec!["run", target]
    };
    args.extend([
        "--arguments-file",
        path(&input),
        "--result-file",
        path(&result),
    ]);
    let records = public.cli(&args, true);
    // A complete independent result is the oracle, including canonical JSON bytes.
    assert_eq!(
        std::fs::read(result).unwrap(),
        serde_json::to_vec(expected).unwrap(),
        "{target}, detached={detached}, {label}",
    );
    clean_execution(&records, detached);
}

fn worklist_expected(inputs: &[i64]) -> Value {
    let scenario = json!({
        "length":inputs.len(),"moved":inputs,"empty-length":0,"reused":[17],
    });
    json!({
        "cell-flat":scenario,"cell-chunked":scenario,
        "buffer-flat":scenario,"buffer-chunked":scenario,
    })
}

#[test]
fn native_owned_worklist_parameterized_three_packages_move_drain_reuse_and_detach() {
    let packages = Packages::stage();
    let public = &packages.consumer;
    author(public, &packages.source(APPLICATION));
    unchanged(public, "owned-worklists-app");
    let artifact = public.root.path().join("owned-worklists.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = deployment(public, "owned-worklists.lkja", "owned-worklists");
    let benchmark_targets = ["cell-flat", "cell-chunked", "buffer-flat", "buffer-chunked"];
    let benchmark_descriptors = [
        include_str!("../../examples/owned-worklists/cell-flat.deployment.json"),
        include_str!("../../examples/owned-worklists/cell-chunked.deployment.json"),
        include_str!("../../examples/owned-worklists/buffer-flat.deployment.json"),
        include_str!("../../examples/owned-worklists/buffer-chunked.deployment.json"),
    ];
    let benchmark_descriptors: Vec<PathBuf> = benchmark_targets
        .iter()
        .zip(benchmark_descriptors)
        .map(|(target, descriptor)| public.input(&format!("{target}.deployment.json"), descriptor))
        .collect();
    let benchmark_inputs: Vec<i64> = (0..513).map(|n| n % 17).collect();
    let benchmark_drained: Vec<i64> = benchmark_inputs.iter().rev().copied().collect();
    let benchmark_expected = json!({
        "length":513,"drained":benchmark_drained,"empty-length":0,"reused":[17],
    });
    let mut workloads = vec![vec![], vec![2], vec![2, 3, 7], vec![0, 255, 128]];
    for length in [31, 32, 33, 64, 65, 513] {
        workloads.push((0..length).map(|n| n % 17).collect());
    }
    for detached in [false, true] {
        if detached {
            packages.detach();
        }
        for (case, inputs) in workloads.iter().enumerate() {
            run_expected(
                public,
                "owned-worklists",
                &descriptor,
                detached,
                &case.to_string(),
                json!([inputs]),
                &worklist_expected(inputs),
            );
        }
        for (target, descriptor) in benchmark_targets.iter().zip(&benchmark_descriptors) {
            run_expected(
                public,
                target,
                descriptor,
                detached,
                "513",
                json!([benchmark_inputs]),
                &benchmark_expected,
            );
        }
    }
    packages.retain("storage-and-move");
}

fn graph(roots: &[i64], nodes: Vec<Value>) -> Value {
    json!({"roots":roots,"nodes":nodes})
}

fn node(id: i64, successors: &[i64]) -> Value {
    json!({"id":id,"successors":successors})
}

fn valid(reachable: &[i64], unreachable: &[i64]) -> Value {
    json!({"case":"valid","value":{"reachable":reachable,"unreachable":unreachable}})
}

fn invalid(code: &str, owner: i64, target: i64) -> Value {
    json!({"case":"invalid","value":{"code":code,"owner":owner,"target":target}})
}

fn graph_cases() -> Vec<(String, Value, Value)> {
    let mut cases = vec![
        ("empty".into(), graph(&[], vec![]), valid(&[], &[])),
        (
            "singleton".into(),
            graph(&[-7], vec![node(-7, &[])]),
            valid(&[-7], &[]),
        ),
        (
            "empty-roots".into(),
            graph(&[], vec![node(1, &[1])]),
            valid(&[], &[1]),
        ),
        (
            "diamond-cycle-disconnected".into(),
            graph(
                &[10],
                vec![
                    node(10, &[20, 30]),
                    node(20, &[40]),
                    node(30, &[40]),
                    node(40, &[20]),
                    node(50, &[50]),
                    node(60, &[]),
                ],
            ),
            valid(&[10, 20, 30, 40], &[50, 60]),
        ),
        (
            "duplicate-roots-and-edges".into(),
            graph(
                &[10, 10, 20],
                vec![node(10, &[20, 20]), node(20, &[30]), node(30, &[10, 30])],
            ),
            valid(&[10, 20, 30], &[]),
        ),
        (
            "signed-extremes-authored-order".into(),
            graph(
                &[i64::MIN],
                vec![
                    node(i64::MAX, &[0]),
                    node(i64::MIN, &[i64::MAX]),
                    node(0, &[i64::MIN]),
                    node(-1, &[]),
                ],
            ),
            valid(&[i64::MAX, i64::MIN, 0], &[-1]),
        ),
        (
            "duplicate-precedes-missing-root-and-successor".into(),
            graph(
                &[100],
                vec![node(1, &[]), node(7, &[]), node(1, &[99]), node(7, &[])],
            ),
            invalid("duplicate-node", 1, 1),
        ),
        (
            "first-missing-root-precedes-successor".into(),
            graph(&[99, 98], vec![node(1, &[97])]),
            invalid("missing-root", -1, 99),
        ),
        (
            "unreachable-missing-successor".into(),
            graph(&[10], vec![node(10, &[]), node(50, &[999])]),
            invalid("missing-successor", 50, 999),
        ),
        (
            "first-missing-successor-in-node-edge-order".into(),
            graph(&[], vec![node(20, &[98, 99]), node(10, &[97])]),
            invalid("missing-successor", 20, 98),
        ),
    ];
    for width in [31, 32, 33, 64, 65] {
        let children: Vec<i64> = (1..=width).collect();
        let mut nodes = vec![node(0, &children)];
        nodes.extend(children.iter().rev().map(|id| node(*id, &[])));
        nodes.push(node(-1, &[-1]));
        let expected: Vec<i64> = std::iter::once(0)
            .chain(children.iter().rev().copied())
            .collect();
        cases.push((
            format!("chunk-boundary-{width}"),
            graph(&[0], nodes),
            valid(&expected, &[-1]),
        ));
    }
    let chain: Vec<i64> = (0..513).collect();
    cases.push((
        "chain-513".into(),
        graph(
            &[0],
            chain
                .iter()
                .map(|id| {
                    if *id < 512 {
                        node(*id, &[*id + 1])
                    } else {
                        node(*id, &[])
                    }
                })
                .collect(),
        ),
        valid(&chain, &[]),
    ));
    let branches: Vec<i64> = (1..=65).collect();
    let mut diamond = vec![node(0, &branches)];
    diamond.extend(branches.iter().rev().map(|id| node(*id, &[66])));
    diamond.extend([node(66, &[]), node(67, &[])]);
    let diamond_expected: Vec<i64> = std::iter::once(0)
        .chain(branches.iter().rev().copied())
        .chain(std::iter::once(66))
        .collect();
    cases.push((
        "wide-diamond".into(),
        graph(&[0], diamond),
        valid(&diamond_expected, &[67]),
    ));
    let admitted_nodes: Vec<i64> = (0..4096).collect();
    cases.extend([
        (
            "node-capacity-inclusive".into(),
            graph(
                &[],
                admitted_nodes.iter().map(|id| node(*id, &[])).collect(),
            ),
            valid(&[], &admitted_nodes),
        ),
        (
            "node-capacity-before-invalid-root".into(),
            graph(&[-99], (0..4097).map(|id| node(id, &[])).collect()),
            json!({"case":"capacity","value":"nodes"}),
        ),
        (
            "root-capacity-inclusive".into(),
            graph(&vec![7; 4096], vec![node(7, &[])]),
            valid(&[7], &[]),
        ),
        (
            "root-capacity-before-invalid-successor".into(),
            graph(&vec![7; 4097], vec![node(7, &[999])]),
            json!({"case":"capacity","value":"roots"}),
        ),
        (
            "edge-capacity-inclusive".into(),
            graph(&[7], vec![node(7, &vec![7; 16384])]),
            valid(&[7], &[]),
        ),
        (
            "edge-capacity-before-invalid-root".into(),
            graph(&[999], vec![node(7, &vec![7; 16385])]),
            json!({"case":"capacity","value":"edges"}),
        ),
    ]);
    cases
}

#[test]
fn native_owned_worklist_provisional_graph_validates_full_candidates_and_detach() {
    let packages = Packages::stage();
    let public = &packages.consumer;
    author(public, &packages.source(REACHABILITY));
    unchanged(public, "provisional-graph");
    let artifact = public.root.path().join("provisional-graph.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let targets = [
        "provisional-reachability-flat",
        "provisional-reachability-chunked",
    ];
    let descriptors = targets.map(|target| deployment(public, "provisional-graph.lkja", target));
    let cases = graph_cases();
    for detached in [false, true] {
        if detached {
            packages.detach();
        }
        for (target, descriptor) in targets.iter().zip(&descriptors) {
            for (name, proposal, expected) in &cases {
                run_expected(
                    public,
                    target,
                    descriptor,
                    detached,
                    name,
                    json!([proposal]),
                    expected,
                );
            }
        }
    }
    // Native candidate errors above are successful typed outcome data; malformed
    // runner data is independently rejected before the native pass executes.
    for (index, arguments) in [json!([{}]), json!([{"roots":["x"],"nodes":[]}])]
        .iter()
        .enumerate()
    {
        let input = public.input(&format!("malformed-{index}.json"), &arguments.to_string());
        let result = public
            .root
            .path()
            .join(format!("malformed-{index}-result.json"));
        let records = public.cli(
            &[
                "run",
                "--deployment",
                path(&descriptors[0]),
                "--arguments-file",
                path(&input),
                "--result-file",
                path(&result),
            ],
            false,
        );
        assert!(
            records
                .iter()
                .any(|record| record.operation == "diagnostic")
        );
        assert!(!result.exists());
    }
    packages.retain("provisional-reachability");
}

// Two independent contract arguments make their order observable in both a
// consuming parameter and a structural result, without relying on the examples.
const SIGNATURES: &str = r#"declarations.begin
(units (module create worklist-validation
  (owned-contract create Exchange (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create First (constraint owned))
    (type-parameter create Second (constraint owned))
    (method method_89000000000000000000000000000001 exchange
      (parameters (Second consume) (Self consume))
      (returns (owned-product (field rest Self) (field value First))) (effect pure)))
  (external create create-cell (visibility private)
    (implementation core.cell.create) (parameter create value (type I64)) (returns OwnedI64Cell))
  (external create empty-buffer (visibility private)
    (implementation core.buffer.empty) (returns ByteBuffer))
  (external create discard-buffer (visibility private)
    (implementation core.buffer.discard)
    (parameter create value (type ByteBuffer) (use consume)) (returns Unit))
  (function create exchange-concrete (visibility public) (effect pure)
    (parameter create source (type ByteBuffer) (use consume))
    (parameter create queue (type (owned-sequence OwnedI64Cell)) (use consume))
    (returns (owned-product (field rest (owned-sequence OwnedI64Cell)) (field value OwnedI64Cell)))
    (body (sequence (call discard-buffer (local source))
      (let (binding value (type OwnedI64Cell) (call create-cell (i64 42)))
        (in (pack-owned
          (type (owned-product (field rest (owned-sequence OwnedI64Cell)) (field value OwnedI64Cell)))
          (field rest (local queue)) (field value (local value))))))))
  (owned-implementation create ConcreteExchange (visibility public)
    (contract Exchange) (self (owned-sequence OwnedI64Cell)) (types OwnedI64Cell ByteBuffer)
    (method method_89000000000000000000000000000001 exchange-concrete))
  (function create exchange (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (type-parameter create U (constraint owned))
    (type-parameter create Q (constraint owned))
    (implementation-parameter implparam_89000000000000000000000000000001 ops Exchange Q (types T U))
    (parameter create source (type U) (use consume))
    (parameter create queue (type Q) (use consume))
    (returns (owned-product (field rest Q) (field value T)))
    (body (method-call parameter@exchange@implparam_89000000000000000000000000000001
      Exchange method_89000000000000000000000000000001 (local source) (local queue))))
  (function create exercise (visibility public) (effect pure) (returns Unit)
    (body (let
      (binding source (type ByteBuffer) (call empty-buffer))
      (binding queue (type (owned-sequence OwnedI64Cell))
        (sequence-empty (type (owned-sequence OwnedI64Cell))))
      (binding result
        (type (owned-product (field rest (owned-sequence OwnedI64Cell)) (field value OwnedI64Cell)))
        (implementation-call exchange (types OwnedI64Cell ByteBuffer (owned-sequence OwnedI64Cell))
          (implementations concrete@ConcreteExchange) (local source) (local queue)))
      (in (unit)))))))
declarations.end
"#;

fn rejected(public: &Native, source: &str, before: &str, name: &str) {
    let input = public.input(
        &format!("invalid-{name}.lkjc"),
        &format!("request base={before}\n{source}"),
    );
    let records = public.plan(&input, false);
    assert!(
        records.iter().any(|record| record.operation == "diagnostic"
            && compact_field(record, "code").starts_with("kernel_")),
        "{name} must fail semantic admission: {records:?}"
    );
    assert_eq!(public.revision(), before, "{name}");
}

fn replaced(source: &str, from: &str, to: &str) -> String {
    assert!(source.contains(from), "negative fixture source: {from}");
    source.replacen(from, to, 1)
}

#[test]
fn native_owned_parameterized_contracts_reject_wrong_arguments_modes_and_witnesses_without_publication()
 {
    let public = Native::new();
    let before = public.revision();
    let cases = [
        (
            "missing-implementation-argument",
            "(types OwnedI64Cell ByteBuffer)",
            "(types OwnedI64Cell)",
        ),
        (
            "extra-implementation-argument",
            "(types OwnedI64Cell ByteBuffer)",
            "(types OwnedI64Cell ByteBuffer OwnedI64Cell)",
        ),
        (
            "swapped-implementation-arguments",
            "(types OwnedI64Cell ByteBuffer)",
            "(types ByteBuffer OwnedI64Cell)",
        ),
        (
            "ordinary-implementation-argument",
            "(types OwnedI64Cell ByteBuffer)",
            "(types I64 ByteBuffer)",
        ),
        (
            "wrong-implementation-self",
            "(contract Exchange) (self (owned-sequence OwnedI64Cell))",
            "(contract Exchange) (self ByteBuffer)",
        ),
        (
            "missing-witness-argument",
            "ops Exchange Q (types T U)",
            "ops Exchange Q (types T)",
        ),
        (
            "extra-witness-argument",
            "ops Exchange Q (types T U)",
            "ops Exchange Q (types T U Q)",
        ),
        (
            "swapped-witness-arguments",
            "ops Exchange Q (types T U)",
            "ops Exchange Q (types U T)",
        ),
        (
            "ordinary-witness-argument",
            "ops Exchange Q (types T U)",
            "ops Exchange Q (types I64 U)",
        ),
        (
            "wrong-selected-contract-argument",
            "(implementation-call exchange (types OwnedI64Cell ByteBuffer (owned-sequence OwnedI64Cell))",
            "(implementation-call exchange (types ByteBuffer ByteBuffer (owned-sequence OwnedI64Cell))",
        ),
        (
            "wrong-selected-self",
            "(implementation-call exchange (types OwnedI64Cell ByteBuffer (owned-sequence OwnedI64Cell))",
            "(implementation-call exchange (types OwnedI64Cell ByteBuffer ByteBuffer)",
        ),
        (
            "method-mode-mismatch",
            "(parameters (Second consume) (Self consume))",
            "(parameters (Second borrow) (Self consume))",
        ),
        (
            "unrestricted-owned-method-item",
            "(parameters (Second consume) (Self consume))",
            "(parameters (Second unrestricted) (Self consume))",
        ),
        (
            "method-result-substitution",
            "(returns (owned-product (field rest Self) (field value First)))",
            "(returns (owned-product (field rest Self) (field value Second)))",
        ),
        (
            "owned-item-in-ordinary-list",
            "(returns (owned-product (field rest Self) (field value First)))",
            "(returns (list First))",
        ),
        (
            "unconstrained-contract-item",
            "(type-parameter create First (constraint owned))",
            "(type-parameter create First)",
        ),
        (
            "unconstrained-witness-item",
            "(type-parameter create T (constraint owned))",
            "(type-parameter create T)",
        ),
        (
            "unconstrained-witness-self",
            "(type-parameter create Q (constraint owned))",
            "(type-parameter create Q)",
        ),
        (
            "borrowed-item-consumed-by-method",
            "(parameter create source (type U) (use consume))",
            "(parameter create source (type U) (use borrow))",
        ),
        (
            "missing-method-map",
            "(method method_89000000000000000000000000000001 exchange-concrete)",
            "",
        ),
        (
            "out-of-scope-witness",
            "(implementations concrete@ConcreteExchange)",
            "(implementations parameter@exchange@implparam_89000000000000000000000000000001)",
        ),
    ];
    for (name, from, to) in cases {
        rejected(&public, &replaced(SIGNATURES, from, to), &before, name);
    }
    rejected(
        &public,
        r#"declarations.begin
(units (module create invalid-task-item
  (owned-contract create TaskItem (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Item (constraint owned))
    (method method_89000000000000000000000000000002 read
      (parameters (Item borrow)) (returns I64) (effect (task))))))
declarations.end
"#,
        &before,
        "borrowed-task-contract-item",
    );
    // The complete independent source is subsequently admitted unchanged; these
    // are invalid meanings rather than malformed notation or a broken base.
    author(&public, SIGNATURES);
    unchanged(&public, "worklist-validation");
    let contract = declaration(&public, "worklist-validation", "Exchange");
    let inspected = public.cli(&["inspect", "owner", "owned_contract", &contract], true);
    assert_eq!(
        compact_field(
            compact_record(&inspected, "owned.contract"),
            "type-parameters"
        ),
        "2"
    );
    let implementation = declaration(&public, "worklist-validation", "ConcreteExchange");
    let inspected = public.cli(
        &["inspect", "owner", "owned_implementation", &implementation],
        true,
    );
    assert_eq!(
        compact_field(
            compact_record(&inspected, "owned.implementation"),
            "type-arguments"
        ),
        "2"
    );
    let arguments: Vec<_> = inspected
        .iter()
        .filter(|record| record.operation == "owned.implementation-type-argument")
        .collect();
    assert_eq!(arguments.len(), 2);
    assert_eq!(compact_field(arguments[0], "index"), "0");
    assert_eq!(compact_field(arguments[1], "index"), "1");
    assert_ne!(
        compact_field(arguments[0], "type"),
        compact_field(arguments[1], "type")
    );
    let function = declaration(&public, "worklist-validation", "exchange");
    let inspected = public.cli(&["inspect", "owner", "pure_function", &function], true);
    assert_eq!(
        compact_field(
            compact_record(&inspected, "owned.implementation-parameter"),
            "type-arguments"
        ),
        "2"
    );
    let arguments: Vec<_> = inspected
        .iter()
        .filter(|record| record.operation == "owned.witness-type-argument")
        .collect();
    assert_eq!(arguments.len(), 2);
    assert_eq!(compact_field(arguments[0], "index"), "0");
    assert_eq!(compact_field(arguments[1], "index"), "1");
    assert_ne!(
        compact_field(arguments[0], "type"),
        compact_field(arguments[1], "type")
    );
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        println!(
            "retained parameterized-contract rejection public evidence: {}",
            public.root.keep().display()
        );
    }
}
