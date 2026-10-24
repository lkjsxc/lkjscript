//! Whole-carrier scalar E2 controls and source-driven E3 native execution.
//! Rust authors the valid seed and re-encodes complete derived carriers. It performs
//! no target semantic admission, normalization, or lowering before the .lkjc compiler.
//! The maintained compiler is bootstrapped once per entry with the Rust backend;
//! these controls do not establish maintained-compiler C2/C3 fixed points.
use super::*;
use crate::platform::kernel::ExpressionOperation as E;
use crate::platform::semantic_id::{BindingId, DeclarationId, ExpressionId, PortId};

const SCALAR_SOURCE: &str = include_str!("../../../../tests/fixtures/selfhost-source-scalar.lkjc");

struct Case {
    name: &'static str,
    carrier: Vec<u8>,
    carrier_sha256: String,
    diagnostic: i64,
    result: Option<i64>,
    e1: OnceLock<()>,
}

impl Case {
    fn new(
        name: &'static str,
        source: KernelSnapshot,
        diagnostic: i64,
        result: Option<i64>,
    ) -> Self {
        assert_eq!(diagnostic == 0, result.is_some());
        let carrier = graph::packet(&source);
        let carrier_sha256 = sha256(&carrier);
        Self {
            name,
            carrier,
            carrier_sha256,
            diagnostic,
            result,
            e1: OnceLock::new(),
        }
    }

    fn assert_e1(&self) {
        self.e1.get_or_init(|| {
            // Every derived semantic negative must reach E2 after independently
            // checking canonical hashes, complete references, syntax and binders.
            let (code, state) = super::result("read-nominal", &self.carrier);
            assert_eq!(
                code, 0,
                "E1 control={}, carrier_sha256={}",
                self.name, self.carrier_sha256,
            );
            assert_eq!(
                state.as_slice(),
                &self.carrier[48..80],
                "E1 complete carrier control={}",
                self.name,
            );
        });
    }
}

fn sha256(bytes: &[u8]) -> String {
    crate::platform::semantic_id::encode_hex(&Sha256::digest(bytes))
}

fn fixture() -> &'static KernelSnapshot {
    static SOURCE: OnceLock<KernelSnapshot> = OnceLock::new();
    SOURCE.get_or_init(|| byte_buffer_tests::author_only(SCALAR_SOURCE).unwrap())
}

fn declaration(source: &KernelSnapshot, name: &str) -> DeclarationId {
    source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.name.as_str() == name =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}

fn declaration_reference(source: &KernelSnapshot, name: &str) -> DeclarationReference {
    DeclarationReference {
        package: source.root.package_id,
        declaration: declaration(source, name),
    }
}

fn fixture_function<'a>(source: &'a KernelSnapshot, name: &str) -> &'a kernel::FunctionDeclaration {
    let OwnerRecord::Declaration(record) =
        &source.owners[&OwnerKey::Declaration(declaration(source, name))]
    else {
        panic!("fixture declaration");
    };
    let kernel::DeclarationPayload::Function(function) = &record.payload else {
        panic!("fixture function");
    };
    function
}

fn expression(source: &KernelSnapshot, id: ExpressionId) -> &kernel::ExpressionRecord {
    let OwnerRecord::Expression(record) = &source.owners[&OwnerKey::Expression(id)] else {
        panic!("fixture expression");
    };
    record
}

fn expression_mut(source: &mut KernelSnapshot, id: ExpressionId) -> &mut kernel::ExpressionRecord {
    let OwnerRecord::Expression(record) = source.owners.get_mut(&OwnerKey::Expression(id)).unwrap()
    else {
        panic!("fixture expression");
    };
    record
}

fn entry_binding(source: &KernelSnapshot, name: &str) -> BindingId {
    let body = fixture_function(source, "entry").body;
    let E::Let { bindings, .. } = &expression(source, body).operation else {
        panic!("entry let");
    };
    bindings
        .iter()
        .find_map(
            |binding| match &source.owners[&OwnerKey::Binding(*binding)] {
                OwnerRecord::Binding(record) if record.name.as_str() == name => Some(*binding),
                _ => None,
            },
        )
        .unwrap()
}

