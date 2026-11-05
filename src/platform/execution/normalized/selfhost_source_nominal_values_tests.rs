//! Source-driven ordinary nominal values. Each stage receives the complete raw
//! source carrier before an independent C0 build or output oracle is created.
//! This is the closed copyable Record/Variant/Match slice, not owned nominals.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use super::*;
use crate::platform::compiler::{CompilationPayload, CompiledCode, CompiledInstruction as I};
use crate::platform::kernel::{CaseReference, FieldReference, FieldSelector, LocalValueReference};
use crate::platform::semantic_id::BindingId;
use std::fmt::Write as _;

const SOURCE: &str = include_str!("../../../../tests/fixtures/selfhost-source-nominal-values.lkjc");
const MODULE: &str = "nominal-controls";
const TARGET: &str = "nominal-binary";

#[derive(Clone, Copy)]
struct Behavior {
    name: &'static str,
    selector: i64,
    argument: i64,
    expected: i64,
}

impl Behavior {
    fn input(self) -> Vec<u8> {
        [self.selector.to_le_bytes(), self.argument.to_le_bytes()].concat()
    }
}

fn fixture() -> Check<&'static KernelSnapshot> {
    static SOURCE_GRAPH: OnceLock<Check<KernelSnapshot>> = OnceLock::new();
    SOURCE_GRAPH
        .get_or_init(|| byte_buffer_tests::author_only(SOURCE))
        .as_ref()
        .map_err(Clone::clone)
}

fn behaviors(seed: i64) -> Vec<Behavior> {
    let mut cases = [
        ("absent-case", 0, 7),
        ("present-unit-case", 1, 11),
        ("integer-payload", 2, seed),
        ("nominal-record-payload", 3, 43),
        ("closure-payload-captures-record", 4, 47),
        ("record-variant-record-nesting", 5, 46),
        ("retained-closure-and-variant-alias", 6, 86),
        ("closure-captures-nominal-variant", 7, 14),
        ("match-returns-bound-closure", 8, 43),
        ("untaken-division-traps", 9, 7),
        ("different-definition-same-field-shape", 13, 44),
    ]
    .into_iter()
    .map(|(name, selector, expected)| Behavior {
        name,
        selector,
        argument: 0,
        expected,
    })
    .collect::<Vec<_>>();
    for (name, selector) in [
        ("branch-buffer-custody-repair", 10),
        ("nested-reverse-cleanup-and-tail-fallback", 11),
        ("consumed-owner-before-tail-match", 12),
    ] {
        for (argument, expected) in [7, 11, seed, 43, 47].into_iter().enumerate() {
            cases.push(Behavior {
                name,
                selector,
                argument: argument as i64,
                expected,
            });
        }
    }
    // Literal outcomes observe the selected owner contents, both levels of Match,
    // and three synchronous length loans before, within an arm, and after Match.
    // No loan spans Match. The expected values do not come from the compiler's
    // instruction or custody plan.
    for (name, selector, expected) in [
        (
            "match-returns-one-affine-buffer",
            14,
            [230, 452, 127 + seed, 281, 102],
        ),
        (
            "lexically-nested-match-custody",
            15,
            [25, 29, seed + 8, 51, 55],
        ),
        (
            "buffer-borrows-before-within-after-match",
            16,
            [31, 35, seed + 24, 67, 71],
        ),
    ] {
        for (argument, expected) in expected.into_iter().enumerate() {
            cases.push(Behavior {
                name,
                selector,
                argument: argument as i64,
                expected,
            });
        }
    }
    cases.push(Behavior {
        name: "nested-match-outer-absent-case",
        selector: 15,
        argument: 5,
        expected: -92,
    });
    cases
}

fn case_named(source: &KernelSnapshot, declaration: &str, name: &str) -> Check<CaseReference> {
    let declaration = function_named(source, MODULE, declaration)?.declaration;
    source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Case(case), OwnerRecord::Case(record))
                if record.declaration == declaration && record.name.as_str() == name =>
            {
                Some(CaseReference {
                    package: source.root.package_id,
                    case: *case,
                })
            }
            _ => None,
        })
        .ok_or_else(|| format!("missing case {name}"))
}

fn field_named(source: &KernelSnapshot, declaration: &str, name: &str) -> Check<FieldReference> {
    let declaration = function_named(source, MODULE, declaration)?.declaration;
    source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Field(field), OwnerRecord::Field(record))
                if record.declaration == declaration && record.name.as_str() == name =>
            {
                Some(FieldReference {
                    package: source.root.package_id,
                    field: *field,
                })
            }
            _ => None,
        })
        .ok_or_else(|| format!("missing field {name}"))
}

fn expression_ids(source: &KernelSnapshot, root: ExpressionId) -> Check<Vec<ExpressionId>> {
    let mut pending = vec![root];
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    while let Some(id) = pending.pop() {
        ensure(
            seen.insert(id),
            "fixture syntax unexpectedly repeats an expression",
        )?;
        let record = match source.owners.get(&OwnerKey::Expression(id)) {
            Some(OwnerRecord::Expression(record)) => record,
            _ => return Err("fixture expression missing".to_owned()),
        };
        pending.extend(record.children().into_iter().map(|child| child.expression));
        if let E::Let { bindings, .. } = &record.operation {
            for binding in bindings {
                match source.owners.get(&OwnerKey::Binding(*binding)) {
                    Some(OwnerRecord::Binding(record)) => {
                        pending.push(record.value.ok_or("Let initializer missing")?);
                    }
                    _ => return Err("Let binding missing".to_owned()),
                }
            }
        }
        result.push(id);
    }
    Ok(result)
}

fn unique_literal(source: &KernelSnapshot, function: &str, value: i64) -> Check<ExpressionId> {
    let root = function_body(source, MODULE, function)?;
    let matches = expression_ids(source, root)?
        .into_iter()
        .filter(
            |id| matches!(operation(source, *id), Ok(E::I64 { value: actual }) if *actual == value),
        )
        .collect::<Vec<_>>();
    equal(
        matches.len(),
        1,
        "one independently selected source literal",
    )?;
    matches
        .first()
        .copied()
        .ok_or_else(|| "source literal missing".to_owned())
}

fn match_root(source: &KernelSnapshot, function: &str) -> Check<ExpressionId> {
    let mut current = function_body(source, MODULE, function)?;
    loop {
        match operation(source, current)? {
            E::Let { body, .. } => current = *body,
            E::Match { .. } => return Ok(current),
            _ => {
                return Err(format!(
                    "{function}: expected nested Lets followed by Match"
                ));
            }
        }
    }
}

