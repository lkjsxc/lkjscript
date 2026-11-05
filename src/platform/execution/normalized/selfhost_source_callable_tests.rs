//! Closed pure callable source controls. Every generated compiler consumes the
//! complete raw graph before C0 builds an independent output oracle.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
use super::artifact_support::NativeOutput;
use super::*;
use crate::platform::compiler::{CompilationPayload, CompiledCode, CompiledInstruction};
use crate::platform::execution::normalized::prepare::{
    NormalizedCode, NormalizedFunctionBody, NormalizedInstruction,
};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, Stdio};

#[path = "selfhost_source_callable_stack_tests.rs"]
mod reference_stack;

const SOURCE: &str = include_str!("../../../../tests/fixtures/selfhost-source-callable.lkjc");
const MODULE: &str = "callable-controls";
const TARGET: &str = "callable-binary";

#[derive(Clone, Copy)]
struct Behavior {
    name: &'static str,
    selector: i64,
    count: i64,
    expected: i64,
    call_depth: Option<usize>,
}

impl Behavior {
    fn new(name: &'static str, selector: i64, count: i64, expected: i64) -> Self {
        Self {
            name,
            selector,
            count,
            expected,
            call_depth: None,
        }
    }

    fn input(self) -> Vec<u8> {
        [self.selector.to_le_bytes(), self.count.to_le_bytes()].concat()
    }
}

fn fixture() -> Check<&'static KernelSnapshot> {
    static SOURCE_GRAPH: OnceLock<Check<KernelSnapshot>> = OnceLock::new();
    SOURCE_GRAPH
        .get_or_init(|| byte_buffer_tests::author_only(SOURCE))
        .as_ref()
        .map_err(Clone::clone)
}

fn behaviors() -> Vec<Behavior> {
    let mut cases = vec![
        Behavior::new("higher-order-record-and-retained-alias", 0, 0, 84),
        Behavior::new("ordered-prefix-and-rebinding", 1, 0, 246),
        Behavior::new("named-empty-bind", 2, 0, 7),
        Behavior::new("closed-generic-external-values", 3, 0, 11),
        Behavior::new("copyable-capture-universe", 4, 0, 282),
        Behavior::new("live-buffer-invoke-cleanup", 5, 0, 7),
        Behavior::new("consumed-buffer-tail-invoke", 6, 0, 7),
        Behavior::new("non-tail-sensitivity-small", 10, 32, 32),
        Behavior::new("already-bound-empty-bind", 11, 0, 42),
        Behavior::new("named-nullary-tail-invoke", 12, 0, 7),
        Behavior::new("external-tail-invoke", 13, 0, 42),
        Behavior::new("untaken-valid-invoke", 14, 0, 42),
        Behavior::new("exact-static-text-return", 15, 0, 42),
        Behavior::new("distinct-semantic-shared-execution-shapes", 16, 0, 42),
    ];
    for count in [255, 256, 257, 2048] {
        for (name, selector) in [
            ("fn0-environment-depth", 7),
            ("record-fn0-environment-depth", 8),
        ] {
            cases.push(Behavior::new(name, selector, count, 2 * (7 + count)));
        }
    }
    for count in [0, 1, 2, 257] {
        cases.push(Behavior {
            name: "indirect-varying-frames",
            selector: 9,
            count,
            expected: 110 + 11 * (count / 2) + count % 2,
            call_depth: Some(16),
        });
    }
    cases
}

fn normalized_code<'a>(
    program: &'a NormalizedProgram,
    source: &KernelSnapshot,
    name: &str,
) -> Check<&'a NormalizedCode> {
    let entry = function_named(source, MODULE, name)?;
    let index = usize::try_from(
        program
            .function(entry)
            .ok_or_else(|| format!("missing normalized {name}"))?
            .0,
    )
    .map_err(|_| format!("normalized index {name}"))?;
    match &program
        .functions
        .get(index)
        .ok_or_else(|| format!("normalized function index {name}"))?
        .body
    {
        NormalizedFunctionBody::Code(code) => Ok(code),
        NormalizedFunctionBody::External(_) => Err(format!("{name} unexpectedly external")),
    }
}

fn canonical_unit(
    loaded: &LoadedArtifact,
    source: &KernelSnapshot,
    name: &str,
) -> Check<CompilationUnit> {
    let entry = function_named(source, MODULE, name)?;
    for (key, bytes) in &loaded.objects {
        if key.domain != ObjectDomain::CompilerUnit {
            continue;
        }
        let unit = CompilationUnit::decode(bytes, *key)
            .map_err(|error| format!("{name}: canonical unit decode: {error:?}"))?;
        if unit.source.owner == OwnerKey::Declaration(entry.declaration) {
            return Ok(unit);
        }
    }
    Err(format!("canonical function unit absent: {name}"))
}

fn function_code(unit: &CompilationUnit) -> Check<&CompiledCode> {
    match &unit.payload {
        CompilationPayload::Function { code, .. } => Ok(code),
        _ => Err("canonical callable unit has non-function payload".to_owned()),
    }
}

fn canonical_markers(
    source: &KernelSnapshot,
    loaded: &LoadedArtifact,
    program: &NormalizedProgram,
) -> Check<()> {
    use CompiledInstruction as I;
    let empty = canonical_unit(loaded, source, "named-empty")?;
    ensure(
        matches!(
            function_code(&empty)?.instructions.as_slice(),
            [
                I::FunctionValue { .. },
                I::BeginBind { arguments: 0 },
                I::Bind { arguments: 0 },
                I::Invoke { arguments: 0 },
                I::Return,
            ]
        ),
        "empty Bind lost a canonical instruction or PC",
    )?;
    let prefix = canonical_unit(loaded, source, "prefixes")?;
    let markers = function_code(&prefix)?
        .instructions
        .iter()
        .enumerate()
        .filter_map(|(pc, instruction)| match instruction {
            I::BeginBind { arguments } => Some((pc, "begin", *arguments)),
            I::Capture { index } => Some((pc, "capture", *index)),
            I::Bind { arguments } => Some((pc, "bind", *arguments)),
            _ => None,
        })
        .collect::<Vec<_>>();
    equal(
        markers,
        vec![
            (1, "begin", 1),
            (3, "capture", 0),
            (4, "bind", 1),
            (5, "begin", 1),
            (7, "capture", 0),
            (8, "bind", 1),
            (12, "begin", 3),
            (14, "capture", 0),
            (16, "capture", 1),
            (18, "capture", 2),
            (19, "bind", 3),
        ],
        "canonical prefix/capture PCs",
    )?;
    let generic = canonical_unit(loaded, source, "generic")?;
    let integer = source
        .types
        .iter()
        .find_map(|(id, ty)| matches!(ty.form, kernel::TypeForm::I64).then_some(*id))
        .ok_or("fixture I64 type absent")?;
    let mut closed_values = 0usize;
    for instruction in &function_code(&generic)?.instructions {
        if let I::FunctionValue { type_arguments, .. } = instruction {
            equal(type_arguments.len(), 1, "closed list FunctionValue arity")?;
            let slot = usize::try_from(*type_arguments.first().ok_or("closed type missing")?)
                .map_err(|_| "closed type index")?;
            equal(
                generic.tables.types.get(slot).copied(),
                Some(integer),
                "canonical supplied I64 substitution",
            )?;
            closed_values += 1;
        }
    }
    equal(
        closed_values,
        2,
        "list-get/list-size named values inventoried",
    )?;

    let live_unit = canonical_unit(loaded, source, "owned-live")?;
    let live_code = function_code(&live_unit)?;
    equal(live_code.parameter_count, 1, "owned-live parameter count")?;
    equal(live_code.local_count, 2, "owned-live buffer local count")?;
    let [
        I::Call {
            function: constructor,
            arguments: 0,
            ..
        },
        I::StoreLocal(1),
        I::LoadLocal { local: 0, .. },
        I::Invoke { arguments: 0 },
        I::Unit,
        I::StoreLocal(1),
        I::Return,
    ] = live_code.instructions.as_slice()
    else {
        return Err("owned-live must retain its initialized buffer through Invoke and clear that slot before Return".to_owned());
    };
    let constructor = usize::try_from(*constructor).map_err(|_| "buffer constructor index")?;
    let empty = function_named(source, MODULE, "empty")?;
    equal(
        live_unit.tables.declarations.get(constructor),
        Some(&empty),
        "owned-live initializes the exact ByteBuffer constructor",
    )?;

    // Preparation marks a terminal continuation across Unit/StoreLocal. This
    // hint retains the cleanup instructions: native dispatch must inspect live
    // owners and fall back to an ordinary call before this same continuation.
    // Drop discards an operand and would not clear the owning local here.
    let live = normalized_code(program, source, "owned-live")?;
    equal(
        live.parameter_count,
        1,
        "prepared owned-live parameter count",
    )?;
    equal(
        live.local_count,
        2,
        "prepared owned-live buffer local count",
    )?;
    let [
        NormalizedInstruction::Call {
            function: constructor,
            arguments: 0,
            ..
        },
        NormalizedInstruction::StoreLocal(1),
        NormalizedInstruction::LoadLocal { local: 0, .. } | NormalizedInstruction::MoveLocal(0),
        NormalizedInstruction::TailInvoke { arguments: 0 },
        NormalizedInstruction::Unit,
        NormalizedInstruction::StoreLocal(1),
        NormalizedInstruction::Return,
    ] = live.instructions.as_ref()
    else {
        return Err(
            "owned-live tail hint lost its live buffer or exact cleanup/Return continuation"
                .to_owned(),
        );
    };
    equal(
        Some(*constructor),
        program.function(empty),
        "prepared owned-live initializes the exact ByteBuffer constructor",
    )?;
    ensure(
        normalized_code(program, source, "owned-clean")?
            .instructions
            .iter()
            .any(|i| matches!(i, NormalizedInstruction::TailInvoke { arguments: 0 })),
        "already-consumed owner should permit terminal Invoke",
    )?;
    ensure(
        normalized_code(program, source, "non-tail")?
            .instructions
            .iter()
            .any(|i| matches!(i, NormalizedInstruction::Invoke { arguments: 1 }))
            && !normalized_code(program, source, "non-tail")?
                .instructions
                .iter()
                .any(|i| matches!(i, NormalizedInstruction::TailInvoke { .. })),
        "stack sensitivity control became tail-recursive",
    )?;
    Ok(())
}