fn entry_initializer(source: &KernelSnapshot, name: &str) -> ExpressionId {
    let OwnerRecord::Binding(binding) =
        &source.owners[&OwnerKey::Binding(entry_binding(source, name))]
    else {
        panic!("entry binding");
    };
    binding.value.unwrap()
}

fn port(source: &KernelSnapshot, component: &str) -> PortId {
    let parent = declaration(source, component);
    source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Port(id), OwnerRecord::Port(record))
                if record.declaration == parent && record.name.as_str() == "run" =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}

fn target_mut(source: &mut KernelSnapshot) -> &mut kernel::TargetRecord {
    source
        .owners
        .values_mut()
        .find_map(|record| match record {
            OwnerRecord::Target(target) if target.name.as_str() == "scalar-main" => Some(target),
            _ => None,
        })
        .unwrap()
}

fn scalar_type(source: &KernelSnapshot, form: kernel::TypeForm) -> TypeObjectDigest {
    source
        .types
        .iter()
        .find_map(|(digest, object)| (object.form == form).then_some(*digest))
        .unwrap()
}

fn insert_unused_type(source: &mut KernelSnapshot, form: kernel::TypeForm) {
    let object = kernel::TypeObject::new(form).unwrap();
    let digest = kernel::encode_type_object(&object).unwrap().0;
    assert!(source.types.insert(digest, object).is_none());
}

fn assert_same_state_different_carrier(before: &Case, after: &Case) {
    assert_eq!(&before.carrier[48..80], &after.carrier[48..80]);
    assert_ne!(before.carrier, after.carrier);
    assert_ne!(before.carrier_sha256, after.carrier_sha256);
}

