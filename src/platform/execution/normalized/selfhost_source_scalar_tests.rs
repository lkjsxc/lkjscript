//! Whole-carrier scalar E2 controls and source-driven E3 native execution.
//! Rust authors the valid seed and re-encodes complete derived carriers. It performs
//! no target semantic admission, normalization, or lowering before the .lkjc compiler.
//! The maintained compiler is bootstrapped once per entry with the Rust backend;
//! these controls do not establish maintained-compiler C2/C3 fixed points.
use super::*;
use crate::platform::kernel::ExpressionOperation as E;
use crate::platform::semantic_id::{BindingId, DeclarationId, ExpressionId, PortId, TargetId};

const SCALAR_SOURCE: &str = include_str!("../../../../tests/fixtures/selfhost-source-scalar.lkjc");
const INTRINSIC_SOURCE: &str =
    include_str!("../../../../tests/fixtures/selfhost-source-scalar-intrinsics.lkjc");

#[derive(Clone, Copy)]
enum ScalarResult {
    Value(i64),
    Trap(&'static str),
}

struct Case {
    name: &'static str,
    carrier: Vec<u8>,
    carrier_sha256: String,
    selected_target: TargetId,
    request: Vec<u8>,
    request_sha256: String,
    diagnostic: i64,
    result: Option<ScalarResult>,
    e1: OnceLock<()>,
}

impl Case {
    fn new(
        name: &'static str,
        source: KernelSnapshot,
        diagnostic: i64,
        result: Option<i64>,
    ) -> Self {
        Self::with_result(name, source, diagnostic, result.map(ScalarResult::Value))
    }

    fn with_result(
        name: &'static str,
        source: KernelSnapshot,
        diagnostic: i64,
        result: Option<ScalarResult>,
    ) -> Self {
        assert_eq!(diagnostic == 0, result.is_some());
        let selected_target = target_id(&source, "scalar-main");
        let carrier = graph::packet(&source);
        let carrier_sha256 = sha256(&carrier);
        let request = source_request(&carrier, selected_target);
        let request_sha256 = sha256(&request);
        Self {
            name,
            carrier,
            carrier_sha256,
            selected_target,
            request,
            request_sha256,
            diagnostic,
            result,
            e1: OnceLock::new(),
        }
    }