fn tail_geometry(source: &KernelSnapshot, program: &NormalizedProgram) -> Check<()> {
    let dispatch = normalized_code(program, source, "dispatch")?;
    let a = normalized_code(program, source, "tail-a")?;
    let b = normalized_code(program, source, "tail-b")?;
    equal(dispatch.parameter_count, 3, "dispatcher parameter geometry")?;
    equal(dispatch.local_count, 3, "dispatcher actual local geometry")?;
    equal(a.parameter_count, 8, "unbound indirect target arity")?;
    equal(b.parameter_count, 14, "six-prefix indirect target arity")?;
    ensure(
        b.local_count >= 16,
        "large target lacks two additional live locals",
    )?;
    ensure(
        dispatch
            .instructions
            .iter()
            .any(|i| matches!(i, NormalizedInstruction::TailInvoke { arguments: 8 })),
        "dispatcher must retain its eight-argument TailInvoke",
    )?;
    for code in [a, b] {
        ensure(
            code.instructions
                .iter()
                .any(|i| matches!(i, NormalizedInstruction::TailInvoke { arguments: 3 })),
            "mutual edge must stay indirect and terminal",
        )?;
    }
    let explicit = 8_i64;
    let source_low = -8 * (i64::from(dispatch.local_count) + explicit + 1);
    let width = 8 * explicit;
    let above = -8 * explicit;
    let below = -8 * (6 + explicit);
    ensure(
        above > source_low && above < source_low + width,
        "zero-prefix destination does not overlap above the source",
    )?;
    ensure(
        below < source_low && below + width > source_low,
        "six-prefix destination does not overlap below the source",
    )?;
    eprintln!(
        "raw source callable geometry canonical_dispatch_locals={} explicit={} prefix_counts=0,6 \
         source_low={} above_low={} below_low={} bytes={} target_locals={},{} \
         observation=canonical-layout-not-rsp-high-water",
        dispatch.local_count,
        explicit,
        source_low,
        above,
        below,
        width,
        a.local_count,
        b.local_count,
    );
    Ok(())
}

fn evaluate_cases(
    label: &str,
    case: &InputCase,
    host: &HostOracle,
    program: &NormalizedProgram,
    cases: &[Behavior],
) -> Check<()> {
    let entry = function_named(&case.source, MODULE, "binary")?;
    let type_count = program.types.len();
    for &test in cases {
        let input = test.input();
        let expected = test.expected.to_le_bytes();
        let control = ExecutionControl::uncancelled();
        let mut policy = NormalizedRunPolicy::foreground();
        if let Some(depth) = test.call_depth {
            policy.maximum_call_depth = depth;
        }
        let arguments = vec![NormalizedValue::bytes(input.as_slice())];
        let vm = NormalizedVm::for_test(program, policy)
            .invoke(entry, arguments.clone(), None, &control)
            .map_err(|error| format!("{label}/{}({}): VM: {error:?}", test.name, test.count))?;
        let reference = NormalizedReferenceInterpreter::new(&case.source, &host.program, policy)
            .invoke(entry, arguments, None, &control)
            .map_err(|error| {
                format!(
                    "{label}/{}({}): reference: {error:?}",
                    test.name, test.count
                )
            })?;
        for (engine, value, frames) in [
            ("vm", vm.0, vm.1.live_call_frames_after),
            ("reference", reference.0, reference.1.live_call_frames_after),
        ] {
            equal(
                filter_value(&value)?,
                (0, expected.as_slice()),
                &format!("{label}/{engine}/{}({})", test.name, test.count),
            )?;
            equal(frames, 0, "callable evaluator frames reclaimed")?;
        }
        if test.call_depth.is_some() && test.count != 0 {
            ensure(
                vm.1.tail_transfers >= u64::try_from(test.count).map_err(|_| "tail count")?,
                "VM varying-frame test did not execute the indirect chain",
            )?;
            ensure(
                reference.1.tail_transfers
                    >= u64::try_from(test.count).map_err(|_| "tail count")?,
                "reference varying-frame test did not execute the indirect chain",
            )?;
        }
        equal(
            program.types.len(),
            type_count,
            "runtime depth changed the shallow descriptor table",
        )?;
        eprintln!(
            "raw source callable behavior case={}/{} count={} expected={} supplied_types={} \
             prepared_types={} vm_calls={} vm_tail_transfers={} reference_tail_transfers={}",
            label,
            test.name,
            test.count,
            test.expected,
            case.source.types.len(),
            type_count,
            vm.1.calls,
            vm.1.tail_transfers,
            reference.1.tail_transfers,
        );
    }
    Ok(())
}

fn execute_cases(label: &str, image: &[u8], cases: &[Behavior]) -> Check<()> {
    for &test in cases {
        let actual = run_filter(
            &format!("{label}/{}({})", test.name, test.count),
            image,
            &test.input(),
            0,
            FOCUSED_DEADLINE,
        )?;
        equal_bytes(
            &actual,
            &test.expected.to_le_bytes(),
            "independent callable native expectation",
        )?;
    }
    Ok(())
}

struct JoinedStackChild(Child);