fn cases() -> &'static [Case] {
    static CASES: OnceLock<Vec<Case>> = OnceLock::new();
    CASES.get_or_init(|| {
        let base = fixture();
        let mut cases = vec![Case::new("baseline", base.clone(), 0, Some(42))];

        for (name, value) in [
            ("zero", 0),
            ("negative", -1),
            ("minimum-i64", i64::MIN),
            ("maximum-i64", i64::MAX),
        ] {
            let mut source = base.clone();
            let initializer = entry_initializer(&source, "seed");
            expression_mut(&mut source, initializer).operation = E::I64 { value };
            cases.push(Case::new(name, source, 0, Some(value)));
        }

        let mut source = base.clone();
        let selected = entry_initializer(&source, "selected");
        expression_mut(&mut source, selected).operation = E::Bool { value: false };
        cases.push(Case::new("false-branch", source, 0, Some(-9)));

        let mut source = base.clone();
        let component = declaration_reference(&source, "other");
        let selected = kernel::PortReference {
            package: source.root.package_id,
            port: port(&source, "other"),
        };
        let target = target_mut(&mut source);
        target.component = component;
        target.port = Some(selected);
        cases.push(Case::new(
            "target-selects-other-component",
            source,
            0,
            Some(7),
        ));

        let mut source = base.clone();
        let boolean = scalar_type(&source, kernel::TypeForm::Bool);
        let unit = scalar_type(&source, kernel::TypeForm::Unit);
        insert_unused_type(
            &mut source,
            kernel::TypeForm::Function {
                parameters: vec![boolean],
                result: unit,
            },
        );
        let supported = Case::new("unused-supported-function-type", source, 0, Some(42));
        assert_same_state_different_carrier(&cases[0], &supported);
        cases.push(supported);

        let mut source = base.clone();
        let body = fixture_function(&source, "unused-result").body;
        expression_mut(&mut source, body).operation = E::Unit {};
        cases.push(Case::new("unused-wrong-return", source, 19, None));

        let mut source = base.clone();
        let body = fixture_function(&source, "choose").body;
        let when_false = match &expression(&source, body).operation {
            E::If { when_false, .. } => *when_false,
            _ => panic!("choose if"),
        };
        expression_mut(&mut source, when_false).operation = E::Bool { value: true };
        cases.push(Case::new("untaken-wrong-branch-type", source, 19, None));

        let mut source = base.clone();
        let body = fixture_function(&source, "choose").body;
        let condition = match &expression(&source, body).operation {
            E::If { condition, .. } => *condition,
            _ => panic!("choose if"),
        };
        expression_mut(&mut source, condition).operation = E::I64 { value: 1 };
        cases.push(Case::new("non-bool-condition", source, 19, None));

        let mut source = base.clone();
        let replacement = declaration_reference(&source, "zero-arity");
        let body = fixture_function(&source, "ordered-call").body;
        let E::Call { function, .. } = &mut expression_mut(&mut source, body).operation else {
            panic!("ordered call");
        };
        // Keep both argument owners and their containment: removing an argument
        // would create an E1 orphan and would not independently exercise E2 arity.
        *function = replacement;
        cases.push(Case::new("unused-wrong-call-arity", source, 19, None));

        let mut source = base.clone();
        let body = fixture_function(&source, "ordered-call").body;
        let E::Call { arguments, .. } = &mut expression_mut(&mut source, body).operation else {
            panic!("ordered call");
        };
        assert_eq!(arguments.len(), 2);
        arguments.swap(0, 1);
        cases.push(Case::new("unused-wrong-call-order", source, 19, None));

        let mut source = base.clone();
        let boolean = scalar_type(&source, kernel::TypeForm::Bool);
        let binding = entry_binding(&source, "unused-annotation");
        let OwnerRecord::Binding(record) =
            source.owners.get_mut(&OwnerKey::Binding(binding)).unwrap()
        else {
            panic!("annotation binding");
        };
        record.declared_type = Some(boolean);
        cases.push(Case::new(
            "unused-binding-annotation-mismatch",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        let selected = kernel::PortReference {
            package: source.root.package_id,
            port: port(&source, "other"),
        };
        target_mut(&mut source).port = Some(selected);
        cases.push(Case::new("target-crosses-components", source, 19, None));

        let mut source = base.clone();
        let replacement = declaration_reference(&source, "choose");
        let key = OwnerKey::Port(port(&source, "other"));
        let OwnerRecord::Port(record) = source.owners.get_mut(&key).unwrap() else {
            panic!("unused port");
        };
        record.implementation = kernel::PortImplementation::Function(replacement);
        cases.push(Case::new(
            "unused-port-signature-mismatch",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        insert_unused_type(&mut source, kernel::TypeForm::Bytes);
        let unsupported = Case::new("unused-unsupported-bytes-type", source, 13, None);
        assert_same_state_different_carrier(&cases[0], &unsupported);
        cases.push(unsupported);

        let pure = "(function create unused-unit (visibility public) (effect pure)";
        let task = "(function create unused-unit (visibility public) (effect (task))";
        assert_eq!(SCALAR_SOURCE.matches(pure).count(), 1);
        let source =
            byte_buffer_tests::author_only(&SCALAR_SOURCE.replacen(pure, task, 1)).unwrap();
        let function = fixture_function(&source, "unused-unit");
        let kernel::FunctionEffect::Task {
            requirements,
            effect_parameters,
        } = &function.effect
        else {
            panic!("empty task fixture");
        };
        assert!(requirements.is_empty());
        assert!(effect_parameters.is_empty());
        assert_eq!(
            source.owners[&OwnerKey::Declaration(declaration(&source, "unused-unit"))].kind(),
            kernel::OwnerKind::TaskFunction,
        );
        cases.push(Case::new("unused-empty-task-is-not-pure", source, 13, None));
        cases
    })
}

struct NativeCompiler {
    entry: &'static str,
    image: Vec<u8>,
    executable_sha256: String,
    source_carrier_sha256: &'static str,
}

fn compiler_carrier_sha256() -> &'static str {
    static DIGEST: OnceLock<String> = OnceLock::new();
    DIGEST.get_or_init(|| sha256(&graph::packet(&compiler().0)))
}

fn build_native_compiler(
    module: &'static str,
    name: &'static str,
    entry: &'static str,
) -> NativeCompiler {
    let (_, program) = compiler();
    let selected = program.function(super::function(module, name)).unwrap().0 as usize;
    let image = crate::platform::compiler::elf_x86_64::compile_function(program, selected)
        .unwrap()
        .0;
    let executable_sha256 = sha256(&image);
    let source_carrier_sha256 = compiler_carrier_sha256();
    eprintln!(
        "selfhost scalar producer entry={entry} bootstrap=rust_elf \
         source_carrier_sha256={source_carrier_sha256} executable_sha256={executable_sha256} \
         executable_bytes={} target=linux-x86_64",
        image.len(),
    );
    NativeCompiler {
        entry,
        image,
        executable_sha256,
        source_carrier_sha256,
    }
}

fn native_inspector() -> &'static NativeCompiler {
    static INSPECTOR: OnceLock<NativeCompiler> = OnceLock::new();
    INSPECTOR
        .get_or_init(|| build_native_compiler("source-scalar", "inspect", "source-scalar::inspect"))
}

fn native_compiler() -> &'static NativeCompiler {
    static COMPILER: OnceLock<NativeCompiler> = OnceLock::new();
    COMPILER.get_or_init(|| build_native_compiler("source-elf", "compile", "source-elf::compile"))
}