    fn select(mut self, selected_target: TargetId) -> Self {
        self.selected_target = selected_target;
        self.request = source_request(&self.carrier, selected_target);
        self.request_sha256 = sha256(&self.request);
        self
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

// Resolve a fixture label to the caller's authored identity only. No host
// component, port, function or normalized-entry selection occurs here.
pub(super) fn target_id(source: &KernelSnapshot, name: &str) -> TargetId {
    source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Target(id), OwnerRecord::Target(record)) if record.name.as_str() == name => {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}

pub(super) fn source_request(carrier: &[u8], selected_target: TargetId) -> Vec<u8> {
    let capacity = 32usize
        .checked_add(carrier.len())
        .expect("source request size");
    let mut request = Vec::with_capacity(capacity);
    request.extend_from_slice(b"LKJSRQ01");
    request.extend_from_slice(&1u32.to_le_bytes());
    request.extend_from_slice(
        &kernel::EncodedOwnerKey::new(OwnerKey::Target(selected_target)).bytes(),
    );
    request.extend_from_slice(&[0; 3]);
    assert_eq!(request.len(), 32);
    request.extend_from_slice(carrier);
    request
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
        let other = Case::new("same-carrier-other-target", base.clone(), 0, Some(7))
            .select(target_id(base, "scalar-other"));
        assert_eq!(cases[0].carrier, other.carrier);
        assert_eq!(cases[0].carrier_sha256, other.carrier_sha256);
        assert_ne!(cases[0].request, other.request);
        assert_ne!(cases[0].request_sha256, other.request_sha256);
        assert_ne!(cases[0].selected_target, other.selected_target);
        cases.push(other);

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
        let wrong_port = kernel::PortReference {
            package: source.root.package_id,
            port: port(&source, "app"),
        };
        let key = OwnerKey::Target(target_id(&source, "scalar-other"));
        let OwnerRecord::Target(record) = source.owners.get_mut(&key).unwrap() else {
            panic!("unselected fixture target");
        };
        record.port = Some(wrong_port);
        cases.push(Case::new(
            "unselected-target-crosses-components",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        let replacement = declaration_reference(&source, "choose");
        let descriptor = kernel::TypeObject::new(kernel::TypeForm::Function {
            parameters: vec![
                scalar_type(&source, kernel::TypeForm::Bool),
                scalar_type(&source, kernel::TypeForm::I64),
            ],
            result: scalar_type(&source, kernel::TypeForm::I64),
        })
        .unwrap();
        let digest = kernel::encode_type_object(&descriptor).unwrap().0;
        source.types.insert(digest, descriptor);
        let key = OwnerKey::Port(port(&source, "other"));
        let OwnerRecord::Port(record) = source.owners.get_mut(&key).unwrap() else {
            panic!("unselected fixture port");
        };
        record.function_type = digest;
        record.implementation = kernel::PortImplementation::Function(replacement);
        cases.push(Case::new(
            "unselected-target-unsupported-entry",
            source,
            13,
            None,
        ));

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
        cases.extend(intrinsic_cases());
        cases
    })
}

fn external_mut<'a>(
    source: &'a mut KernelSnapshot,
    name: &str,
) -> &'a mut kernel::ExternalDeclaration {
    let id = declaration(source, name);
    let OwnerRecord::Declaration(record) =
        source.owners.get_mut(&OwnerKey::Declaration(id)).unwrap()
    else {
        panic!("intrinsic declaration");
    };
    let kernel::DeclarationPayload::External(external) = &mut record.payload else {
        panic!("intrinsic external");
    };
    external
}

fn intrinsic_cases() -> Vec<Case> {
    // Author one valid complete carrier. Selection changes the one port's target
    // without removing any external, function, expression, or supplied type.
    let base = byte_buffer_tests::author_only(INTRINSIC_SOURCE).unwrap();
    let mut cases = Vec::new();
    let outcomes = [
        (
            "intrinsic-arithmetic",
            "arithmetic",
            ScalarResult::Value(42),
        ),
        (
            "intrinsic-truncation",
            "truncation",
            ScalarResult::Value(-13),
        ),
        (
            "intrinsic-false-relations",
            "false-relations",
            ScalarResult::Value(0),
        ),
        (
            "intrinsic-minimum-division",
            "minimum-division",
            ScalarResult::Value(i64::MIN),
        ),
        (
            "intrinsic-maximum-addition",
            "maximum-addition",
            ScalarResult::Value(i64::MAX),
        ),
        (
            "intrinsic-recursion",
            "recursive-entry",
            ScalarResult::Value(42),
        ),
        (
            "intrinsic-add-overflow",
            "add-trap",
            ScalarResult::Trap("integer_overflow"),
        ),
        (
            "intrinsic-subtract-overflow",
            "subtract-trap",
            ScalarResult::Trap("integer_overflow"),
        ),
        (
            "intrinsic-multiply-overflow",
            "multiply-trap",
            ScalarResult::Trap("integer_overflow"),
        ),
        (
            "intrinsic-divide-zero",
            "divide-zero",
            ScalarResult::Trap("integer_division"),
        ),
        (
            "intrinsic-divide-overflow",
            "divide-overflow",
            ScalarResult::Trap("integer_division"),
        ),
        (
            "intrinsic-strict-and",
            "strict-and",
            ScalarResult::Trap("integer_overflow"),
        ),
        (
            "intrinsic-strict-or",
            "strict-or",
            ScalarResult::Trap("integer_overflow"),
        ),
        (
            "intrinsic-left-before-right",
            "left-before-right",
            ScalarResult::Trap("integer_division"),
        ),
        (
            "intrinsic-operand-before-intrinsic",
            "operand-before-intrinsic",
            ScalarResult::Trap("integer_overflow"),
        ),
    ];
    for (name, selected, result) in outcomes {
        let mut source = base.clone();
        let function = declaration_reference(&source, selected);
        let key = OwnerKey::Port(port(&source, "main"));
        let OwnerRecord::Port(record) = source.owners.get_mut(&key).unwrap() else {
            panic!("intrinsic fixture port");
        };
        record.implementation = kernel::PortImplementation::Function(function);
        cases.push(Case::with_result(name, source, 0, Some(result)));
    }

    for (name, implementation, expected) in [
        ("unused-unknown-intrinsic", "core.i64.unknown", 13),
        ("unused-outside-scalar-intrinsic", "core.i64.less-equal", 13),
        ("unused-forged-intrinsic-signature", "core.bool.and", 19),
    ] {
        let mut source = base.clone();
        external_mut(&mut source, "unused-sum").implementation =
            kernel::ImplementationName::new(implementation).unwrap();
        cases.push(Case::new(name, source, expected, None));
    }

    let mut source = base.clone();
    let boolean = scalar_type(&source, kernel::TypeForm::Bool);
    external_mut(&mut source, "unused-sum").result = boolean;
    cases.push(Case::new("unused-intrinsic-wrong-result", source, 19, None));

    let mut source = base.clone();
    let boolean = scalar_type(&source, kernel::TypeForm::Bool);
    let parameter = external_mut(&mut source, "unused-sum").parameters[1];
    let OwnerRecord::Parameter(record) = source
        .owners
        .get_mut(&OwnerKey::Parameter(parameter))
        .unwrap()
    else {
        panic!("intrinsic fixture parameter");
    };
    record.ty = boolean;
    cases.push(Case::new(
        "unused-intrinsic-wrong-second-parameter",
        source,
        19,
        None,
    ));

    let mut source = base.clone();
    let parameter = external_mut(&mut source, "unused-sum")
        .parameters
        .pop()
        .unwrap();
    assert!(
        source
            .owners
            .remove(&OwnerKey::Parameter(parameter))
            .is_some()
    );
    cases.push(Case::new("unused-intrinsic-wrong-arity", source, 19, None));

    for (name, use_mode) in [
        ("unused-intrinsic-borrow", kernel::ParameterUse::Borrow),
        ("unused-intrinsic-consume", kernel::ParameterUse::Consume),
    ] {
        let mut source = base.clone();
        let parameter = external_mut(&mut source, "unused-sum").parameters[0];
        let OwnerRecord::Parameter(record) = source
            .owners
            .get_mut(&OwnerKey::Parameter(parameter))
            .unwrap()
        else {
            panic!("intrinsic fixture custody");
        };
        record.use_mode = use_mode;
        cases.push(Case::new(name, source, 13, None));
    }

    let mut source = base.clone();
    let declaration = declaration(&source, "unused-sum");
    let parameter = crate::platform::semantic_id::TypeParameterId::migrate(
        b"scalar-intrinsic-unused-type-parameter",
        0,
    );
    external_mut(&mut source, "unused-sum")
        .type_parameters
        .push(parameter);
    assert!(
        source
            .owners
            .insert(
                OwnerKey::TypeParameter(parameter),
                OwnerRecord::TypeParameter(kernel::TypeParameterRecord {
                    header: kernel::OwnerHeader::new(
                        OwnerKey::TypeParameter(parameter),
                        kernel::OwnerKind::TypeParameter,
                    ),
                    declaration,
                    name: kernel::Name::new("Extra").unwrap(),
                    constraints: kernel::TypeParameterConstraints::None,
                }),
            )
            .is_none()
    );
    cases.push(Case::new(
        "unused-intrinsic-generic-binder",
        source,
        13,
        None,
    ));

    let mut source = base.clone();
    let body = fixture_function(&source, "unused-call").body;
    let argument = match &expression(&source, body).operation {
        E::Call { arguments, .. } => arguments[1],
        _ => panic!("unused intrinsic call"),
    };
    expression_mut(&mut source, argument).operation = E::Bool { value: false };
    cases.push(Case::new("unused-intrinsic-call-type", source, 19, None));

    let mut source = base.clone();
    let callee = declaration_reference(&source, "sum");
    let body = fixture_function(&source, "unused-zero-call").body;
    let E::Call { function, .. } = &mut expression_mut(&mut source, body).operation else {
        panic!("unused zero-arity call");
    };
    *function = callee;
    cases.push(Case::new("unused-intrinsic-call-arity", source, 19, None));
    cases
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

fn run_filter(compiler: &NativeCompiler, case: &Case, input: &[u8], input_format: &str) -> Vec<u8> {
    case.assert_e1();
    eprintln!(
        "selfhost scalar request case={} entry={} source_carrier_sha256={} \
         executable_sha256={} carrier_sha256={} carrier_bytes={} semantic_state_hex={} \
         request_sha256={} selected_target={} input={input_format} stdin_sha256={} \
         target=linux-x86_64 arguments=[] environment=empty \
         stdin_chunk_bytes=127 filter_deadline_seconds=10",
        case.name,
        compiler.entry,
        compiler.source_carrier_sha256,
        compiler.executable_sha256,
        case.carrier_sha256,
        case.carrier.len(),
        crate::platform::semantic_id::encode_hex(&case.carrier[48..80]),
        case.request_sha256,
        case.selected_target,
        sha256(input),
    );
    let start = std::time::Instant::now();
    let output = native_filter_tests::execute_filter(&compiler.image, input, &[], 127);
    eprintln!(
        "selfhost scalar response case={} entry={} executable_sha256={} \
         carrier_sha256={} request_sha256={} selected_target={} \
         exit={:?} output_sha256={} output_bytes={} elapsed_ms={}",
        case.name,
        compiler.entry,
        compiler.executable_sha256,
        case.carrier_sha256,
        case.request_sha256,
        case.selected_target,
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
        assert_eq!(
            run_filter(inspector, case, &case.carrier, "LKJSG001"),
            expected,
            "case={}",
            case.name
        );
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
fn selfhost_source_scalar_e3_compiles_selected_requests_and_executes_independent_results() {
    let compiler = native_compiler();
    for case in cases() {
        let image = run_filter(compiler, case, &case.request, "LKJSRQ01/profile1");
        if case.name == "same-carrier-other-target" {
            assert_eq!(
                image,
                run_filter(compiler, case, &case.request, "LKJSRQ01/profile1"),
            );
        }
        if matches!(
            case.name,
            "baseline" | "same-carrier-other-target" | "intrinsic-arithmetic"
        ) {
            // These complete producer comparisons cover the E3 passes and
            // backend on both independent language engines before execution.
            let value = super::invoke(
                "source-elf",
                "compile",
                vec![NormalizedValue::bytes(case.request.as_slice())],
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
                 request_sha256={} selected_target={} \
                 program_sha256={} target=linux-x86_64 output=ELF64 normalized_profile=4 \
                 arguments=[] environment=empty \
                 program_deadline_seconds=5 exit={:?} stdout_sha256={} elapsed_ms={}",
                case.name,
                compiler.executable_sha256,
                case.carrier_sha256,
                case.request_sha256,
                case.selected_target,
                sha256(&image),
                output.status.code(),
                sha256(&output.stdout),
                start.elapsed().as_millis(),
            );
            match expected {
                ScalarResult::Value(expected) => {
                    assert!(output.status.success(), "case={}", case.name);
                    assert!(output.stderr.is_empty(), "case={}", case.name);
                    assert_eq!(
                        output.stdout,
                        expected.to_string().as_bytes(),
                        "case={}",
                        case.name,
                    );
                }
                ScalarResult::Trap(code) => {
                    assert_eq!(output.status.code(), Some(70), "case={}", case.name);
                    assert!(output.stdout.is_empty(), "case={}", case.name);
                    assert_eq!(
                        output.stderr,
                        format!("normalized_{code}\n").as_bytes(),
                        "case={}",
                        case.name,
                    );
                }
            }
        }
    }
    check_request_controls(compiler);
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
    // the complete carrier and caller's typed target selector; Rust does not normalize it.
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
    let selected_target = target_id(&source, "scalar-capacity-main");
    let request = source_request(&carrier, selected_target);
    assert!(carrier.len() <= 8 * 1024 * 1024);
    let compiler = native_compiler();
    let started = std::time::Instant::now();
    // Deliberately bypass run_filter: this capacity control runs only natively,
    // without repeating the large carrier on the VM and reference interpreter.
    let output = native_filter_tests::execute_filter(&compiler.image, &request, &[], 4093);
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
         input=LKJSRQ01 request_profile=1 request_sha256={} selected_target={} \
         target=linux-x86_64 arguments=[] environment=empty \
         stdin_chunk_bytes=4093 filter_deadline_seconds=10",
        compiler.source_carrier_sha256,
        compiler.executable_sha256,
        sha256(&carrier),
        carrier.len(),
        sha256(&output.stdout),
        output.stdout.len(),
        sha256(&request),
        selected_target,
    );
}

fn assert_request_rejection(
    compiler: &NativeCompiler,
    name: &str,
    carrier_fixture: &[u8],
    request: &[u8],
    diagnostic: i64,
) {
    let value = super::invoke(
        "source-elf",
        "compile",
        vec![NormalizedValue::bytes(request)],
    );
    assert_eq!(
        super::field(&value, "diagnostic"),
        &NormalizedValue::I64(diagnostic),
        "request control={name}",
    );
    let NormalizedValue::Bytes(image) = super::field(&value, "image") else {
        panic!("request rejection image");
    };
    assert!(image.is_empty(), "request control={name}");
    let selector = request.get(12..29).unwrap_or(&[]);
    let payload = request.get(32..).unwrap_or(&[]);
    eprintln!(
        "selfhost scalar request control={name} source_carrier_sha256={} \
         executable_sha256={} carrier_fixture_sha256={} request_sha256={} \
         request_payload_sha256={} selector_hex={} request_bytes={} \
         input=LKJSRQ01 expected_request_profile=1 diagnostic={diagnostic} \
         target=linux-x86_64 arguments=[] environment=empty \
         stdin_chunk_bytes=127 filter_deadline_seconds=10",
        compiler.source_carrier_sha256,
        compiler.executable_sha256,
        sha256(carrier_fixture),
        sha256(request),
        sha256(payload),
        crate::platform::semantic_id::encode_hex(selector),
        request.len(),
    );
    let output = native_filter_tests::execute_filter(&compiler.image, request, &[], 127);
    assert_eq!(
        output.status.code(),
        Some(65),
        "request control={name}: {:?}",
        output.stderr
    );
    assert!(output.stdout.is_empty(), "request control={name}");
    assert_eq!(
        output.stderr,
        format!("compiler_rejected:{diagnostic}\n").as_bytes(),
        "request control={name}",
    );
}

fn check_request_controls(compiler: &NativeCompiler) {
    let baseline = &cases()[0];
    let carrier = baseline.carrier.as_slice();
    let valid = baseline.request.as_slice();
    let reject = |name: &str, request: &[u8], diagnostic| {
        assert_request_rejection(compiler, name, carrier, request, diagnostic)
    };

    reject("legacy-raw-only", carrier, 12);
    for length in [0, 8, 12, 28, 31, 32] {
        reject(&format!("request-prefix-{length}"), &valid[..length], 12);
    }
    let mut changed = valid.to_vec();
    changed[0] ^= 1;
    reject("request-wrong-magic", &changed, 12);
    let mut changed = valid.to_vec();
    changed[8..12].copy_from_slice(&2u32.to_le_bytes());
    reject("request-unknown-profile", &changed, 12);
    let mut changed = valid.to_vec();
    // Keep the live Target's identity bytes: a lookup that ignores the tag
    // would otherwise resolve the valid target and silently accept this request.
    changed[12] = 2;
    reject("request-wrong-owner-domain", &changed, 12);
    let mut changed = valid.to_vec();
    changed[13..29].fill(0);
    reject("request-zero-identity", &changed, 12);
    for offset in 29..32 {
        let mut changed = valid.to_vec();
        changed[offset] = 1;
        reject(&format!("request-reserved-{offset}"), &changed, 12);
    }
    reject("request-truncated-payload", &valid[..valid.len() - 1], 12);
    let mut changed = valid.to_vec();
    changed.push(0);
    reject("request-trailing-payload", &changed, 12);

    let missing = TargetId::migrate(b"scalar-request-missing-target", 0);
    assert!(!fixture().owners.contains_key(&OwnerKey::Target(missing)));
    assert!(
        !fixture()
            .retirements
            .contains_key(&OwnerKey::Target(missing))
    );
    reject(
        "request-missing-live-target",
        &source_request(carrier, missing),
        19,
    );

    let mut retired_source = fixture().clone();
    let retired = TargetId::migrate(b"scalar-request-retired-target", 0);
    let key = OwnerKey::Target(retired);
    assert!(!retired_source.owners.contains_key(&key));
    assert!(
        retired_source
            .retirements
            .insert(
                key,
                kernel::RetirementRecord {
                    graph_contract_version: 1,
                    owner: key,
                    last_kind: kernel::OwnerKind::Target,
                    last_name: None,
                    last_parent: None,
                    last_live_revision: crate::platform::semantic_id::RevisionId::from_digest(
                        [31; 32]
                    ),
                    deletion_change: kernel::ChangeDigest::from_bytes([41; 32]),
                },
            )
            .is_none()
    );
    let retired_carrier = graph::packet(&retired_source);
    assert_eq!(
        super::invoke(
            "source-scalar",
            "validate",
            vec![NormalizedValue::bytes(retired_carrier.as_slice())]
        ),
        NormalizedValue::I64(0),
        "retirement control reaches target selection after complete E2",
    );
    assert_request_rejection(
        compiler,
        "request-retired-target",
        &retired_carrier,
        &source_request(&retired_carrier, retired),
        19,
    );

    let unsupported = cases()
        .iter()
        .find(|case| case.name == "unused-unsupported-bytes-type")
        .unwrap();
    assert_request_rejection(
        compiler,
        "request-whole-inventory-before-missing-selector",
        &unsupported.carrier,
        &source_request(&unsupported.carrier, missing),
        13,
    );

    let mut no_targets = fixture().clone();
    no_targets
        .owners
        .retain(|key, _| !matches!(key, OwnerKey::Target(_)));
    let no_target_carrier = graph::packet(&no_targets);
    assert_request_rejection(
        compiler,
        "request-no-target-inventory",
        &no_target_carrier,
        &source_request(&no_target_carrier, baseline.selected_target),
        13,
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
    let selected_target = TargetId::migrate(b"raw-scalar-control-selector", 0);
    let request = source_request(&carrier, selected_target);
    let request_sha256 = sha256(&request);

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
    let value = super::invoke(
        "source-elf",
        "compile",
        vec![NormalizedValue::bytes(request.as_slice())],
    );
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
         request_sha256={request_sha256} selected_target={selected_target} \
         input=LKJSRQ01 request_profile=1 target=linux-x86_64 arguments=[] environment=empty \
         stdin_chunk_bytes=127 filter_deadline_seconds=10",
        compiler.entry,
        compiler.source_carrier_sha256,
        compiler.executable_sha256,
        carrier.len(),
        crate::platform::semantic_id::encode_hex(&carrier[48..80]),
    );
    let start = std::time::Instant::now();
    let output = native_filter_tests::execute_filter(&compiler.image, &request, &[], 127);
    eprintln!(
        "selfhost scalar raw response case={name} entry={} executable_sha256={} \
         carrier_sha256={carrier_sha256} request_sha256={request_sha256} \
         selected_target={selected_target} diagnostic={diagnostic} exit={:?} \
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