impl Drop for JoinedStackChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn with_small_stack(label: &str, image: &[u8], input: &[u8]) -> Check<NativeOutput> {
    crate::release_container::inspect_static_elf_bytes(image)
        .map_err(|error| format!("{label}: static ELF: {error:?}"))?;
    let directory = tempfile::tempdir().map_err(|error| format!("{label}: tempdir: {error}"))?;
    let binary = directory.path().join("program");
    let input_path = directory.path().join("input");
    let stdout_path = directory.path().join("stdout");
    let stderr_path = directory.path().join("stderr");
    crate::platform::owned_output::publish_executable_create_new(
        &binary,
        image,
        None,
        "callable constrained-stack acceptance",
    )
    .map_err(|error| format!("{label}: executable: {error:?}"))?;
    fs::write(&input_path, input).map_err(|error| format!("{label}: input: {error}"))?;
    let stdin = File::open(&input_path).map_err(|error| error.to_string())?;
    let stdout = File::create(&stdout_path).map_err(|error| error.to_string())?;
    let stderr = File::create(&stderr_path).map_err(|error| error.to_string())?;
    let started = Instant::now();
    // The literal shell program sets a caller-selected test stack policy.
    // No language or compiler admission limit is changed.
    let mut child = JoinedStackChild(
        Command::new("/bin/bash")
            .arg("-c")
            .arg("ulimit -c 0 && ulimit -s 256 && exec \"$1\"")
            .arg("callable-stack-control")
            .arg(&binary)
            .env_clear()
            .current_dir(directory.path())
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| format!("{label}: spawn: {error}"))?,
    );
    let status = loop {
        if let Some(status) = child
            .0
            .try_wait()
            .map_err(|error| format!("{label}: wait: {error}"))?
        {
            break status;
        }
        if started.elapsed() >= FOCUSED_DEADLINE {
            return Err(format!("{label}: explicit caller deadline exceeded"));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let output = fs::read(&stdout_path).map_err(|error| error.to_string())?;
    let errors = fs::read(&stderr_path).map_err(|error| error.to_string())?;
    eprintln!(
        "raw source callable stack case={label} image_sha256={} input_sha256={} \
         output_sha256={} stack_policy_kib=256 core_policy_bytes=0 exit={:?} signal={:?} elapsed_ms={}",
        sha256(image),
        sha256(input),
        sha256(&output),
        status.code(),
        status.signal(),
        started.elapsed().as_millis(),
    );
    Ok(NativeOutput {
        stdout: output,
        stderr: errors,
        status,
    })
}

fn stack_controls(label: &str, image: &[u8]) -> Check<()> {
    for count in [257, 100_000] {
        let expected = 110 + 11 * (count / 2) + count % 2;
        let test = Behavior::new("tail-stack", 9, count, expected);
        let output = with_small_stack(&format!("{label}/tail({count})"), image, &test.input())?;
        equal(
            output.status.code(),
            Some(0),
            "indirect tail under explicit stack policy",
        )?;
        equal_bytes(
            &output.stdout,
            &expected.to_le_bytes(),
            "tail continuation and sentinels",
        )?;
        equal_bytes(&output.stderr, &[], "tail success stderr")?;
    }
    let small = Behavior::new("non-tail-small", 10, 32, 32);
    let output = with_small_stack(&format!("{label}/non-tail-small"), image, &small.input())?;
    equal(output.status.code(), Some(0), "non-tail small control")?;
    equal_bytes(
        &output.stdout,
        &32_i64.to_le_bytes(),
        "non-tail small expected result",
    )?;
    equal_bytes(&output.stderr, &[], "non-tail small stderr")?;
    let large = Behavior::new("non-tail-large", 10, 100_000, 100_000);
    let output = with_small_stack(&format!("{label}/non-tail-large"), image, &large.input())?;
    equal(
        output.status.signal(),
        Some(11),
        "non-tail stack-limit sensitivity must be SIGSEGV",
    )?;
    ensure(
        output.stdout.is_empty(),
        "non-tail stack fault published an answer",
    )?;
    Ok(())
}

#[derive(Clone, Copy)]
enum HostAdmission {
    Reject,
    RejectCode(&'static str),
    Accept,
    NotClaimed,
}

struct CallableRejection {
    name: &'static str,
    source: KernelSnapshot,
    diagnostic: i64,
    e1: Option<i64>,
    host: HostAdmission,
}

fn indirect_parts(
    source: &KernelSnapshot,
    expression: ExpressionId,
) -> Check<(ExpressionId, Vec<ExpressionId>)> {
    match operation(source, expression)? {
        E::Invoke { callee, arguments } | E::Bind { callee, arguments } => {
            Ok((*callee, arguments.clone()))
        }
        _ => Err("callable fixture indirect operation missing".to_owned()),
    }
}

fn rejections(base: &KernelSnapshot) -> Check<Vec<CallableRejection>> {
    let mut out = Vec::new();
    let mut changed = base.clone();
    let body = function_body(&changed, MODULE, "unused-invoke")?;
    let (callee, mut arguments) = indirect_parts(&changed, body)?;
    let removed = arguments.pop().ok_or("arity fixture operand absent")?;
    ensure(
        matches!(operation(&changed, removed)?, E::I64 { .. }),
        "arity removal must preserve containment by deleting a leaf",
    )?;
    ensure(
        changed
            .owners
            .remove(&OwnerKey::Expression(removed))
            .is_some(),
        "arity operand owner absent",
    )?;
    set_operation(&mut changed, body, E::Invoke { callee, arguments })?;
    out.push(CallableRejection {
        name: "uncalled-wrong-invoke-arity",
        source: changed,
        diagnostic: 19,
        e1: Some(0),
        host: HostAdmission::Reject,
    });

    let mut changed = base.clone();
    let body = function_body(&changed, MODULE, "unused-bind")?;
    let (_, arguments) = indirect_parts(&changed, body)?;
    let capture = *arguments.first().ok_or("capture fixture missing")?;
    set_operation(&mut changed, capture, E::Bool { value: true })?;
    out.push(CallableRejection {
        name: "uncalled-wrong-capture-type",
        source: changed,
        diagnostic: 19,
        e1: Some(0),
        host: HostAdmission::Reject,
    });

    let mut changed = base.clone();
    let body = function_body(&changed, MODULE, "other-entry")?;
    let (_, arguments) = indirect_parts(&changed, body)?;
    set_operation(
        &mut changed,
        *arguments.first().ok_or("unselected operand")?,
        E::Bool { value: true },
    )?;
    out.push(CallableRejection {
        name: "unselected-target-wrong-invoke-type",
        source: changed,
        diagnostic: 19,
        e1: Some(0),
        host: HostAdmission::Reject,
    });

    let mut changed = base.clone();
    let body = function_body(&changed, MODULE, "untaken")?;
    let branch = match operation(&changed, body)? {
        E::If { when_false, .. } => *when_false,
        _ => return Err("untaken callable branch missing".to_owned()),
    };
    let (_, arguments) = indirect_parts(&changed, branch)?;
    set_operation(
        &mut changed,
        *arguments.first().ok_or("untaken operand")?,
        E::Bool { value: true },
    )?;
    out.push(CallableRejection {
        name: "untaken-wrong-invoke-type",
        source: changed,
        diagnostic: 19,
        e1: Some(0),
        host: HostAdmission::Reject,
    });

    let mut changed = base.clone();
    let body = function_body(&changed, MODULE, "unused-invoke")?;
    let (callee, _) = indirect_parts(&changed, body)?;
    set_operation(&mut changed, callee, E::I64 { value: 42 })?;
    out.push(CallableRejection {
        name: "uncalled-noncallable-callee",
        source: changed,
        diagnostic: 19,
        e1: Some(0),
        host: HostAdmission::Reject,
    });

    let mut changed = base.clone();
    let body = function_body(&changed, MODULE, "unused-bind")?;
    let (callee, _) = indirect_parts(&changed, body)?;
    let zero = function_named(&changed, MODULE, "base")?;
    let mut replacement = operation(&changed, callee)?.clone();
    match &mut replacement {
        E::FunctionValue { function, .. } => *function = zero,
        _ => return Err("excess prefix FunctionValue absent".to_owned()),
    }
    set_operation(&mut changed, callee, replacement)?;
    out.push(CallableRejection {
        name: "uncalled-excess-capture-prefix",
        source: changed,
        diagnostic: 19,
        e1: Some(0),
        host: HostAdmission::Reject,
    });

    let mut changed = base.clone();
    let body = function_body(&changed, MODULE, "unused-bind")?;
    let (_, arguments) = indirect_parts(&changed, body)?;
    let buffer_body = function_body(&changed, MODULE, "buffer-leaf")?;
    let buffer = operation(&changed, buffer_body)?.clone();
    ensure(
        matches!(&buffer, E::Call { arguments, .. } if arguments.is_empty()),
        "affine capture mutation must not add dangling children",
    )?;
    set_operation(
        &mut changed,
        *arguments.first().ok_or("affine capture absent")?,
        buffer,
    )?;
    out.push(CallableRejection {
        name: "actual-affine-capture",
        source: changed,
        diagnostic: 19,
        e1: Some(0),
        host: HostAdmission::Reject,
    });

    let mut changed = base.clone();
    let body = function_body(&changed, MODULE, "unused-static")?;
    let (_, arguments) = indirect_parts(&changed, body)?;
    let argument = *arguments.first().ok_or("StaticText operand absent")?;
    let value = match operation(&changed, argument)? {
        E::StaticText { value } => value.clone(),
        _ => return Err("exact StaticText literal absent".to_owned()),
    };
    set_operation(&mut changed, argument, E::Text { value })?;
    out.push(CallableRejection {
        name: "text-is-not-static-text-invoke",
        source: changed,
        diagnostic: 19,
        e1: Some(0),
        host: HostAdmission::Reject,
    });

    let mut changed = base.clone();
    let body = function_body(&changed, MODULE, "shape-text-identity")?;
    let other = function_named(&changed, MODULE, "shape-static-number")?;
    let mut replacement = operation(&changed, body)?.clone();
    match &mut replacement {
        E::FunctionValue { function, .. } => *function = other,
        _ => return Err("projection FunctionValue control absent".to_owned()),
    }
    set_operation(&mut changed, body, replacement)?;
    out.push(CallableRejection {
        name: "equal-runtime-function-shape-does-not-erase-semantic-identity",
        source: changed,
        diagnostic: 19,
        e1: Some(0),
        host: HostAdmission::Reject,
    });

    let mut changed = base.clone();
    let body = function_body(&changed, MODULE, "unused-shape-holder")?;
    let other = function_named(&changed, MODULE, "shape-static-holder")?;
    let mut replacement = operation(&changed, body)?.clone();
    match &mut replacement {
        E::Call { function, .. } => *function = other,
        _ => return Err("projection nested-record control absent".to_owned()),
    }
    set_operation(&mut changed, body, replacement)?;
    out.push(CallableRejection {
        name: "equal-runtime-nested-record-shape-does-not-erase-semantic-identity",
        source: changed,
        diagnostic: 19,
        e1: Some(0),
        host: HostAdmission::Reject,
    });

    let mut changed = base.clone();
    let body = function_body(&changed, MODULE, "unused-bind")?;
    let (callee, _) = indirect_parts(&changed, body)?;
    let direct_only = function_named(&changed, MODULE, "direct-only")?;
    let mut replacement = operation(&changed, callee)?.clone();
    match &mut replacement {
        E::FunctionValue { function, .. } => *function = direct_only,
        _ => return Err("direct-only FunctionValue absent".to_owned()),
    }
    set_operation(&mut changed, callee, replacement)?;
    out.push(CallableRejection {
        name: "direct-only-signature-as-value",
        source: changed,
        diagnostic: 13,
        e1: Some(0),
        host: HostAdmission::NotClaimed,
    });

    let mut changed = base.clone();
    let task = function_named(&changed, MODULE, "task-seed")?;
    match changed
        .owners
        .get_mut(&OwnerKey::Declaration(task.declaration))
    {
        Some(OwnerRecord::Declaration(record)) => match &mut record.payload {
            kernel::DeclarationPayload::Function(function) => {
                function.effect = kernel::FunctionEffect::Task {
                    requirements: vec![],
                    effect_parameters: vec![],
                };
                // The payload and canonical owner header are one kind contract.
                record.header.kind = kernel::OwnerKind::TaskFunction;
            }
            _ => return Err("task seed is not a function".to_owned()),
        },
        _ => return Err("task seed owner absent".to_owned()),
    }
    out.push(CallableRejection {
        name: "uncalled-task-outside-source-profile",
        source: changed,
        diagnostic: 13,
        e1: None,
        host: HostAdmission::Accept,
    });

    let mut changed = base.clone();
    let body = function_body(&changed, MODULE, "unused-generic")?;
    let boolean = changed
        .types
        .iter()
        .find_map(|(id, ty)| matches!(ty.form, kernel::TypeForm::Bool).then_some(*id))
        .ok_or("fixture Bool missing")?;
    let mut replacement = operation(&changed, body)?.clone();
    match &mut replacement {
        E::FunctionValue { type_arguments, .. } => {
            *type_arguments
                .first_mut()
                .ok_or("closed list type argument missing")? = boolean;
        }
        _ => return Err("closed generic FunctionValue missing".to_owned()),
    }
    set_operation(&mut changed, body, replacement)?;
    out.push(CallableRejection {
        name: "generalized-generic-outside-i64-closure",
        source: changed,
        diagnostic: 13,
        e1: Some(0),
        host: HostAdmission::NotClaimed,
    });

    let mut changed = base.clone();
    let before = kernel::semantic_state_digest(base)
        .map_err(|error| format!("baseline semantic state: {error:?}"))?;
    let unused = kernel::TypeObject::new(kernel::TypeForm::F64)
        .map_err(|error| format!("unused F64 type: {error:?}"))?;
    let identity = kernel::encode_type_object(&unused)
        .map_err(|error| format!("unused type encoding: {error:?}"))?
        .0;
    ensure(
        changed.types.insert(identity, unused).is_none(),
        "unused F64 was already supplied",
    )?;
    equal(
        kernel::semantic_state_digest(&changed)
            .map_err(|error| format!("unused type semantic state: {error:?}"))?,
        before,
        "unused supplied type is outside semantic-state commitment",
    )?;
    out.push(CallableRejection {
        name: "unused-supplied-unsupported-type",
        source: changed,
        diagnostic: 13,
        e1: Some(13),
        // The carrier must inspect unused supplied descriptors. A canonical
        // repository snapshot separately requires every type to be reachable.
        host: HostAdmission::RejectCode("kernel_full_type_unreachable"),
    });
    Ok(out)
}

fn verify_rejections(left_name: &str, left: &[u8], right_name: &str, right: &[u8]) -> Check<()> {
    let base = fixture()?;
    kernel::validate_full(base)
        .map_err(|error| format!("callable negative baseline must be host-valid: {error:?}"))?;
    let selected = target_named(base, TARGET)?;
    let mut names = BTreeSet::new();
    for test in rejections(base)? {
        ensure(names.insert(test.name), "duplicate callable negative label")?;
        let carrier = graph::packet(&test.source);
        // The synthetic mutation writer recomputes every owner/type digest and
        // the source-state/provenance framing. No stale-hash early rejection
        // can stand in for the requested E2 diagnostic.
        if let Some(e1) = test.e1 {
            admission_layer_result("read-nominal", &carrier, e1)?;
        }
        for mode in [SourceOutputMode::Elf, SourceOutputMode::CanonicalArtifact] {
            let request = raw_source::request(&carrier, selected, mode)
                .map_err(|error| format!("{} request: {error:?}", test.name))?;
            for (producer, image) in [(left_name, left), (right_name, right)] {
                let output = run_filter(
                    &format!("{producer}/callable-negative/{}/{mode:?}", test.name),
                    image,
                    &request,
                    test.diagnostic,
                    FOCUSED_DEADLINE,
                )?;
                ensure(output.is_empty(), "rejected callable source published code")?;
            }
        }
        match test.host {
            HostAdmission::Reject => ensure(
                kernel::validate_full(&test.source).is_err(),
                "independent kernel accepted actual callable type/custody error",
            )?,
            HostAdmission::RejectCode(code) => {
                let diagnostics = kernel::validate_full(&test.source)
                    .err()
                    .ok_or_else(|| format!("{} must be rejected by the host", test.name))?;
                equal(
                    diagnostics.len(),
                    1,
                    "exact independent host rejection count",
                )?;
                ensure(
                    diagnostics.iter().all(|diagnostic| diagnostic.code == code),
                    &format!(
                        "{} host rejection must be {code}: {diagnostics:?}",
                        test.name
                    ),
                )?;
            }
            HostAdmission::Accept => {
                kernel::validate_full(&test.source)
                    .map_err(|error| format!("{} must be host-valid: {error:?}", test.name))?;
            }
            HostAdmission::NotClaimed => {}
        }
        eprintln!(
            "raw source callable negative case={} diagnostic={} source_carrier_sha256={} \
             supplied_owners={} supplied_types={} selected_target={} modes=2 generations=2",
            test.name,
            test.diagnostic,
            sha256(&carrier),
            test.source.owners.len(),
            test.source.types.len(),
            selected,
        );
    }
    equal(
        names.len(),
        14,
        "nonempty complete callable negative corpus",
    )
}

fn raw_elf(label: &str, compiler: &[u8], case: &InputCase, target: &str) -> Check<Vec<u8>> {
    let request = case.request(target, SourceOutputMode::Elf)?;
    let image = run_filter(label, compiler, &request, 0, FOCUSED_DEADLINE)?;
    crate::release_container::inspect_static_elf_bytes(&image)
        .map_err(|error| format!("{label}: static ELF: {error:?}"))?;
    Ok(image)
}

fn identity_evaluators(
    case: &InputCase,
    host: &HostOracle,
    program: &NormalizedProgram,
) -> Check<()> {
    for function in ["identity", "identity-empty-bind"] {
        let entry = function_named(&case.source, MODULE, function)?;
        for input in [vec![], vec![0, 255, 17, 128, 3]] {
            let arguments = vec![NormalizedValue::bytes(input.as_slice())];
            let control = ExecutionControl::uncancelled();
            let policy = NormalizedRunPolicy::foreground();
            let vm = NormalizedVm::for_test(program, policy)
                .invoke(entry, arguments.clone(), None, &control)
                .map_err(|error| format!("{function}: VM: {error:?}"))?;
            let reference =
                NormalizedReferenceInterpreter::new(&case.source, &host.program, policy)
                    .invoke(entry, arguments, None, &control)
                    .map_err(|error| format!("{function}: reference: {error:?}"))?;
            for (value, frames) in [
                (vm.0, vm.1.live_call_frames_after),
                (reference.0, reference.1.live_call_frames_after),
            ] {
                equal(
                    filter_value(&value)?,
                    (0, input.as_slice()),
                    "identity caller expectation",
                )?;
                equal(frames, 0, "identity callable frames reclaimed")?;
            }
        }
    }
    Ok(())
}

fn verify_behavior(left_name: &str, left: &[u8], right_name: &str, right: &[u8]) -> Check<()> {
    let case = case_from_snapshot(fixture()?)?;
    // All source compilation requests finish before host_oracle. The additional
    // identity ELF selections use the same complete S; their canonical units are
    // included in both already-requested whole-package artifacts.
    let first = raw_outputs(
        &format!("{left_name}/callable"),
        left,
        &case,
        TARGET,
        FOCUSED_DEADLINE,
    )?;
    let second = raw_outputs(
        &format!("{right_name}/callable"),
        right,
        &case,
        TARGET,
        FOCUSED_DEADLINE,
    )?;
    let identities = [
        (
            raw_elf(
                &format!("{left_name}/identity"),
                left,
                &case,
                "callable-identity",
            )?,
            raw_elf(
                &format!("{left_name}/empty-bind-identity"),
                left,
                &case,
                "callable-empty-bind-identity",
            )?,
        ),
        (
            raw_elf(
                &format!("{right_name}/identity"),
                right,
                &case,
                "callable-identity",
            )?,
            raw_elf(
                &format!("{right_name}/empty-bind-identity"),
                right,
                &case,
                "callable-empty-bind-identity",
            )?,
        ),
    ];
    equal_bytes(&first.elf, &second.elf, "callable stage ELF identity")?;
    equal_bytes(
        &first.artifact,
        &second.artifact,
        "callable stage artifact identity",
    )?;
    equal_bytes(
        &identities[0].0,
        &identities[1].0,
        "identity stage ELF equality",
    )?;
    equal_bytes(
        &identities[0].1,
        &identities[1].1,
        "empty-bind identity stage ELF equality",
    )?;
    let host = host_oracle(&case)?;
    let loaded = strict_artifact(left_name, &first.artifact, &case, &host)?;
    strict_artifact(right_name, &second.artifact, &case, &host)?;
    let program = NormalizedProgram::prepare(loaded.clone())
        .map_err(|error| format!("callable artifact prepare: {error:?}"))?;
    canonical_markers(&case.source, &loaded, &program)?;
    tail_geometry(&case.source, &program)?;
    let cases = behaviors();
    evaluate_cases("callable", &case, &host, &program, &cases)?;
    identity_evaluators(&case, &host, &program)?;
    let entry = function_named(&case.source, MODULE, "binary")?;
    let native = native_image(&host.program, entry)?;
    for (producer, image) in [
        ("C0", native.as_slice()),
        (left_name, first.elf.as_slice()),
        (right_name, second.elf.as_slice()),
    ] {
        execute_cases(producer, image, &cases)?;
        stack_controls(producer, image)?;
    }
    let native_identity = native_image(
        &host.program,
        function_named(&case.source, MODULE, "identity")?,
    )?;
    let native_empty = native_image(
        &host.program,
        function_named(&case.source, MODULE, "identity-empty-bind")?,
    )?;
    for (producer, baseline, empty) in [
        ("C0", native_identity.as_slice(), native_empty.as_slice()),
        (
            left_name,
            identities[0].0.as_slice(),
            identities[0].1.as_slice(),
        ),
        (
            right_name,
            identities[1].0.as_slice(),
            identities[1].1.as_slice(),
        ),
    ] {
        for input in [Vec::new(), (0..131_073).map(|i| (i % 251) as u8).collect()] {
            super::phase::verify_empty_bind_identity_pair(baseline, empty, &input, &input)?;
            eprintln!(
                "raw source empty-bind phase pair producer={producer} input_bytes={} \
                 baseline_sha256={} empty_bind_sha256={} named_and_bound_cases=2",
                input.len(),
                sha256(baseline),
                sha256(empty),
            );
        }
    }
    eprintln!(
        "raw source callable stages={left_name},{right_name} cases={} output_modes=2 \
         independent_engines=vm,canonical-reference,c0-native,source-native \
         runtime_wrapper_counts=255,256,257,2048 record_wrappers=true \
         source_descriptor_growth=none canonical_bind_markers=exact",
        cases.len(),
    );
    Ok(())
}

fn verify_mutation(left_name: &str, left: &[u8], right_name: &str, right: &[u8]) -> Check<()> {
    let mut source = fixture()?.clone();
    let body = function_body(&source, MODULE, "base")?;
    ensure(
        matches!(operation(&source, body)?, E::I64 { value: 7 }),
        "behavioral mutation baseline changed",
    )?;
    set_operation(&mut source, body, E::I64 { value: 9 })?;
    let case = case_from_snapshot(&source)?;
    let first = raw_outputs(
        &format!("{left_name}/callable-mutation"),
        left,
        &case,
        TARGET,
        FOCUSED_DEADLINE,
    )?;
    let second = raw_outputs(
        &format!("{right_name}/callable-mutation"),
        right,
        &case,
        TARGET,
        FOCUSED_DEADLINE,
    )?;
    equal_bytes(
        &first.elf,
        &second.elf,
        "mutated callable ELF stage identity",
    )?;
    equal_bytes(
        &first.artifact,
        &second.artifact,
        "mutated callable artifact stage identity",
    )?;
    let host = host_oracle(&case)?;
    let loaded = strict_artifact("callable-mutation-left", &first.artifact, &case, &host)?;
    strict_artifact("callable-mutation-right", &second.artifact, &case, &host)?;
    let program = NormalizedProgram::prepare(loaded)
        .map_err(|error| format!("mutated callable artifact: {error:?}"))?;
    let cases = [
        Behavior::new("mutated-named-callable", 2, 0, 9),
        Behavior::new("mutated-record-chain", 8, 257, 2 * (9 + 257)),
    ];
    evaluate_cases("source-mutation-7-to-9", &case, &host, &program, &cases)?;
    let native = native_image(
        &host.program,
        function_named(&case.source, MODULE, "binary")?,
    )?;
    for (producer, image) in [
        ("C0", native.as_slice()),
        (left_name, first.elf.as_slice()),
        (right_name, second.elf.as_slice()),
    ] {
        execute_cases(&format!("{producer}/source-mutation-7-to-9"), image, &cases)?;
    }
    Ok(())
}

fn wide_source(count: usize) -> Check<(String, i64)> {
    ensure(count != 0, "wide fixture requires a nonempty prefix")?;
    let estimate = count
        .checked_mul(192)
        .and_then(|n| n.checked_add(4096))
        .ok_or("wide source representation overflow")?;
    let mut source = String::new();
    source
        .try_reserve(estimate)
        .map_err(|error| format!("wide source reservation: {error}"))?;
    source.push_str("declarations.begin\n(units (module create callable-wide\n");
    source.push_str("(external create add (visibility private) (implementation core.i64.add) (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))\n");
    source.push_str("(external create mul (visibility private) (implementation core.i64.multiply) (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))\n");
    source.push_str("(function create select (visibility private) (effect pure) ");
    for index in 0..count {
        write!(source, "(parameter create p{index} (type I64)) ")
            .map_err(|error| format!("wide parameter formatting: {error}"))?;
    }
    // A handful of noncommuting weighted probes, including both sides of the
    // old 128-slot edge and the last slot, expose prefix loss or reversal.
    let positions = BTreeSet::from([
        0,
        127.min(count - 1),
        128.min(count - 1),
        count / 2,
        count - 1,
    ]);
    let weights = [2_i64, 3, 5, 7, 11];
    let mut expression = String::new();
    let mut expected = 0_i64;
    for (position, weight) in positions.into_iter().zip(weights) {
        let value = i64::try_from(position)
            .map_err(|_| "wide value representation")?
            .checked_add(1)
            .ok_or("wide value addition")?;
        expected = expected
            .checked_add(value.checked_mul(weight).ok_or("wide checksum product")?)
            .ok_or("wide checksum sum")?;
        let term = format!("(call mul (local p{position}) (i64 {weight}))");
        expression = if expression.is_empty() {
            term
        } else {
            format!("(call add {expression} {term})")
        };
    }
    writeln!(source, "(returns I64) (body {expression}))")
        .map_err(|error| format!("wide result formatting: {error}"))?;
    source.push_str("(function create entry (visibility public) (effect pure) (returns I64) (body (let (binding f0 (function-value select)) ");
    let mut start = 0usize;
    let mut chunk = 0usize;
    while start < count {
        let end = start
            .checked_add((count - start).min(64))
            .ok_or("wide chunk extent")?;
        let next = chunk.checked_add(1).ok_or("wide chunk index")?;
        write!(source, "(binding f{next} (bind (local f{chunk}) ")
            .map_err(|error| format!("wide binding formatting: {error}"))?;
        for index in start..end {
            let value = i64::try_from(index)
                .map_err(|_| "wide literal representation")?
                .checked_add(1)
                .ok_or("wide literal addition")?;
            write!(source, "(i64 {value}) ")
                .map_err(|error| format!("wide literal formatting: {error}"))?;
        }
        source.push_str(")) ");
        start = end;
        chunk = next;
    }
    writeln!(source, "(in (invoke (local f{chunk}))))))")
        .map_err(|error| format!("wide invocation formatting: {error}"))?;
    source.push_str("(component create app (visibility public) (port create run (type (function () I64)) (function entry))))\n");
    source.push_str("(target create callable-wide (component callable-wide::app) (runner command) (port callable-wide::app::run)))\ndeclarations.end\n");
    Ok((source, expected))
}

fn unit_extents(host: &HostOracle) -> (usize, usize) {
    let mut count = 0usize;
    let mut maximum = 0usize;
    for (key, bytes) in &host.loaded.objects {
        if key.domain == ObjectDomain::CompilerUnit {
            count += 1;
            maximum = maximum.max(bytes.len());
        }
    }
    (count, maximum)
}

fn wide_markers(
    program: &NormalizedProgram,
    entry: DeclarationReference,
    count: usize,
) -> Check<()> {
    let index = usize::try_from(program.function(entry).ok_or("wide entry absent")?.0)
        .map_err(|_| "wide entry index")?;
    let code = match &program
        .functions
        .get(index)
        .ok_or("wide function absent")?
        .body
    {
        NormalizedFunctionBody::Code(code) => code,
        _ => return Err("wide source function became external".to_owned()),
    };
    let mut active = None::<(u32, u32)>;
    let mut captures = 0usize;
    let mut chunks = 0usize;
    for instruction in code.instructions.iter() {
        match *instruction {
            NormalizedInstruction::BeginBind { arguments } => {
                ensure(active.is_none(), "wide BeginBind nested unexpectedly")?;
                ensure(
                    arguments > 0 && arguments <= 64,
                    "wide chunk generation changed",
                )?;
                active = Some((arguments, 0));
            }
            NormalizedInstruction::Capture { index } => {
                let (arguments, next) = active.as_mut().ok_or("wide Capture outside BeginBind")?;
                equal(index, *next, "wide Capture ordinal")?;
                ensure(*next < *arguments, "wide Capture exceeded chunk")?;
                *next += 1;
                captures += 1;
            }
            NormalizedInstruction::Bind { arguments } => {
                let (declared, observed) = active.take().ok_or("wide Bind outside BeginBind")?;
                equal(arguments, declared, "wide Bind declared arity")?;
                equal(observed, declared, "wide Bind complete captures")?;
                chunks += 1;
            }
            _ => {}
        }
    }
    ensure(active.is_none(), "wide canonical binding left unfinished")?;
    equal(captures, count, "wide canonical captures retained")?;
    equal(chunks, count.div_ceil(64), "wide prefix chunk count")?;
    ensure(
        code.instructions
            .iter()
            .any(|i| matches!(i, NormalizedInstruction::TailInvoke { arguments: 0 })),
        "wide final fully-bound invocation must remain terminal",
    )?;
    Ok(())
}

fn verify_wide(
    left_name: &str,
    left: &[u8],
    right_name: &str,
    right: &[u8],
    count: usize,
) -> Check<()> {
    let (text, expected) = wide_source(count)?;
    let source = byte_buffer_tests::author_only(&text)?;
    let case = case_from_snapshot(&source)?;
    let first = raw_outputs(
        &format!("{left_name}/callable-wide-{count}"),
        left,
        &case,
        "callable-wide",
        FOCUSED_DEADLINE,
    )?;
    let second = raw_outputs(
        &format!("{right_name}/callable-wide-{count}"),
        right,
        &case,
        "callable-wide",
        FOCUSED_DEADLINE,
    )?;
    equal_bytes(&first.elf, &second.elf, "wide callable stage ELF identity")?;
    equal_bytes(
        &first.artifact,
        &second.artifact,
        "wide callable stage artifact identity",
    )?;
    let host = host_oracle(&case)?;
    let loaded = strict_artifact("callable-wide-left", &first.artifact, &case, &host)?;
    strict_artifact("callable-wide-right", &second.artifact, &case, &host)?;
    let program = NormalizedProgram::prepare(loaded)
        .map_err(|error| format!("wide callable preparation: {error:?}"))?;
    let entry = function_named(&case.source, "callable-wide", "entry")?;
    wide_markers(&program, entry, count)?;
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::foreground();
    let vm = NormalizedVm::for_test(&program, policy)
        .invoke(entry, vec![], None, &control)
        .map_err(|error| format!("wide VM: {error:?}"))?;
    let reference = NormalizedReferenceInterpreter::new(&case.source, &host.program, policy)
        .invoke(entry, vec![], None, &control)
        .map_err(|error| format!("wide reference: {error:?}"))?;
    for (value, frames) in [
        (vm.0, vm.1.live_call_frames_after),
        (reference.0, reference.1.live_call_frames_after),
    ] {
        match value {
            NormalizedValue::I64(value) => equal(value, expected, "wide independent checksum")?,
            _ => return Err("wide callable result is not I64".to_owned()),
        }
        equal(frames, 0, "wide callable frames reclaimed")?;
    }
    let native = native_image(&host.program, entry)?;
    for (name, image) in [
        ("C0", native.as_slice()),
        (left_name, first.elf.as_slice()),
        (right_name, second.elf.as_slice()),
    ] {
        run_scalar(
            &format!("{name}/callable-wide-{count}/execute"),
            image,
            expected,
            FOCUSED_DEADLINE,
        )?;
    }
    let prefix_bytes = count.checked_mul(8).ok_or("native prefix byte extent")?;
    let (units, maximum_unit) = unit_extents(&host);
    eprintln!(
        "raw source callable wide count={count} prefix_payload_bytes={prefix_bytes} \
         max_bind_chunk=64 chunks={} expected={expected} source_bytes={} source_carrier_bytes={} \
         source_carrier_sha256={} artifact_bytes={} compiler_units={units} maximum_unit_bytes={} \
         storage_unit_limit={:?} codec_unit_limit={} output_modes=2 independent_engines=4",
        count.div_ceil(64),
        text.len(),
        case.carrier.len(),
        sha256(&case.carrier),
        host.bytes.len(),
        maximum_unit,
        ObjectDomain::CompilerUnit.maximum_bytes(),
        crate::platform::compiler::unit::MAXIMUM_COMPILER_UNIT_BYTES,
    );
    Ok(())
}

pub(super) fn verify_stages(
    left_name: &str,
    left: &[u8],
    right_name: &str,
    right: &[u8],
) -> Check<()> {
    verify_behavior(left_name, left, right_name, right)?;
    verify_mutation(left_name, left, right_name, right)?;
    verify_rejections(left_name, left, right_name, right)?;
    for count in [129, 257, 1025] {
        verify_wide(left_name, left, right_name, right, count)?;
    }
    Ok(())
}

fn projection_call(
    reference: bool,
    module: &str,
    name: &str,
    arguments: Vec<NormalizedValue>,
) -> Check<NormalizedValue> {
    let (source, program) = compiler();
    let entry = function_named(source, module, name)?;
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let (value, frames) = if reference {
        let result = NormalizedReferenceInterpreter::new(source, program, policy)
            .invoke(entry, arguments, None, &control)
            .map_err(|error| format!("projection reference {module}::{name}: {error:?}"))?;
        (result.0, result.1.live_call_frames_after)
    } else {
        let result = NormalizedVm::for_test(program, policy)
            .invoke(entry, arguments, None, &control)
            .map_err(|error| format!("projection VM {module}::{name}: {error:?}"))?;
        (result.0, result.1.live_call_frames_after)
    };
    equal(frames, 0, "projection evaluator frames reclaimed")?;
    Ok(value)
}

fn projection_field<'a>(value: &'a NormalizedValue, name: &str) -> Check<&'a NormalizedValue> {
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
        return Err("projection result is not a structural record".to_owned());
    };
    fields
        .iter()
        .find_map(|(key, value)| (key.as_str() == name).then_some(value))
        .ok_or_else(|| format!("projection field absent: {name}"))
}