fn run_filter(compiler: &NativeCompiler, case: &Case) -> Vec<u8> {
    case.assert_e1();
    eprintln!(
        "selfhost scalar request case={} entry={} source_carrier_sha256={} \
         executable_sha256={} carrier_sha256={} carrier_bytes={} semantic_state_hex={} \
         input=LKJSG001 target=linux-x86_64 arguments=[] environment=empty \
         stdin_chunk_bytes=127 filter_deadline_seconds=10",
        case.name,
        compiler.entry,
        compiler.source_carrier_sha256,
        compiler.executable_sha256,
        case.carrier_sha256,
        case.carrier.len(),
        crate::platform::semantic_id::encode_hex(&case.carrier[48..80]),
    );
    let start = std::time::Instant::now();
    let output = native_filter_tests::execute_filter(&compiler.image, &case.carrier, &[], 127);
    eprintln!(
        "selfhost scalar response case={} entry={} executable_sha256={} \
         carrier_sha256={} exit={:?} output_sha256={} output_bytes={} elapsed_ms={}",
        case.name,
        compiler.entry,
        compiler.executable_sha256,
        case.carrier_sha256,
        output.status.code(),
        sha256(&output.stdout),
        output.stdout.len(),
        start.elapsed().as_millis(),
    );
    assert_eq!(
        output.status.code(),
        Some(if case.diagnostic == 0 { 0 } else { 65 }),
        "case={}, stderr={:?}",
        case.name,
        output.stderr,
    );
    if case.diagnostic == 0 {
        assert!(output.stderr.is_empty(), "case={}", case.name);
        assert!(!output.stdout.is_empty(), "case={}", case.name);
    } else {
        assert!(output.stdout.is_empty(), "case={}", case.name);
        assert_eq!(
            output.stderr,
            format!("compiler_rejected:{}\n", case.diagnostic).as_bytes(),
            "case={}",
            case.name,
        );
    }
    output.stdout
}

#[test]
fn selfhost_source_scalar_e2_checks_complete_carriers_in_vm_reference_and_native() {
    let inspector = native_inspector();
    for case in cases() {
        case.assert_e1();
        let value = super::invoke(
            "source-scalar",
            "inspect",
            vec![NormalizedValue::bytes(case.carrier.as_slice())],
        );
        assert_eq!(
            super::field(&value, "diagnostic"),
            &NormalizedValue::I64(case.diagnostic),
            "case={}",
            case.name,
        );
        let NormalizedValue::Bytes(image) = super::field(&value, "image") else {
            panic!("scalar inspection image");
        };
        let expected: &[u8] = if case.diagnostic == 0 {
            &case.carrier[48..80]
        } else {
            &[]
        };
        assert_eq!(image.as_ref(), expected, "case={}", case.name);
        assert_eq!(run_filter(inspector, case), expected, "case={}", case.name);
    }

    // Exercise the separate public diagnostic-only entry without repeating the
    // entire inspection matrix.
    let negative = cases()
        .iter()
        .find(|case| case.name == "unused-wrong-return")
        .unwrap();
    for case in [cases().first().unwrap(), negative] {
        assert_eq!(
            super::invoke(
                "source-scalar",
                "validate",
                vec![NormalizedValue::bytes(case.carrier.as_slice())],
            ),
            NormalizedValue::I64(case.diagnostic),
            "case={}",
            case.name,
        );
    }
}