fn arm(source: &KernelSnapshot, function: &str, name: &str) -> Check<kernel::MatchExpressionArm> {
    let wanted = case_named(source, "Choice", name)?;
    match operation(source, match_root(source, function)?)? {
        E::Match { arms, .. } => arms
            .iter()
            .find(|arm| arm.case == wanted)
            .cloned()
            .ok_or_else(|| format!("{function}: missing {name} arm")),
        _ => Err("Match root changed".to_owned()),
    }
}

fn canonical_unit(
    loaded: &LoadedArtifact,
    source: &KernelSnapshot,
    module: &str,
    name: &str,
) -> Check<CompilationUnit> {
    let entry = function_named(source, module, name)?;
    for (key, bytes) in &loaded.objects {
        if key.domain == ObjectDomain::CompilerUnit {
            let unit = CompilationUnit::decode(bytes, *key)
                .map_err(|error| format!("{name}: canonical decode: {error:?}"))?;
            if unit.source.owner == OwnerKey::Declaration(entry.declaration) {
                return Ok(unit);
            }
        }
    }
    Err(format!("{name}: canonical unit missing"))
}

fn function_code(unit: &CompilationUnit) -> Check<&CompiledCode> {
    match &unit.payload {
        CompilationPayload::Function { code, .. } => Ok(code),
        _ => Err("expected a canonical function payload".to_owned()),
    }
}