fn projection_bytes<'a>(value: &'a NormalizedValue, name: &str) -> Check<&'a [u8]> {
    match projection_field(value, name)? {
        NormalizedValue::Bytes(bytes) => Ok(bytes.as_ref()),
        _ => Err(format!("projection field is not Bytes: {name}")),
    }
}

fn projection_success(value: &NormalizedValue) -> Check<()> {
    ensure(
        matches!(projection_field(value, "code")?, NormalizedValue::I64(0)),
        "source projection returned a diagnostic",
    )
}

fn projection_word(bytes: &[u8], at: usize) -> Check<u64> {
    let end = at.checked_add(8).ok_or("projection word extent overflow")?;
    let word: [u8; 8] = bytes
        .get(at..end)
        .ok_or("projection word missing")?
        .try_into()
        .map_err(|_| "projection word width")?;
    Ok(u64::from_le_bytes(word))
}

struct ProjectionType<'a> {
    identity: u64,
    kind: u64,
    payload: &'a [u8],
}

fn projection_types(bytes: &[u8]) -> Check<Vec<ProjectionType<'_>>> {
    let mut at = 0usize;
    let mut rows = Vec::new();
    while at < bytes.len() {
        let payload = at.checked_add(16).ok_or("semantic header overflow")?;
        let count = usize::try_from(projection_word(
            bytes,
            at.checked_add(8).ok_or("semantic length offset overflow")?,
        )?)
        .map_err(|_| "semantic length domain")?;
        let end = payload
            .checked_add(count)
            .ok_or("semantic payload overflow")?;
        rows.push(ProjectionType {
            identity: u64::try_from(at)
                .map_err(|_| "semantic identity domain")?
                .checked_add(1)
                .ok_or("semantic identity overflow")?,
            kind: projection_word(bytes, at)?,
            payload: bytes.get(payload..end).ok_or("semantic payload extent")?,
        });
        at = end;
    }
    Ok(rows)
}