#[test]
fn selfhost_source_scalar_e3_compiles_raw_carriers_and_executes_independent_results() {
    let compiler = native_compiler();
    for case in cases() {
        let image = run_filter(compiler, case);
        if case.name == "baseline" {
            // This one complete producer comparison covers the E3 passes and
            // backend on both independent language engines before execution.
            let value = super::invoke(
                "source-elf",
                "compile",
                vec![NormalizedValue::bytes(case.carrier.as_slice())],
            );
            assert_eq!(super::field(&value, "diagnostic"), &NormalizedValue::I64(0),);
            let NormalizedValue::Bytes(expected) = super::field(&value, "image") else {
                panic!("source compiler image");
            };
            assert_eq!(image.as_slice(), expected.as_ref());
        }
        if let Some(expected) = case.result {
            let start = std::time::Instant::now();
            let output = execute(&image, &[]);
            eprintln!(
                "selfhost scalar execution case={} compiler_sha256={} carrier_sha256={} \
                 program_sha256={} target=linux-x86_64 output=ELF64 normalized_profile=4 \
                 arguments=[] environment=empty \
                 program_deadline_seconds=5 exit={:?} stdout_sha256={} elapsed_ms={}",
                case.name,
                compiler.executable_sha256,
                case.carrier_sha256,
                sha256(&image),
                output.status.code(),
                sha256(&output.stdout),
                start.elapsed().as_millis(),
            );
            assert!(output.status.success(), "case={}", case.name);
            assert!(output.stderr.is_empty(), "case={}", case.name);
            assert_eq!(
                output.stdout,
                expected.to_string().as_bytes(),
                "case={}",
                case.name,
            );
        }
    }
}