fn switch_inventory(
    source: &KernelSnapshot,
    loaded: &LoadedArtifact,
    module: &str,
    function: &str,
    variant: &str,
    expected_count: usize,
) -> Check<()> {
    let declaration = function_named(source, module, variant)?;
    let expected = match source
        .owners
        .get(&OwnerKey::Declaration(declaration.declaration))
    {
        Some(OwnerRecord::Declaration(record)) => match &record.payload {
            kernel::DeclarationPayload::Variant { cases, .. } => cases
                .iter()
                .map(|case| CaseReference {
                    package: source.root.package_id,
                    case: *case,
                })
                .collect::<Vec<_>>(),
            _ => return Err("expected canonical Variant declaration".to_owned()),
        },
        _ => return Err("canonical Variant declaration missing".to_owned()),
    };
    equal(
        expected.len(),
        expected_count,
        "complete canonical case count",
    )?;
    ensure(
        expected.windows(2).all(|pair| pair[0] < pair[1]),
        "source cases lack canonical order",
    )?;
    let unit = canonical_unit(loaded, source, module, function)?;
    let code = function_code(&unit)?;
    let switches = code
        .instructions
        .iter()
        .enumerate()
        .filter_map(|(pc, instruction)| {
            if let I::SwitchVariant(arms) = instruction {
                Some((pc, arms))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    equal(switches.len(), 1, "one intended complete Match")?;
    let (pc, arms) = *switches.first().ok_or("compiled Match missing")?;
    equal(
        arms.len(),
        expected_count,
        "canonical Switch retains all arms",
    )?;
    let mut prior = pc;
    let mut locals = BTreeSet::new();
    let mut common_exit = None;
    for (index, jump) in arms.iter().enumerate() {
        let case_index = usize::try_from(jump.case).map_err(|_| "canonical case index extent")?;
        equal(
            unit.tables.cases.get(case_index),
            expected.get(index),
            "canonical case identity/order; table indices need not be numerically sorted",
        )?;
        let source_case = expected.get(index).ok_or("source case position")?;
        let payload = match source.owners.get(&OwnerKey::Case(source_case.case)) {
            Some(OwnerRecord::Case(record)) => record.payload,
            _ => return Err("canonical case owner missing".to_owned()),
        };
        equal(
            jump.binding_local.is_some(),
            payload.is_some(),
            "None versus present Unit binding",
        )?;
        if let Some(local) = jump.binding_local {
            ensure(
                local >= code.parameter_count && local < code.local_count,
                "arm payload is not a nonparameter in-frame local",
            )?;
            ensure(locals.insert(local), "different arms share a payload local")?;
        }
        let target = usize::try_from(jump.target).map_err(|_| "canonical target extent")?;
        ensure(
            target > prior && target < code.instructions.len(),
            "forward canonical arm targets",
        )?;
        let next = arms
            .get(index + 1)
            .map(|arm| usize::try_from(arm.target))
            .transpose()
            .map_err(|_| "canonical next target extent")?;
        // Every canonical arm, including the last, retains its explicit Jump.
        let end = next.unwrap_or(code.instructions.len());
        let exit = code
            .instructions
            .get(target..end)
            .ok_or("canonical arm interval")?
            .iter()
            .find_map(|instruction| {
                if let I::Jump(target) = instruction {
                    Some(*target)
                } else {
                    None
                }
            })
            .ok_or("canonical arm lacks explicit exit Jump")?;
        if let Some(previous) = common_exit {
            equal(
                exit,
                previous,
                "all canonical arm exits share the same join",
            )?;
        } else {
            common_exit = Some(exit);
        }
        prior = target;
    }
    Ok(())
}

fn nominal_markers(case: &InputCase, loaded: &LoadedArtifact) -> Check<()> {
    switch_inventory(&case.source, loaded, MODULE, "inspect", "Choice", 5)?;
    switch_inventory(&case.source, loaded, MODULE, "unused-match", "Choice", 5)?;
    switch_inventory(&case.source, loaded, MODULE, "branch-repair", "Choice", 5)?;
    let choice = canonical_unit(loaded, &case.source, MODULE, "Choice")?;
    let CompilationPayload::Variant { cases, .. } = &choice.payload else {
        return Err("Choice declaration was not emitted as a canonical Variant unit".to_owned());
    };
    for (name, present) in [("none", false), ("marker", true)] {
        let identity = case_named(&case.source, "Choice", name)?;
        let layout = cases
            .iter()
            .find(|layout| {
                usize::try_from(layout.case)
                    .ok()
                    .and_then(|index| choice.tables.cases.get(index))
                    .copied()
                    == Some(identity)
            })
            .ok_or("canonical None/Unit case layout missing")?;
        equal(
            layout.payload.is_some(),
            present,
            "canonical None and Some(Unit) differ",
        )?;
        if let Some(payload) = layout.payload {
            let index = usize::try_from(payload).map_err(|_| "payload type extent")?;
            let ty = choice
                .tables
                .types
                .get(index)
                .ok_or("payload type table entry")?;
            ensure(
                matches!(
                    case.source.types.get(ty).map(|ty| &ty.form),
                    Some(kernel::TypeForm::Unit)
                ),
                "marker must retain its exact Unit payload type",
            )?;
        }
    }
    let point = canonical_unit(loaded, &case.source, MODULE, "Point")?;
    let other = canonical_unit(loaded, &case.source, MODULE, "OtherPoint")?;
    ensure(
        point.source.owner != other.source.owner,
        "same-shaped nominal definitions collapsed",
    )?;
    ensure(
        matches!(point.payload, CompilationPayload::Record { .. })
            && matches!(other.payload, CompilationPayload::Record { .. }),
        "ordinary nominal Record declarations lost their canonical units",
    )?;

    let live = canonical_unit(loaded, &case.source, MODULE, "live-buffer-match")?;
    let code = function_code(&live)?;
    let switch = code
        .instructions
        .iter()
        .position(|i| matches!(i, I::SwitchVariant(_)))
        .ok_or("live-owner Match missing")?;
    let stores = code.instructions[..switch]
        .iter()
        .filter_map(|i| {
            if let I::StoreLocal(local) = i {
                Some(*local)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    equal(stores.len(), 2, "two nested live owner initializations")?;
    let [
        I::Unit,
        I::StoreLocal(inner),
        I::Unit,
        I::StoreLocal(outer),
        I::Return,
    ] = code
        .instructions
        .get(
            code.instructions
                .len()
                .checked_sub(5)
                .ok_or("cleanup extent")?..,
        )
        .ok_or("cleanup suffix")?
    else {
        return Err("live Match lost nested reverse cleanup before Return".to_owned());
    };
    equal(
        (*inner, *outer),
        (stores[1], stores[0]),
        "inner owner clears before outer owner",
    )?;
    let result = function_named(&case.source, MODULE, "result")?;
    let result_calls = code
        .instructions
        .iter()
        .filter(|i| match i {
            I::Call {
                function,
                arguments: 1,
                ..
            } => {
                usize::try_from(*function)
                    .ok()
                    .and_then(|index| live.tables.declarations.get(index))
                    .copied()
                    == Some(result)
            }
            _ => false,
        })
        .count();
    equal(
        result_calls,
        5,
        "every live-owner arm retains its call and cleanup continuation",
    )?;
    // A source-native TailCall with either owner live is rejected by complete
    // Mode0 flow admission. Successful raw compilation plus these exact canonical
    // continuations and all-arm execution exercise the required fallback.
    Ok(())
}

fn evaluate_cases(
    label: &str,
    case: &InputCase,
    host: &HostOracle,
    program: &NormalizedProgram,
    module: &str,
    cases: &[Behavior],
) -> Check<()> {
    let entry = function_named(&case.source, module, "binary")?;
    for &test in cases {
        let input = test.input();
        let arguments = vec![NormalizedValue::bytes(input.as_slice())];
        let policy = NormalizedRunPolicy::foreground();
        let control = ExecutionControl::uncancelled();
        let vm = NormalizedVm::for_test(program, policy)
            .invoke(entry, arguments.clone(), None, &control)
            .map_err(|error| format!("{label}/{}: VM: {error:?}", test.name))?;
        let reference = NormalizedReferenceInterpreter::new(&case.source, &host.program, policy)
            .invoke(entry, arguments, None, &control)
            .map_err(|error| format!("{label}/{}: canonical reference: {error:?}", test.name))?;
        let expected = test.expected.to_le_bytes();
        for (engine, value, frames) in [
            ("vm", vm.0, vm.1.live_call_frames_after),
            (
                "canonical-reference",
                reference.0,
                reference.1.live_call_frames_after,
            ),
        ] {
            equal(
                filter_value(&value)?,
                (0, expected.as_slice()),
                &format!(
                    "{label}/{engine}/{}({},{})",
                    test.name, test.selector, test.argument
                ),
            )?;
            equal(frames, 0, "nominal evaluator frames reclaimed")?;
        }
    }
    Ok(())
}

fn execute_cases(label: &str, image: &[u8], cases: &[Behavior]) -> Check<()> {
    for &test in cases {
        let actual = run_filter(
            &format!("{label}/{}({},{})", test.name, test.selector, test.argument),
            image,
            &test.input(),
            0,
            FOCUSED_DEADLINE,
        )?;
        equal_bytes(
            &actual,
            &test.expected.to_le_bytes(),
            "independent nominal native result",
        )?;
    }
    Ok(())
}

fn compiled_pair(
    stages: &[(&str, &[u8])],
    case: &InputCase,
    label: &str,
    target: &str,
) -> Check<Vec<RawOutputs>> {
    ensure(
        !stages.is_empty(),
        "nominal source control needs an actual compiler stage",
    )?;
    let mut outputs: Vec<RawOutputs> = Vec::new();
    for (name, compiler) in stages {
        let output = raw_outputs(
            &format!("{name}/{label}"),
            compiler,
            case,
            target,
            FOCUSED_DEADLINE,
        )?;
        if let Some(previous) = outputs.first() {
            equal_bytes(&output.elf, &previous.elf, "nominal stage ELF identity")?;
            equal_bytes(
                &output.artifact,
                &previous.artifact,
                "nominal stage artifact identity",
            )?;
        }
        outputs.push(output);
    }
    Ok(outputs)
}

fn verify_semantics(stages: &[(&str, &[u8])]) -> Check<()> {
    let base = case_from_snapshot(fixture()?)?;
    let mut source = base.source.clone();
    let seed = function_body(&source, MODULE, "seed")?;
    ensure(
        matches!(operation(&source, seed)?, E::I64 { value: 42 }),
        "nominal mutation baseline",
    )?;
    set_operation(&mut source, seed, E::I64 { value: 46 })?;
    let changed = case_from_snapshot(&source)?;
    ensure(
        base.carrier != changed.carrier,
        "nominal source mutation did not change raw input",
    )?;
    let baseline = compiled_pair(stages, &base, "nominal-values", TARGET)?;
    let mutated = compiled_pair(stages, &changed, "nominal-values-mutated", TARGET)?;
    let mut selected: Vec<Vec<u8>> = Vec::new();
    for (name, image) in stages {
        let request = base.request("nominal-other", SourceOutputMode::Elf)?;
        let output = run_filter(
            &format!("{name}/nominal-other"),
            image,
            &request,
            0,
            FOCUSED_DEADLINE,
        )?;
        if let Some(previous) = selected.first() {
            equal_bytes(
                &output,
                previous,
                "alternate-target nominal stage ELF identity",
            )?;
        }
        selected.push(output);
    }
    // Every raw request above finishes before C0 creates target code, inferred
    // types, custody evidence, normalized programs or an artifact oracle.
    let host = host_oracle(&base)?;
    let changed_host = host_oracle(&changed)?;
    let mut base_program = None;
    let mut changed_program = None;
    for ((name, _), (before, after)) in stages.iter().zip(baseline.iter().zip(&mutated)) {
        ensure(
            before.elf != after.elf && before.artifact != after.artifact,
            "42-to-46 nominal source mutation did not change generated outputs",
        )?;
        let loaded = strict_artifact(
            &format!("{name}/nominal-values"),
            &before.artifact,
            &base,
            &host,
        )?;
        nominal_markers(&base, &loaded)?;
        let loaded_changed = strict_artifact(
            &format!("{name}/nominal-values-mutated"),
            &after.artifact,
            &changed,
            &changed_host,
        )?;
        nominal_markers(&changed, &loaded_changed)?;
        if base_program.is_none() {
            base_program = Some(
                NormalizedProgram::prepare(loaded)
                    .map_err(|error| format!("nominal artifact prepare: {error:?}"))?,
            );
            changed_program = Some(
                NormalizedProgram::prepare(loaded_changed)
                    .map_err(|error| format!("nominal mutated artifact prepare: {error:?}"))?,
            );
        }
    }
    let cases = behaviors(42);
    let changed_cases = behaviors(46);
    equal(cases.len(), 42, "complete nominal behavior case inventory")?;
    evaluate_cases(
        "nominal",
        &base,
        &host,
        base_program.as_ref().ok_or("nominal program missing")?,
        MODULE,
        &cases,
    )?;
    evaluate_cases(
        "nominal-mutated",
        &changed,
        &changed_host,
        changed_program
            .as_ref()
            .ok_or("mutated nominal program missing")?,
        MODULE,
        &changed_cases,
    )?;
    let native = native_image(
        &host.program,
        function_named(&base.source, MODULE, "binary")?,
    )?;
    let changed_native = native_image(
        &changed_host.program,
        function_named(&changed.source, MODULE, "binary")?,
    )?;
    execute_cases("nominal/C0-native", &native, &cases)?;
    execute_cases("nominal-mutated/C0-native", &changed_native, &changed_cases)?;
    for (((name, _), (before, after)), other) in stages
        .iter()
        .zip(baseline.iter().zip(&mutated))
        .zip(&selected)
    {
        execute_cases(&format!("{name}/nominal"), &before.elf, &cases)?;
        execute_cases(
            &format!("{name}/nominal-mutated"),
            &after.elf,
            &changed_cases,
        )?;
        run_scalar(
            &format!("{name}/nominal-other/execute"),
            other,
            44,
            FOCUSED_DEADLINE,
        )?;
    }
    let other = native_image(
        &host.program,
        function_named(&base.source, MODULE, "other-entry")?,
    )?;
    run_scalar("nominal-other/C0-native", &other, 44, FOCUSED_DEADLINE)?;
    eprintln!(
        "raw source nominal cases={} generations={} modes=2 mutation=42-to-46 \
        source_sha256={} mutated_source_sha256={} independent_engines=vm,reference,c0-native,source-native \
        nominal_scope=ordinary-closed-copyable owned_nominals=false",
        cases.len(),
        stages.len(),
        sha256(&base.carrier),
        sha256(&changed.carrier)
    );
    Ok(())
}

#[derive(Clone, Copy)]
enum HostAdmission {
    Reject,
    RejectCode(&'static str),
    Accept,
}

struct Rejection {
    name: &'static str,
    source: KernelSnapshot,
    diagnostic: i64,
    prior: Option<&'static str>,
    layer: &'static str,
    layer_code: i64,
    host: HostAdmission,
}

impl Rejection {
    fn typed(name: &'static str, source: KernelSnapshot) -> Self {
        Self {
            name,
            source,
            diagnostic: 19,
            prior: None,
            layer: "read-nominal",
            layer_code: 0,
            host: HostAdmission::Reject,
        }
    }
}

fn call_arguments(source: &KernelSnapshot, id: ExpressionId) -> Check<Vec<ExpressionId>> {
    match operation(source, id)? {
        E::Call { arguments, .. } => Ok(arguments.clone()),
        _ => Err("negative fixture expected a Call".to_owned()),
    }
}

fn unique_call_to(source: &KernelSnapshot, function: &str, callee: &str) -> Check<ExpressionId> {
    let wanted = function_named(source, MODULE, callee)?;
    let root = function_body(source, MODULE, function)?;
    let calls = expression_ids(source, root)?
        .into_iter()
        .filter(|id| {
            matches!(operation(source, *id),
            Ok(E::Call { function, .. }) if *function == wanted)
        })
        .collect::<Vec<_>>();
    equal(calls.len(), 1, "one independently selected source call")?;
    calls
        .first()
        .copied()
        .ok_or_else(|| format!("{function}: missing call to {callee}"))
}

fn lexical_binding(source: &KernelSnapshot, function: &str, name: &str) -> Check<BindingId> {
    let root = function_body(source, MODULE, function)?;
    let mut found = None;
    for id in expression_ids(source, root)? {
        if let E::Let { bindings, .. } = operation(source, id)? {
            for binding in bindings {
                let Some(OwnerRecord::Binding(record)) =
                    source.owners.get(&OwnerKey::Binding(*binding))
                else {
                    return Err("fixture lexical binding owner is missing".to_owned());
                };
                if record.name.as_str() == name {
                    ensure(
                        found.is_none(),
                        "fixture lexical binding name must be unique",
                    )?;
                    found = Some(*binding);
                }
            }
        }
    }
    found.ok_or_else(|| format!("{function}: missing lexical binding {name}"))
}

fn binding_annotation(
    source: &mut KernelSnapshot,
    binding: BindingId,
    annotation: Option<TypeObjectDigest>,
) -> Check<()> {
    match source.owners.get_mut(&OwnerKey::Binding(binding)) {
        Some(OwnerRecord::Binding(record)) => {
            ensure(
                record.kind == kernel::BindingKind::MatchPayload && record.value.is_none(),
                "negative fixture payload binding kind/value",
            )?;
            record.declared_type = annotation;
            Ok(())
        }
        _ => Err("negative fixture payload binding missing".to_owned()),
    }
}

fn rejections(base: &KernelSnapshot) -> Check<Vec<Rejection>> {
    let mut out = Vec::new();
    let mut source = base.clone();
    let value = unique_literal(&source, "unused-point", 1)?;
    set_operation(&mut source, value, E::Bool { value: true })?;
    out.push(Rejection::typed("uncalled-record-field-type", source));

    let mut source = base.clone();
    let body = arm(&source, "selected-only", "point")?.body;
    let argument = *call_arguments(&source, body)?
        .first()
        .ok_or("untaken trap argument")?;
    set_operation(&mut source, argument, E::Bool { value: true })?;
    out.push(Rejection::typed("untaken-match-arm-call-type", source));

    let mut source = base.clone();
    let value = unique_literal(&source, "other-entry", 53)?;
    set_operation(&mut source, value, E::Bool { value: true })?;
    out.push(Rejection::typed(
        "unselected-target-nominal-field-type",
        source,
    ));

    let mut source = base.clone();
    let value = unique_literal(&source, "unused-number", 3)?;
    set_operation(&mut source, value, E::Bool { value: true })?;
    out.push(Rejection::typed("variant-payload-exact-type", source));

    let mut source = base.clone();
    let body = function_body(&source, MODULE, "unused-field")?;
    let foreign = field_named(&source, "OtherPoint", "left")?;
    let mut changed = operation(&source, body)?.clone();
    match &mut changed {
        E::Field { selector, .. } => *selector = FieldSelector::Nominal(foreign),
        _ => return Err("nominal field negative shape".to_owned()),
    }
    set_operation(&mut source, body, changed)?;
    out.push(Rejection::typed(
        "same-shaped-foreign-nominal-field",
        source,
    ));

    let mut source = base.clone();
    let root = match_root(&source, "unused-match")?;
    let marker = case_named(&source, "Choice", "marker")?;
    let mut changed = operation(&source, root)?.clone();
    let removed = match &mut changed {
        E::Match { arms, .. } => arms
            .iter_mut()
            .find(|arm| arm.case == marker)
            .ok_or("Unit arm missing")?
            .payload_binding
            .take()
            .ok_or("Unit binder missing")?,
        _ => return Err("Unit binder negative Match missing".to_owned()),
    };
    ensure(
        source.owners.remove(&OwnerKey::Binding(removed)).is_some(),
        "Unit binder owner removal",
    )?;
    set_operation(&mut source, root, changed)?;
    out.push(Rejection::typed("present-unit-requires-a-binder", source));

    let boolean = base
        .types
        .iter()
        .find_map(|(id, ty)| matches!(ty.form, kernel::TypeForm::Bool).then_some(*id))
        .ok_or("fixture Bool type")?;
    for (name, annotation) in [
        ("payload-binding-exact-annotation", Some(boolean)),
        ("payload-binding-missing-annotation", None),
    ] {
        let mut source = base.clone();
        let binding = arm(&source, "unused-match", "marker")?
            .payload_binding
            .ok_or("annotation binder")?;
        binding_annotation(&mut source, binding, annotation)?;
        let mut test = Rejection::typed(name, source);
        test.host = HostAdmission::RejectCode("kernel_type_match_binding");
        out.push(test);
    }

    let mut source = base.clone();
    let root = match_root(&source, "unused-match")?;
    let none = case_named(&source, "Choice", "none")?;
    let mut changed = operation(&source, root)?.clone();
    let removed = match &mut changed {
        E::Match { arms, .. } => {
            let index = arms
                .iter()
                .position(|arm| arm.case == none)
                .ok_or("absent arm index")?;
            arms.remove(index)
        }
        _ => return Err("exhaustiveness negative Match missing".to_owned()),
    };
    ensure(
        removed.payload_binding.is_none()
            && matches!(operation(&source, removed.body)?, E::I64 { value: 31 }),
        "exhaustiveness removal must delete only the intended leaf",
    )?;
    ensure(
        source
            .owners
            .remove(&OwnerKey::Expression(removed.body))
            .is_some(),
        "removed arm leaf owner",
    )?;
    set_operation(&mut source, root, changed)?;
    out.push(Rejection::typed("nonexhaustive-uncalled-match", source));

    let mut source = base.clone();
    let body = arm(&source, "unused-match", "number")?.body;
    set_operation(&mut source, body, E::Bool { value: true })?;
    out.push(Rejection::typed(
        "incompatible-uncalled-match-result",
        source,
    ));

    let mut source = base.clone();
    let root = match_root(&source, "unused-match")?;
    let none = case_named(&source, "Choice", "none")?;
    let foreign = case_named(&source, "Outer", "empty")?;
    let mut changed = operation(&source, root)?.clone();
    match &mut changed {
        E::Match { arms, .. } => {
            arms.iter_mut()
                .find(|arm| arm.case == none)
                .ok_or("foreign-case seed arm")?
                .case = foreign;
            arms.sort_by_key(|arm| arm.case);
        }
        _ => return Err("foreign-case Match missing".to_owned()),
    }
    set_operation(&mut source, root, changed)?;
    out.push(Rejection::typed("match-case-from-another-variant", source));

    let mut source = base.clone();
    let body = arm(&source, "branch-repair", "number")?.body;
    let items = match operation(&source, body)? {
        E::Sequence { items } => items.clone(),
        _ => return Err("affine negative sequence missing".to_owned()),
    };
    equal(items.len(), 3, "affine branch's two disposals and result")?;
    let first = *call_arguments(&source, items[0])?
        .first()
        .ok_or("first consume argument")?;
    let second = *call_arguments(&source, items[1])?
        .first()
        .ok_or("second consume argument")?;
    let replacement = operation(&source, second)?.clone();
    ensure(
        matches!(
            &replacement,
            E::Local {
                value: LocalValueReference::LexicalBinding(_)
            }
        ),
        "consume mutation must preserve a local leaf",
    )?;
    set_operation(&mut source, first, replacement)?;
    out.push(Rejection::typed(
        "match-arm-double-consumes-bytebuffer",
        source,
    ));

    let mut source = base.clone();
    let body = arm(&source, "inspect", "number")?.body;
    let sibling = arm(&source, "inspect", "point")?
        .payload_binding
        .ok_or("sibling binder")?;
    set_operation(
        &mut source,
        body,
        E::Local {
            value: LocalValueReference::MatchPayload(sibling),
        },
    )?;
    out.push(Rejection {
        name: "match-binder-is-visible-only-in-its-own-arm",
        source,
        diagnostic: 15,
        prior: Some("read-graph"),
        layer: "read-syntax",
        layer_code: 15,
        host: HostAdmission::RejectCode("kernel_full_lexical_scope"),
    });

    let mut source = base.clone();
    let root = match_root(&source, "unused-match")?;
    let none = case_named(&source, "Choice", "none")?;
    let missing = crate::platform::semantic_id::CaseId::migrate(b"source-nominal-missing-case", 0);
    ensure(
        !source.owners.contains_key(&OwnerKey::Case(missing)),
        "missing-case identity collision",
    )?;
    let mut changed = operation(&source, root)?.clone();
    match &mut changed {
        E::Match { arms, .. } => {
            arms.iter_mut()
                .find(|arm| arm.case == none)
                .ok_or("missing-endpoint seed arm")?
                .case
                .case = missing;
            arms.sort_by_key(|arm| arm.case);
        }
        _ => return Err("missing-endpoint Match absent".to_owned()),
    }
    set_operation(&mut source, root, changed)?;
    out.push(Rejection {
        name: "match-case-endpoint-missing",
        source,
        diagnostic: 14,
        prior: None,
        layer: "read-graph",
        layer_code: 14,
        host: HostAdmission::Reject,
    });

    let mut source = base.clone();
    let before = kernel::semantic_state_digest(base)
        .map_err(|error| format!("baseline state: {error:?}"))?;
    let ty = kernel::TypeObject::new(kernel::TypeForm::F64)
        .map_err(|error| format!("unused F64: {error:?}"))?;
    let identity = kernel::encode_type_object(&ty)
        .map_err(|error| format!("F64 encoding: {error:?}"))?
        .0;
    ensure(
        source.types.insert(identity, ty).is_none(),
        "F64 negative was already supplied",
    )?;
    equal(
        kernel::semantic_state_digest(&source)
            .map_err(|error| format!("unused state: {error:?}"))?,
        before,
        "unused descriptor leaves semantic-state unchanged; carrier SHA still binds it",
    )?;
    out.push(Rejection {
        name: "unused-supplied-unsupported-descriptor",
        source,
        diagnostic: 13,
        prior: None,
        layer: "read-nominal",
        layer_code: 13,
        host: HostAdmission::RejectCode("kernel_full_type_unreachable"),
    });

    // These three mutations preserve the value types and syntax containment.
    // Rejection must therefore reach ownership, not a mismatched result type.
    let mut source = base.clone();
    let site = unique_call_to(&source, "affine-match-output", "freeze")?;
    let argument = *call_arguments(&source, site)?
        .first()
        .ok_or("affine result freeze argument")?;
    let moved = lexical_binding(&source, "affine-match-output", "left")?;
    set_operation(
        &mut source,
        argument,
        E::Local {
            value: LocalValueReference::LexicalBinding(moved),
        },
    )?;
    let mut test = Rejection::typed("match-affine-result-reuses-consumed-input", source);
    test.host = HostAdmission::RejectCode("kernel_buffer_ownership");
    out.push(test);

    let mut source = base.clone();
    let site = unique_call_to(&source, "nested-match-output", "size")?;
    let argument = *call_arguments(&source, site)?
        .first()
        .ok_or("nested continuation borrow argument")?;
    let moved = lexical_binding(&source, "nested-match-output", "nested-first")?;
    set_operation(
        &mut source,
        argument,
        E::Local {
            value: LocalValueReference::LexicalBinding(moved),
        },
    )?;
    let mut test = Rejection::typed("nested-match-continuation-borrows-moved-owner", source);
    test.host = HostAdmission::RejectCode("kernel_buffer_ownership");
    out.push(test);

    let mut source = base.clone();
    let answer = lexical_binding(&source, "borrowed-match-output", "answer")?;
    let call = match source.owners.get(&OwnerKey::Binding(answer)) {
        Some(OwnerRecord::Binding(record)) => record.value.ok_or("answer initializer missing")?,
        _ => return Err("answer binding missing".to_owned()),
    };
    let matched = *call_arguments(&source, call)?
        .get(1)
        .ok_or("length-plus-Match argument")?;
    let none = case_named(&source, "Choice", "none")?;
    let body = match operation(&source, matched)? {
        E::Match { arms, .. } => {
            arms.iter()
                .find(|arm| arm.case == none)
                .ok_or("length-loan Match absent arm")?
                .body
        }
        _ => {
            return Err(
                "length-plus-Match fixture must evaluate Match in the second argument".to_owned(),
            );
        }
    };
    let read = *call_arguments(&source, body)?
        .first()
        .ok_or("arm length read")?;
    let consumed = function_named(&source, MODULE, "consume-size")?;
    let borrowed = function_named(&source, MODULE, "size")?;
    let mut changed = operation(&source, read)?.clone();
    match &mut changed {
        E::Call { function, .. } => {
            equal(
                *function,
                borrowed,
                "negative starts with the exact borrowing intrinsic",
            )?;
            *function = consumed;
        }
        _ => return Err("moved-owner negative expected a length call".to_owned()),
    }
    set_operation(&mut source, read, changed)?;
    let mut test = Rejection::typed(
        "match-arm-consumes-owner-before-continuation-borrow",
        source,
    );
    test.host = HostAdmission::RejectCode("kernel_buffer_ownership");
    out.push(test);

    // A general borrowed helper is valid host meaning but deliberately outside
    // this source profile. E1 admits its unchanged types and containment; E2
    // must report unsupported13, independently of the later host-valid oracle.
    let mut source = base.clone();
    let helper = function_named(&source, MODULE, "consume-size")?;
    let parameter = match source
        .owners
        .get(&OwnerKey::Declaration(helper.declaration))
    {
        Some(OwnerRecord::Declaration(record)) => match &record.payload {
            kernel::DeclarationPayload::Function(function) => {
                equal(
                    function.parameters.len(),
                    1,
                    "one borrowed-helper parameter",
                )?;
                function.parameters[0]
            }
            _ => return Err("borrowed-helper declaration is not a function".to_owned()),
        },
        _ => return Err("borrowed-helper declaration missing".to_owned()),
    };
    match source.owners.get_mut(&OwnerKey::Parameter(parameter)) {
        Some(OwnerRecord::Parameter(record)) => {
            equal(
                record.use_mode,
                kernel::ParameterUse::Consume,
                "helper starts with consuming parameter",
            )?;
            record.use_mode = kernel::ParameterUse::Borrow;
        }
        _ => return Err("borrowed-helper parameter missing".to_owned()),
    }
    out.push(Rejection {
        name: "borrowed-helper-outside-current-source-profile",
        source,
        diagnostic: 13,
        prior: None,
        layer: "read-nominal",
        layer_code: 0,
        host: HostAdmission::Accept,
    });

    // Zero-parameter nominal recursion is not an expanding slot cycle. The
    // source E1 reader must admit it, while this deliberately acyclic E2 slice
    // reports unsupported13. No fabricated semantic cycle rejection is allowed.
    let recursive = SOURCE.replacen("(module create nominal-controls",
        "(module create nominal-controls\n    (record create Recursive (visibility private) (field create next (type Recursive)))", 1);
    ensure(recursive != SOURCE, "recursive fixture insertion marker")?;
    let source = byte_buffer_tests::author_only(&recursive)?;
    out.push(Rejection {
        name: "recursive-nominal-is-outside-current-copyable-profile",
        source,
        diagnostic: 13,
        prior: None,
        layer: "read-nominal",
        layer_code: 0,
        host: HostAdmission::Accept,
    });
    equal(out.len(), 20, "complete nominal negative inventory")?;
    Ok(out)
}

fn verify_rejections(stages: &[(&str, &[u8])]) -> Check<()> {
    let base = fixture()?;
    kernel::validate_full(base)
        .map_err(|error| format!("nominal baseline must be host-valid: {error:?}"))?;
    let baseline_carrier = graph::packet(base);
    let mut names = BTreeSet::new();
    for test in rejections(base)? {
        ensure(names.insert(test.name), "duplicate nominal negative label")?;
        // graph::packet re-encodes every changed owner/type and rebuilds hashes,
        // source-state and synthetic revision bindings. These are test inputs,
        // never edits to an accepted GraphRepository.
        let carrier = graph::packet(&test.source);
        ensure(
            carrier != baseline_carrier,
            "negative mutation did not change the complete raw carrier",
        )?;
        if let Some(prior) = test.prior {
            admission_layer_result(prior, &carrier, 0)?;
        }
        admission_layer_result(test.layer, &carrier, test.layer_code)?;
        let selected = target_named(&test.source, TARGET)?;
        for mode in [SourceOutputMode::Elf, SourceOutputMode::CanonicalArtifact] {
            let request = raw_source::request(&carrier, selected, mode)
                .map_err(|error| format!("{}: request: {error:?}", test.name))?;
            for (producer, image) in stages {
                let bytes = run_filter(
                    &format!("{producer}/nominal-negative/{}/{mode:?}", test.name),
                    image,
                    &request,
                    test.diagnostic,
                    FOCUSED_DEADLINE,
                )?;
                ensure(bytes.is_empty(), "rejected nominal source published output")?;
            }
        }
        // Independent host admission is observed only after raw stage rejection;
        // no host diagnosis, inferred types or custody proof was supplied to it.
        match test.host {
            HostAdmission::Accept => {
                kernel::validate_full(&test.source)
                    .map_err(|error| format!("{} must remain host-valid: {error:?}", test.name))?;
            }
            HostAdmission::Reject | HostAdmission::RejectCode(_) => {
                let errors = kernel::validate_full(&test.source)
                    .err()
                    .ok_or_else(|| format!("host accepted {}", test.name))?;
                if let HostAdmission::RejectCode(code) = test.host {
                    ensure(
                        errors.iter().any(|error| error.code == code),
                        &format!("{}: host must reach {code}: {errors:?}", test.name),
                    )?;
                    if code == "kernel_full_type_unreachable" {
                        equal(
                            errors.len(),
                            1,
                            "unused descriptor has one exact host-only reachability refusal",
                        )?;
                    }
                }
                ensure(
                    errors
                        .iter()
                        .all(|error| error.class != crate::platform::DiagnosticClass::Resource),
                    "host negative stopped at a resource policy instead of intended semantics",
                )?;
            }
        }
        equal_bytes(
            &graph::packet(&test.source),
            &carrier,
            "host inspection did not mutate raw evidence",
        )?;
        eprintln!(
            "raw source nominal negative={} diagnostic={} e1_layer={} e1_code={} \
            carrier_sha256={} supplied_owners={} supplied_types={} generations={} modes=2",
            test.name,
            test.diagnostic,
            test.layer,
            test.layer_code,
            sha256(&carrier),
            test.source.owners.len(),
            test.source.types.len(),
            stages.len()
        );
    }
    equal(
        names.len(),
        20,
        "nonempty complete source nominal rejection corpus",
    )
}

fn wide_literal(index: usize) -> Check<i64> {
    i64::try_from(index)
        .ok()
        .and_then(|value| value.checked_mul(3))
        .and_then(|value| value.checked_add(1000))
        .ok_or_else(|| "wide literal extent".to_owned())
}

fn wide_source(count: usize) -> Check<String> {
    ensure(count > 0, "wide Match fixture must be nonempty")?;
    let mut text = String::from(
        r#"declarations.begin
(units
  (module create nominal-wide
    (external create eq (visibility private) (implementation core.i64.equal)
      (parameter create a (type I64)) (parameter create b (type I64)) (returns Bool))
    (external create empty (visibility private) (implementation core.buffer.empty) (returns ByteBuffer))
    (external create word (visibility private) (implementation core.buffer.append-i64-le)
      (parameter create value (type I64))
      (parameter create buffer (type ByteBuffer) (use consume)) (returns ByteBuffer))
    (external create freeze (visibility private) (implementation core.buffer.freeze)
      (parameter create buffer (type ByteBuffer) (use consume)) (returns Bytes))
    (external create read (visibility private) (implementation core.bytes.read-i64-le)
      (parameter create data (type Bytes)) (parameter create index (type I64)) (returns I64))
    (variant create Wide (visibility private)
"#,
    );
    for index in 0..count {
        let payload = match index % 3 {
            0 => "",
            1 => " (payload Unit)",
            _ => " (payload I64)",
        };
        writeln!(text, "      (case create c{index}{payload})").map_err(|_| "wide case text")?;
    }
    text.push_str("    )\n    (function create choose (visibility private) (effect pure)\n      (parameter create selector (type I64)) (returns Wide)\n      (body ");
    for index in 0..count {
        if index + 1 < count {
            write!(text, "(if (call eq (local selector) (i64 {index})) ")
                .map_err(|_| "wide choice condition")?;
        }
        let payload = match index % 3 {
            0 => String::new(),
            1 => " (unit)".to_owned(),
            _ => format!(" (i64 {})", wide_literal(index)?),
        };
        write!(text, "(variant Wide::c{index}{payload}) ").map_err(|_| "wide constructor")?;
    }
    text.push_str(&")".repeat(count - 1));
    text.push_str("))\n    (function create score (visibility private) (effect pure)\n      (parameter create value (type Wide)) (returns I64)\n      (body (match (local value)\n");
    for index in 0..count {
        let value = wide_literal(index)?;
        match index % 3 {
            0 => writeln!(text, "        (arm Wide::c{index} (i64 {value}))"),
            1 => writeln!(text, "        (arm Wide::c{index} (payload u{index} Unit) (sequence (local u{index}) (i64 {value})))"),
            _ => writeln!(text, "        (arm Wide::c{index} (payload n{index} I64) (local n{index}))"),
        }.map_err(|_| "wide arm text")?;
    }
    text.push_str(r#"      )))
    (function create binary (visibility public) (effect pure)
      (parameter create data (type Bytes))
      (returns (record (diagnostic I64) (image Bytes)))
      (body (record structural (field diagnostic (i64 0))
        (field image
          (let
            (binding value (call score (call choose (call read (local data) (i64 0)))))
            (binding initial (type ByteBuffer) (call empty))
            (binding output (type ByteBuffer) (call word (local value) (local initial)))
            (in (call freeze (local output))))))))
    (component create app (visibility private)
      (port create run (type (function (Bytes) (record (diagnostic I64) (image Bytes)))) (function binary))))
  (target create nominal-wide (component nominal-wide::app)
    (runner command) (port nominal-wide::app::run)))
declarations.end
"#);
    Ok(text)
}

fn wide_behaviors(source: &KernelSnapshot, count: usize) -> Check<Vec<Behavior>> {
    let declaration = function_named(source, "nominal-wide", "Wide")?;
    let cases = match source
        .owners
        .get(&OwnerKey::Declaration(declaration.declaration))
    {
        Some(OwnerRecord::Declaration(record)) => match &record.payload {
            kernel::DeclarationPayload::Variant { cases, .. } => cases,
            _ => return Err("wide Variant definition missing".to_owned()),
        },
        _ => return Err("wide Variant owner missing".to_owned()),
    };
    equal(cases.len(), count, "wide canonical inventory count")?;
    let mut selected = BTreeSet::from([0_usize, 1, 2]);
    for ordinal in [0, count / 2, 255, 256] {
        let case = cases.get(ordinal).ok_or("wide canonical ordinal")?;
        let authored = match source.owners.get(&OwnerKey::Case(*case)) {
            Some(OwnerRecord::Case(record)) => record
                .name
                .as_str()
                .strip_prefix('c')
                .ok_or("wide case name prefix")?
                .parse::<usize>()
                .map_err(|_| "wide authored case number")?,
            _ => return Err("wide canonical case owner missing".to_owned()),
        };
        ensure(
            authored < count,
            "wide authored case number outside fixture",
        )?;
        selected.insert(authored);
        eprintln!(
            "wide nominal execution selects canonical_ordinal={ordinal} authored_case=c{authored} expected={}",
            wide_literal(authored)?
        );
    }
    selected
        .into_iter()
        .map(|index| {
            Ok(Behavior {
                name: "wide-canonical-case",
                selector: i64::try_from(index).map_err(|_| "wide selector extent")?,
                argument: 0,
                expected: wide_literal(index)?,
            })
        })
        .collect()
}

fn verify_wide(stages: &[(&str, &[u8])]) -> Check<()> {
    const COUNT: usize = 257;
    let text = wide_source(COUNT)?;
    let source = byte_buffer_tests::author_only(&text)?;
    let case = case_from_snapshot(&source)?;
    let outputs = compiled_pair(stages, &case, "nominal-wide-257", "nominal-wide")?;
    let host = host_oracle(&case)?;
    let mut program = None;
    for ((name, _), output) in stages.iter().zip(&outputs) {
        let loaded = strict_artifact(
            &format!("{name}/nominal-wide-257"),
            &output.artifact,
            &case,
            &host,
        )?;
        switch_inventory(
            &case.source,
            &loaded,
            "nominal-wide",
            "score",
            "Wide",
            COUNT,
        )?;
        if program.is_none() {
            program = Some(
                NormalizedProgram::prepare(loaded)
                    .map_err(|error| format!("wide nominal prepare: {error:?}"))?,
            );
        }
    }
    let cases = wide_behaviors(&case.source, COUNT)?;
    evaluate_cases(
        "nominal-wide",
        &case,
        &host,
        program.as_ref().ok_or("wide program missing")?,
        "nominal-wide",
        &cases,
    )?;
    let native = native_image(
        &host.program,
        function_named(&case.source, "nominal-wide", "binary")?,
    )?;
    execute_cases("nominal-wide/C0-native", &native, &cases)?;
    for ((name, _), output) in stages.iter().zip(&outputs) {
        execute_cases(&format!("{name}/nominal-wide"), &output.elf, &cases)?;
    }
    // 257 crosses the byte-sized ordinal shape and reaches the actual canonical
    // positions255/256. The current wire already uses wider integers; this is
    // a wide framing/order/complete-path control, not a retired-capacity claim.
    eprintln!(
        "raw source wide Match cases={COUNT} selected_cases={} source_bytes={} \
        carrier_bytes={} carrier_sha256={} artifact_bytes={} modes=2 generations={} \
        canonical_ordinals_beyond_u8=true retired_capacity_limit_claim=false",
        cases.len(),
        text.len(),
        case.carrier.len(),
        sha256(&case.carrier),
        host.bytes.len(),
        stages.len()
    );
    Ok(())
}

pub(super) fn verify_stages(
    left_name: &str,
    left: &[u8],
    right_name: &str,
    right: &[u8],
) -> Check<()> {
    let stages = [(left_name, left), (right_name, right)];
    verify_semantics(&stages)?;
    verify_rejections(&stages)?;
    verify_wide(&stages)
}

#[test]
fn selfhost_source_nominal_values_match_independent_engines_artifacts_and_mutations() -> Check<()> {
    verify_semantics(&[("C1", bootstrap_compiler()?)])
}

#[test]
fn selfhost_source_nominal_values_inventory_rejections_reach_exact_layers() -> Check<()> {
    verify_rejections(&[("C1", bootstrap_compiler()?)])
}

#[test]
fn selfhost_source_nominal_match_257_cases_preserve_complete_canonical_dispatch() -> Check<()> {
    verify_wide(&[("C1", bootstrap_compiler()?)])
}