fn projection_semantic_id(rows: &[ProjectionType<'_>], kind: u64, payload: &[u8]) -> Check<u64> {
    let mut matching = rows
        .iter()
        .filter(|row| row.kind == kind && row.payload == payload);
    let identity = matching
        .next()
        .ok_or("expected semantic descriptor absent")?
        .identity;
    ensure(
        matching.next().is_none(),
        "exact semantic descriptor was duplicated",
    )?;
    Ok(identity)
}

fn projection_words(words: &[u64]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn projection_record(child: u64, name: &[u8]) -> Check<Vec<u8>> {
    let mut row = projection_words(&[
        1,
        child,
        u64::try_from(name.len()).map_err(|_| "semantic field-name length")?,
    ]);
    row.extend_from_slice(name);
    Ok(row)
}

fn projection_descriptor(descriptors: &[u8], identity: u64) -> Check<&[u8]> {
    let at = usize::try_from(identity)
        .ok()
        .and_then(|identity| identity.checked_sub(1))
        .and_then(|identity| identity.checked_mul(32))
        .ok_or("runtime descriptor offset")?;
    let end = at
        .checked_add(32)
        .ok_or("runtime descriptor extent overflow")?;
    descriptors
        .get(at..end)
        .ok_or_else(|| "runtime descriptor extent".to_owned())
}

fn projection_record_shape(
    descriptors: &[u8],
    payload: &[u8],
    identity: u64,
    child: u64,
    name: &[u8],
) -> Check<()> {
    let descriptor = projection_descriptor(descriptors, identity)?;
    equal(projection_word(descriptor, 0)?, 8, "projected Record kind")?;
    equal(
        projection_word(descriptor, 8)?,
        1,
        "projected Record field count",
    )?;
    equal(
        projection_word(descriptor, 24)?,
        0,
        "projected Record reserved word",
    )?;
    let field =
        usize::try_from(projection_word(descriptor, 16)?).map_err(|_| "projected field offset")?;
    let field_end = field
        .checked_add(32)
        .ok_or("projected field extent overflow")?;
    let field = payload
        .get(field..field_end)
        .ok_or("projected field extent")?;
    equal(
        projection_word(field, 0)?,
        child,
        "projected Record child alias",
    )?;
    let start = usize::try_from(projection_word(field, 8)?).map_err(|_| "projected name offset")?;
    let count =
        usize::try_from(projection_word(field, 16)?).map_err(|_| "projected name length")?;
    let end = start
        .checked_add(count)
        .ok_or("projected name extent overflow")?;
    equal(
        projection_word(field, 24)?,
        0,
        "projected field reserved word",
    )?;
    equal_bytes(
        payload.get(start..end).ok_or("projected name extent")?,
        name,
        "projected field-name bytes",
    )
}

fn observe_projection(reference: bool) -> Check<[Vec<u8>; 4]> {
    let case = case_from_snapshot(fixture()?)?;
    // These are compiler-source VM/reference observations of private E2/E3 state.
    // No host-normalized target or inferred table is supplied to the first call.
    // C2/C3 ELF/artifact behavior uses the same full fixture in verify_stages.
    let admitted = projection_call(
        reference,
        "source-scalar",
        "prepare",
        vec![NormalizedValue::bytes(case.carrier.as_slice())],
    )?;
    projection_success(&admitted)?;
    let semantic_types = projection_bytes(&admitted, "types")?;
    let semantic_facts = projection_bytes(&admitted, "facts")?;
    let projected = projection_call(
        reference,
        "source-runtime-types",
        "prepare",
        vec![
            NormalizedValue::bytes(case.carrier.as_slice()),
            NormalizedValue::bytes(semantic_types),
            NormalizedValue::bytes(projection_bytes(&admitted, "mapping")?),
            NormalizedValue::bytes(semantic_facts),
        ],
    )?;
    projection_success(&projected)?;
    let types = projection_bytes(&projected, "types")?;
    ensure(
        types.starts_with(semantic_types),
        "private projection rewrote semantic types",
    )?;
    let rows = projection_types(types)?;
    let integer = projection_semantic_id(&rows, 1, &[])?;
    let text = projection_semantic_id(&rows, 4, &[])?;
    let static_text = projection_semantic_id(&rows, 8, &[])?;
    let text_fn = projection_semantic_id(&rows, 11, &projection_words(&[1, text, integer]))?;
    let static_fn =
        projection_semantic_id(&rows, 11, &projection_words(&[1, static_text, integer]))?;
    let text_inner = projection_semantic_id(&rows, 10, &projection_record(text_fn, b"chosen")?)?;
    let static_inner =
        projection_semantic_id(&rows, 10, &projection_record(static_fn, b"chosen")?)?;
    let text_outer = projection_semantic_id(&rows, 10, &projection_record(text_inner, b"nested")?)?;
    let static_outer =
        projection_semantic_id(&rows, 10, &projection_record(static_inner, b"nested")?)?;
    let different_name =
        projection_semantic_id(&rows, 10, &projection_record(text_fn, b"choice")?)?;
    let mapping = projection_bytes(&projected, "mapping")?;
    let representatives = projection_bytes(&projected, "records")?;
    equal(mapping.len() % 16, 0, "semantic/runtime mapping rows")?;
    equal(representatives.len() % 8, 0, "runtime representative rows")?;
    let mut aliases = std::collections::BTreeMap::new();
    let mut previous = 0;
    for row in mapping.as_chunks::<16>().0 {
        let semantic = projection_word(row, 0)?;
        ensure(
            previous < semantic,
            "projection mapping is not in semantic order",
        )?;
        aliases.insert(semantic, projection_word(row, 8)?);
        previous = semantic;
    }
    equal(
        aliases.len(),
        rows.len(),
        "every semantic descriptor has a runtime mapping",
    )?;
    let runtime = |semantic| {
        aliases
            .get(&semantic)
            .copied()
            .ok_or("semantic alias missing".to_owned())
    };
    let pairs = [
        (text_fn, static_fn),
        (text_inner, static_inner),
        (text_outer, static_outer),
    ];
    let mut selected = BTreeSet::new();
    for (left, right) in pairs {
        ensure(
            left != right,
            "semantic Text/StaticText compounds were conflated",
        )?;
        equal(
            runtime(left)?,
            runtime(right)?,
            "equal execution shape must reuse its runtime ID",
        )?;
        let representative = usize::try_from(
            runtime(left)?
                .checked_sub(8)
                .ok_or("compound runtime ID is primitive")?,
        )
        .map_err(|_| "representative index")?
        .checked_mul(8)
        .ok_or("representative byte offset")?;
        equal(
            projection_word(representatives, representative)?,
            left.min(right),
            "runtime ID must retain the first-seen semantic representative",
        )?;
        selected.insert(left);
        selected.insert(right);
    }
    ensure(
        runtime(different_name)? != runtime(text_inner)?,
        "equal-length different field names must not alias",
    )?;
    let projected_facts = projection_bytes(&projected, "facts")?;
    equal(
        projected_facts.len(),
        semantic_facts.len(),
        "owner fact inventory extent",
    )?;
    let mut seen = BTreeSet::new();
    for (semantic, projected) in semantic_facts
        .as_chunks::<8>()
        .0
        .iter()
        .zip(projected_facts.as_chunks::<8>().0)
    {
        let semantic = projection_word(semantic, 0)?;
        if selected.contains(&semantic) {
            equal(
                projection_word(projected, 0)?,
                runtime(semantic)?,
                "expression facts use runtime aliases",
            )?;
            seen.insert(semantic);
        }
    }
    equal(
        seen,
        selected,
        "every selected Function/Record shape reaches an expression fact",
    )?;
    let mut signature_uses = 0;
    for row in projection_bytes(&projected, "signatures")?
        .as_chunks::<8>()
        .0
    {
        signature_uses += usize::from(projection_word(row, 0)? == runtime(text_fn)?);
    }
    ensure(
        signature_uses >= 2,
        "both named Text/StaticText signatures must map to the shared ID",
    )?;
    let encoded = projection_call(
        reference,
        "source-runtime-types",
        "encode",
        vec![
            NormalizedValue::bytes(types),
            NormalizedValue::bytes(mapping),
            NormalizedValue::bytes(representatives),
            NormalizedValue::I64(0),
        ],
    )?;
    projection_success(&encoded)?;
    let descriptors = projection_bytes(&encoded, "descriptors")?;
    let payload = projection_bytes(&encoded, "payload")?;
    equal(descriptors.len() % 32, 0, "runtime descriptor row extent")?;
    equal(
        descriptors.len() / 32,
        7 + representatives.len() / 8,
        "dense runtime descriptor IDs",
    )?;
    let function = projection_descriptor(descriptors, runtime(text_fn)?)?;
    equal(projection_word(function, 0)?, 9, "projected Function kind")?;
    equal(projection_word(function, 8)?, 1, "projected Function arity")?;
    equal(
        projection_word(function, 24)?,
        1,
        "projected Function result",
    )?;
    let parameter = usize::try_from(projection_word(function, 16)?)
        .map_err(|_| "projected parameter offset")?;
    equal(
        projection_word(payload, parameter)?,
        4,
        "Text and StaticText execution representation",
    )?;
    projection_record_shape(
        descriptors,
        payload,
        runtime(text_inner)?,
        runtime(text_fn)?,
        b"chosen",
    )?;
    projection_record_shape(
        descriptors,
        payload,
        runtime(text_outer)?,
        runtime(text_inner)?,
        b"nested",
    )?;
    projection_record_shape(
        descriptors,
        payload,
        runtime(different_name)?,
        runtime(text_fn)?,
        b"choice",
    )?;
    eprintln!(
        "source callable projection engine={} carrier_sha256={} semantic_types={} runtime_compounds={} \
         semantic_alias_pairs=3 semantic_prefix_unchanged=true field_name_window_control=true",
        if reference { "reference" } else { "vm" },
        sha256(&case.carrier),
        rows.len(),
        representatives.len() / 8,
    );
    Ok([
        mapping.to_vec(),
        representatives.to_vec(),
        descriptors.to_vec(),
        payload.to_vec(),
    ])
}

#[test]
fn selfhost_source_callable_projection_interns_execution_shapes_without_erasing_semantic_types()
-> Check<()> {
    let vm = observe_projection(false)?;
    let reference = observe_projection(true)?;
    for (index, (vm, reference)) in vm.iter().zip(&reference).enumerate() {
        equal_bytes(
            vm,
            reference,
            &format!("compiler-source projection oracle {index}"),
        )?;
    }
    Ok(())
}

#[test]
fn selfhost_source_callables_match_independent_engines_and_canonical_artifacts() -> Check<()> {
    let image = bootstrap_compiler()?;
    verify_behavior("C1-first", image, "C1-repeat", image)?;
    verify_mutation("C1-first", image, "C1-repeat", image)
}

#[test]
fn selfhost_source_callable_inventory_rejections_reach_exact_layers() -> Check<()> {
    let image = bootstrap_compiler()?;
    verify_rejections("C1-first", image, "C1-repeat", image)
}

#[test]
fn selfhost_source_callable_wide_prefixes_match_independent_engines() -> Check<()> {
    let image = bootstrap_compiler()?;
    for count in [129, 257, 1025] {
        verify_wide("C1-first", image, "C1-repeat", image, count)?;
    }
    Ok(())
}

#[test]
#[ignore = "explicit extent probe before scheduling the 8193-prefix source workload; C0 sizing only"]
fn selfhost_source_callable_prefix_8193_unit_and_carrier_extent_probe() -> Check<()> {
    let (text, expected) = wide_source(8193)?;
    let source = byte_buffer_tests::author_only(&text)
        .map_err(|error| format!("8193 prefix source-authoring dependency: {error}"))?;
    let case = case_from_snapshot(&source)?;
    let host = host_oracle(&case).map_err(|error| format!(
        "8193 prefix canonical-unit/storage dependency: storage_limit={:?} codec_limit={}: {error}",
        ObjectDomain::CompilerUnit.maximum_bytes(),
        crate::platform::compiler::unit::MAXIMUM_COMPILER_UNIT_BYTES,
    ))?;
    let entry = function_named(&case.source, "callable-wide", "entry")?;
    let index = usize::try_from(host.program.function(entry).ok_or("probe entry absent")?.0)
        .map_err(|_| "probe entry index")?;
    let projection = crate::platform::compiler::selfhost_input::export(&host.program, index)
        .map_err(|error| format!("8193 prefix private carrier dependency: {error:?}"))?;
    let profile = projection.get(8..12).ok_or("probe profile extent")?;
    equal(
        profile,
        9_u32.to_le_bytes().as_slice(),
        "probe current Mode-0 profile",
    )?;
    let (units, maximum_unit) = unit_extents(&host);
    wide_markers(&host.program, entry, 8193)?;
    eprintln!(
        "callable extent probe classification=c0-sizing-only source_stages_executed=false \
         count=8193 prefix_payload_bytes=65544 expected={expected} source_bytes={} \
         source_carrier_bytes={} source_carrier_sha256={} private_carrier_bytes={} \
         private_carrier_sha256={} artifact_bytes={} units={} maximum_unit_bytes={} \
         storage_unit_limit={:?} codec_unit_limit={}",
        text.len(),
        case.carrier.len(),
        sha256(&case.carrier),
        projection.len(),
        sha256(&projection),
        host.bytes.len(),
        units,
        maximum_unit,
        ObjectDomain::CompilerUnit.maximum_bytes(),
        crate::platform::compiler::unit::MAXIMUM_COMPILER_UNIT_BYTES,
    );
    Ok(())
}

#[test]
#[ignore = "explicit full 8193-prefix crossing; schedule after the independent unit/carrier extent probe"]
fn selfhost_source_callable_prefix_8193_crosses_native_chunk_target_end_to_end() -> Check<()> {
    let image = bootstrap_compiler()?;
    verify_wide("C1-first", image, "C1-repeat", image, 8193)
}