#[test]
fn selfhost_source_scalar_e3_lowers_more_than_historical_node_limit_natively() {
    const HISTORICAL_MAX_NODES: usize = 32_768;
    let leaf_count = HISTORICAL_MAX_NODES / 2 + 1;

    fn append_sequence(proposal: &mut String, first: usize, leaves: usize, last: usize) {
        if leaves == 1 {
            proposal.push_str(if first == last {
                " (i64 42)"
            } else {
                " (i64 0)"
            });
            return;
        }
        // Choose the fewest children that retain the minimum tree depth, then
        // distribute leaves evenly. Each Sequence has between 2 and 16 children.
        let mut child_capacity = 1;
        while (leaves - 1) / child_capacity >= 16 {
            child_capacity *= 16;
        }
        let children = 1 + (leaves - 1) / child_capacity;
        let base = leaves / children;
        let larger = leaves % children;
        proposal.push_str(" (sequence");
        let mut next = first;
        for child in 0..children {
            let count = base + usize::from(child < larger);
            append_sequence(proposal, next, count, last);
            next += count;
        }
        proposal.push(')');
    }

    let mut proposal = String::from(
        "declarations.begin\n\
         (units (module create scalar-capacity\n\
          (function create entry (visibility public) (effect pure) (returns I64)\n\
           (body",
    );
    append_sequence(&mut proposal, 0, leaf_count, leaf_count - 1);
    proposal.push_str(
        "))\n\
          (component create app (visibility public)\n\
           (port create run (type (function () I64)) (function entry))))\n\
         (target create scalar-capacity-main (component scalar-capacity::app)\n\
          (runner command) (port scalar-capacity::app::run)))\n\
         declarations.end\n",
    );
    // Public authoring supplies the seed. The native source compiler receives
    // only the complete raw carrier; Rust does not normalize this target.
    // This large fixture selects its own bounded authoring policy. A Sequence
    // tree has fewer internal nodes than leaves, plus five declaration owners.
    // Summary planning has initial and final passes; the relation allowance is
    // a separate explicit fixture budget, not a product-default change.
    let owner_budget = u64::try_from(leaf_count)
        .unwrap()
        .checked_mul(2)
        .unwrap()
        .checked_add(5)
        .unwrap();
    let empty = empty_normalized_snapshot(b"owned-byte-buffer");
    let memory = crate::platform::MemProjectStore::new(128 * 1_048_576).unwrap();
    let repo = GraphRepository::create_memory(&memory, &empty, None)
        .unwrap()
        .repository;
    let base = repo.view_current().unwrap().revision();
    let request = format!("request base={base}\n{proposal}");
    let mut decoded =
        crate::platform::control::decode_compact_change("buffer", request.as_bytes()).unwrap();
    decoded.semantic.budget.impact.maximum_affected_owners = owner_budget;
    decoded.semantic.budget.impact.maximum_summary_owners = owner_budget.checked_mul(2).unwrap();
    decoded.semantic.budget.impact.maximum_summary_edits = owner_budget;
    decoded.semantic.budget.impact.maximum_behavior_owners = owner_budget;
    decoded.semantic.budget.impact.maximum_relation_edges = owner_budget.checked_mul(16).unwrap();
    let plan = repo
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    repo.publish(&plan.publication).unwrap();
    let source = repo
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    assert!(u64::try_from(source.owners.len()).unwrap() <= owner_budget);
    let carrier = graph::packet(&source);
    assert!(carrier.len() <= 8 * 1024 * 1024);
    let compiler = native_compiler();
    let started = std::time::Instant::now();
    // Deliberately bypass run_filter: this capacity control runs only natively,
    // without repeating the large carrier on the VM and reference interpreter.
    let output = native_filter_tests::execute_filter(&compiler.image, &carrier, &[], 4093);
    let compile_ms = started.elapsed().as_millis();
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    // Sequence entry/exit emit no nodes. Across the complete tree, sum(k - 1)
    // is leaf_count - 1, so literals, Drops and the final Return total 2*leaves.
    // Each Drop and Return has exactly one operand-state byte; there are no locals.
    let expected_nodes = 2 * leaf_count;
    let expected_state_bytes = leaf_count;
    assert_eq!(expected_nodes, 32_770);
    assert!(expected_nodes > HISTORICAL_MAX_NODES);
    assert_eq!(
        output.stdout.len(),
        65536 + 512 + expected_nodes * 128 + expected_state_bytes
    );
    assert_eq!(output.stdout.len(), 4_276_993);
    let program = execute(&output.stdout, &[]);
    assert_eq!(program.status.code(), Some(0), "{:?}", program.stderr);
    assert_eq!(program.stdout, b"42");
    assert!(program.stderr.is_empty());
    eprintln!(
        "selfhost scalar capacity bootstrap=rust_elf source_carrier_sha256={} \
         executable_sha256={} carrier_sha256={} carrier_bytes={} \
         expected_mode0_nodes={expected_nodes} expected_mode0_state_bytes={expected_state_bytes} \
         program_sha256={} program_bytes={} compile_ms={compile_ms} \
         source_shape=balanced_sequence_tree max_sequence_fanout=16 derived_sequence_levels=4 \
         input=LKJSG001 target=linux-x86_64 arguments=[] environment=empty \
         stdin_chunk_bytes=4093 filter_deadline_seconds=10",
        compiler.source_carrier_sha256,
        compiler.executable_sha256,
        sha256(&carrier),
        carrier.len(),
        sha256(&output.stdout),
        output.stdout.len(),
    );
}

fn assert_raw_scalar_e3_rejection(
    name: &str,
    source: &KernelSnapshot,
    links_diagnostic: i64,
    diagnostic: i64,
) {
    let carrier = graph::packet(source);
    let carrier_sha256 = sha256(&carrier);
    let argument = || vec![NormalizedValue::bytes(carrier.as_slice())];

    // The complete carrier is canonically encoded with recomputed commitments.
    // Passing the carrier validator excludes a digest/encoding rejection.
    assert_eq!(
        super::invoke("source-graph", "validate", argument()),
        NormalizedValue::I64(0),
        "carrier validation case={name} carrier_sha256={carrier_sha256}",
    );
    assert_eq!(
        super::invoke("source-links", "validate", argument()),
        NormalizedValue::I64(links_diagnostic),
        "reference validation case={name} carrier_sha256={carrier_sha256}",
    );
    if links_diagnostic == 0 {
        // Both cycle owners have exactly one incoming containment edge. Prove
        // that parent-table construction succeeds before forest validation.
        let prepared = super::invoke("source-members", "prepare", argument());
        assert_eq!(super::field(&prepared, "code"), &NormalizedValue::I64(0));
        assert_eq!(
            super::invoke("source-scope", "validate", argument()),
            NormalizedValue::I64(diagnostic),
            "forest validation case={name} carrier_sha256={carrier_sha256}",
        );
    }

    // invoke compares the VM and independent language reference interpreter.
    // Neither receives a Rust admission result or normalized target program.
    let value = super::invoke("source-elf", "compile", argument());
    assert_eq!(
        super::field(&value, "diagnostic"),
        &NormalizedValue::I64(diagnostic),
        "source compiler case={name} carrier_sha256={carrier_sha256}",
    );
    let NormalizedValue::Bytes(image) = super::field(&value, "image") else {
        panic!("raw source rejection image");
    };
    assert!(image.is_empty(), "case={name}");

    let compiler = native_compiler();
    eprintln!(
        "selfhost scalar raw request case={name} construction=independent-owner-records \
         entry={} source_carrier_sha256={} executable_sha256={} \
         carrier_sha256={carrier_sha256} carrier_bytes={} semantic_state_hex={} \
         input=LKJSG001 target=linux-x86_64 arguments=[] environment=empty \
         stdin_chunk_bytes=127 filter_deadline_seconds=10",
        compiler.entry,
        compiler.source_carrier_sha256,
        compiler.executable_sha256,
        carrier.len(),
        crate::platform::semantic_id::encode_hex(&carrier[48..80]),
    );
    let start = std::time::Instant::now();
    let output = native_filter_tests::execute_filter(&compiler.image, &carrier, &[], 127);
    eprintln!(
        "selfhost scalar raw response case={name} entry={} executable_sha256={} \
         carrier_sha256={carrier_sha256} diagnostic={diagnostic} exit={:?} \
         output_sha256={} output_bytes={} elapsed_ms={}",
        compiler.entry,
        compiler.executable_sha256,
        output.status.code(),
        sha256(&output.stdout),
        output.stdout.len(),
        start.elapsed().as_millis(),
    );
    assert_eq!(output.status.code(), Some(65), "case={name}");
    assert!(output.stdout.is_empty(), "case={name}");
    assert_eq!(
        output.stderr,
        format!("compiler_rejected:{diagnostic}\n").as_bytes(),
        "case={name}",
    );
}

#[test]
fn selfhost_source_scalar_e3_rejects_hash_consistent_missing_expression_endpoint() {
    const SEED: &[u8] = b"scalar-raw-missing-expression";
    let mut source = crate::platform::execution::normalized::tests::empty_normalized_snapshot(SEED);
    let owner = ExpressionId::migrate(SEED, 0);
    let missing = ExpressionId::migrate(SEED, 1);
    source.owners.insert(
        OwnerKey::Expression(owner),
        OwnerRecord::Expression(
            kernel::ExpressionRecord::new(
                owner,
                E::Sequence {
                    items: vec![missing],
                },
            )
            .unwrap(),
        ),
    );
    assert!(!source.owners.contains_key(&OwnerKey::Expression(missing)));
    assert_raw_scalar_e3_rejection("raw-missing-expression-endpoint", &source, 14, 14);
}

#[test]
fn selfhost_source_scalar_e3_rejects_hash_consistent_disconnected_expression_cycle() {
    const SEED: &[u8] = b"scalar-raw-disconnected-expression-cycle";
    let mut source = crate::platform::execution::normalized::tests::empty_normalized_snapshot(SEED);
    let left = ExpressionId::migrate(SEED, 0);
    let right = ExpressionId::migrate(SEED, 1);
    for (owner, child) in [(left, right), (right, left)] {
        assert!(
            source
                .owners
                .insert(
                    OwnerKey::Expression(owner),
                    OwnerRecord::Expression(
                        kernel::ExpressionRecord::new(owner, E::Sequence { items: vec![child] })
                            .unwrap(),
                    ),
                )
                .is_none()
        );
    }
    assert_eq!(source.owners.len(), 2);
    assert_raw_scalar_e3_rejection("raw-disconnected-expression-cycle", &source, 0, 15);
}
