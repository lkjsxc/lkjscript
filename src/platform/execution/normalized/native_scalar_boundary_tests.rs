//! Runtime I64 boundaries through direct, callable, nominal and owned paths.
//! Each image is prepared once; neither test initializes raw compiler generations.
use super::super::*;
use super::{PhasePolicy, caller::Mode, verify_observed};
use crate::platform::execution::normalized::prepare::{
    NormalizedCode, NormalizedFunctionBody, NormalizedInstruction as I,
};
use crate::platform::execution::normalized::tests::selfhost_tests::artifact;
use crate::platform::kernel::TypeForm;
use std::sync::{Arc, OnceLock};

const SOURCE: &str = include_str!("../../../../tests/fixtures/native-scalar-boundaries.lkjc");
const MODULE: &str = "native-scalar-boundaries";
const DEADLINE: Duration = Duration::from_secs(120);

struct Fixture {
    source: KernelSnapshot,
    program: NormalizedProgram,
    entry: DeclarationReference,
    rust: Vec<u8>,
    maintained: Arc<[u8]>,
}

fn code<'a>(
    source: &KernelSnapshot,
    program: &'a NormalizedProgram,
    name: &str,
) -> Check<&'a NormalizedCode> {
    let entry = function_named(source, MODULE, name)?;
    let index = program.function(entry).ok_or("boundary function absent")?;
    let index = usize::try_from(index.0).map_err(|_| "boundary function index extent")?;
    match &program.functions[index].body {
        NormalizedFunctionBody::Code(code) => Ok(code),
        NormalizedFunctionBody::External(_) => Err("boundary graph body absent".to_owned()),
    }
}

fn fallback_shapes(source: &KernelSnapshot, program: &NormalizedProgram) -> Check<()> {
    let fallback = program
        .function(function_named(source, MODULE, "fallback")?)
        .ok_or("boundary fallback absent")?;
    let index = usize::try_from(fallback.0).map_err(|_| "boundary fallback index extent")?;
    let function = &program.functions[index];
    let [value, flag] = function.parameters.as_ref() else {
        return Err("boundary fallback must have exactly two parameters".to_owned());
    };
    ensure(
        matches!(
            program.types.get(&value.ty).map(|ty| &ty.form),
            Some(TypeForm::I64)
        ) && matches!(
            program.types.get(&flag.ty).map(|ty| &ty.form),
            Some(TypeForm::Bool)
        ) && matches!(
            program.types.get(&function.result.ty).map(|ty| &ty.form),
            Some(TypeForm::I64)
        ),
        "boundary fallback must retain its exact I64/Bool signature",
    )?;

    // The two successors put a raw parameter and a boxed field projection at
    // one Return. This forces whole-body scalar-flow decline, independently of
    // the runtime live-owner tail fallback exercised by held.
    let instructions = code(source, program, "fallback")?.instructions.as_ref();
    let [
        flag_read,
        I::JumpIfFalse(false_target),
        raw_read,
        I::Jump(join_target),
        field_read,
        I::Record {
            layout: None,
            fields,
            ..
        },
        I::Field(field),
        I::Return,
    ] = instructions
    else {
        return Err("boundary fallback lost its raw/boxed branch join".to_owned());
    };
    ensure(
        matches!(flag_read, I::LoadLocal { local: 1, .. } | I::MoveLocal(1))
            && matches!(raw_read, I::LoadLocal { local: 0, .. } | I::MoveLocal(0))
            && matches!(field_read, I::LoadLocal { local: 0, .. } | I::MoveLocal(0))
            && *false_target == 4
            && *join_target == 7
            && fields.len() == 1
            && fields.first() == Some(field),
        "boundary fallback successors must converge with raw and boxed I64 values",
    )?;
    for (name, tail) in [("fallback-call", false), ("fallback-tail", true)] {
        let instructions = &code(source, program, name)?.instructions;
        let ordinary = instructions
            .iter()
            .filter(|instruction| {
                matches!(instruction, I::Call { function, arguments: 2, .. } if *function == fallback)
            })
            .count();
        let tail_calls = instructions
            .iter()
            .filter(|instruction| {
                matches!(instruction, I::TailCall { function, arguments: 2, .. } if *function == fallback)
            })
            .count();
        equal(
            (ordinary, tail_calls),
            if tail { (0, 1) } else { (1, 0) },
            &format!("{name}: direct fallback call form"),
        )?;
    }
    for (name, tail) in [("fallback-invoke", false), ("fallback-tail-invoke", true)] {
        let instructions = &code(source, program, name)?.instructions;
        ensure(
            instructions.iter().any(|instruction| {
                matches!(instruction, I::FunctionValue { function, .. } if *function == fallback)
            }) && instructions
                .iter()
                .any(|instruction| matches!(instruction, I::Bind { arguments: 1 })),
            "boundary callable must bind the exact fallback function",
        )?;
        let ordinary = instructions
            .iter()
            .filter(|instruction| matches!(instruction, I::Invoke { arguments: 1 }))
            .count();
        let tail_invokes = instructions
            .iter()
            .filter(|instruction| matches!(instruction, I::TailInvoke { arguments: 1 }))
            .count();
        equal(
            (ordinary, tail_invokes),
            if tail { (0, 1) } else { (1, 0) },
            &format!("{name}: callable fallback call form"),
        )?;
    }
    Ok(())
}

fn fixture() -> Check<&'static Fixture> {
    static FIXTURE: OnceLock<Check<Fixture>> = OnceLock::new();
    FIXTURE
        .get_or_init(|| {
            let source = byte_buffer_tests::author_only(SOURCE)?;
            let program = prepare_snapshot(&source);
            fallback_shapes(&source, &program)?;
            let entry = function_named(&source, MODULE, "entry")?;
            let identity = program
                .function(function_named(&source, MODULE, "identity")?)
                .ok_or("boundary identity absent")?;
            ensure(
                code(&source, &program, "direct")?
                    .instructions
                    .iter()
                    .any(|instruction| matches!(instruction, I::Call { function, .. } if *function == identity)),
                "boundary direct path lost its ordinary call",
            )?;
            for (name, arguments) in [("factory", 1), ("captured", 0)] {
                ensure(
                    code(&source, &program, name)?
                        .instructions
                        .iter()
                        .any(|instruction| matches!(instruction, I::Bind { arguments: found } if *found == arguments)),
                    "boundary fixture lost its nonempty or empty Bind",
                )?;
            }
            ensure(
                code(&source, &program, "nominal")?
                    .instructions
                    .iter()
                    .any(|instruction| matches!(instruction, I::SwitchVariant(cases) if cases.len() == 3)),
                "boundary nominal fixture lost exhaustive payload dispatch",
            )?;
            let word = program
                .function(function_named(&source, MODULE, "word")?)
                .ok_or("boundary buffer writer absent")?;
            let add = program
                .function(function_named(&source, MODULE, "add")?)
                .ok_or("boundary checked addition absent")?;
            let instructions = &code(&source, &program, "checked")?.instructions;
            let allocation = instructions
                .iter()
                .position(|instruction| matches!(instruction, I::Call { function, .. } if *function == word))
                .ok_or("boundary overflow lost owned backing allocation")?;
            let arithmetic = instructions
                .iter()
                .position(|instruction| matches!(instruction, I::Call { function, .. } | I::TailCall { function, .. } if *function == add))
                .ok_or("boundary overflow lost checked arithmetic")?;
            ensure(
                allocation < arithmetic,
                "boundary overflow must follow the owned allocation",
            )?;
            let rust = native_image(&program, entry)?;
            let index = program.function(entry).ok_or("boundary entry absent")?;
            let index = usize::try_from(index.0).map_err(|_| "boundary entry index extent")?;
            let maintained =
                artifact::selfhost_native_probe(&program, index, "native-scalar-boundaries::entry");
            Ok(Fixture {
                source,
                program,
                entry,
                rust,
                maintained,
            })
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn request(selector: i64, value: i64, delta: i64) -> Vec<u8> {
    [
        selector.to_le_bytes(),
        value.to_le_bytes(),
        delta.to_le_bytes(),
    ]
    .concat()
}

fn independent_success(fixture: &Fixture, input: &[u8], expected: &[u8]) -> Check<()> {
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::foreground();
    let arguments = vec![NormalizedValue::bytes(input)];
    let vm = NormalizedVm::for_test(&fixture.program, policy)
        .invoke(fixture.entry, arguments.clone(), None, &control)
        .map_err(|e| format!("scalar boundary VM: {e:?}"))?;
    let reference = NormalizedReferenceInterpreter::new(&fixture.source, &fixture.program, policy)
        .invoke(fixture.entry, arguments, None, &control)
        .map_err(|e| format!("scalar boundary reference: {e:?}"))?;
    for (label, value, frames) in [
        ("VM", vm.0, vm.1.live_call_frames_after),
        ("reference", reference.0, reference.1.live_call_frames_after),
    ] {
        let (diagnostic, output) = filter_value(&value)?;
        equal(diagnostic, 0, &format!("{label}: boundary diagnostic"))?;
        equal_bytes(output, expected, &format!("{label}: literal I64 bytes"))?;
        equal(frames, 0, &format!("{label}: boundary frame cleanup"))?;
    }
    Ok(())
}

fn phase_policy() -> PhasePolicy {
    PhasePolicy {
        address_space: None,
        deadline: DEADLINE,
    }
}

#[test]
fn native_scalar_runtime_boundaries_match_independent_engines_and_preserve_outputs() -> Check<()> {
    let fixture = fixture()?;
    let mut cases = Vec::new();
    for (selector, path) in [
        (0, "direct-branch"),
        (1, "capture-empty-bind-field"),
        (2, "nominal-number"),
        (3, "live-owner-call"),
        (4, "consumed-owner-invoke"),
    ] {
        for value in [i64::MIN, -2, -1, 0, 1, 255, 256, i64::MAX] {
            cases.push((selector, path, value, 0, value));
        }
    }
    // Constructor selection is separate from the numeric payload: zero and -1
    // above must cross the number arm too, not accidentally select Unit/absence.
    cases.extend([
        (2, "nominal-unit", i64::MIN, 1, 0),
        (2, "nominal-none", i64::MAX, 2, -1),
    ]);
    // Runtime flags exercise both representations at the join for each call
    // form; input bytes keep the branch and signed extremes out of folding.
    for (selector, path) in [
        (6, "fallback-call"),
        (7, "fallback-tail"),
        (8, "fallback-invoke"),
        (9, "fallback-tail-invoke"),
    ] {
        for value in [i64::MIN, 0, i64::MAX] {
            for flag in [0, 1] {
                cases.push((selector, path, value, flag, value));
            }
        }
    }
    for (selector, path, value, delta, expected_value) in cases {
        let input = request(selector, value, delta);
        let expected = expected_value.to_le_bytes();
        independent_success(fixture, &input, &expected)?;
        for (emitter, image) in [
            ("Rust", fixture.rust.as_slice()),
            ("maintained", fixture.maintained.as_ref()),
        ] {
            let label = format!("scalar-boundary/{emitter}/{path}/{value}/{delta}");
            let observations = verify_observed(
                &label,
                image,
                &input,
                &expected,
                Mode::AllocationIdentity,
                phase_policy(),
                None,
            )?;
            equal(
                observations.iter().map(|row| row.frame).collect::<Vec<_>>(),
                vec![100, 0, 1, 2, 3, 8, 9, 30, 31, 10, 11],
                "scalar boundary phase inventory",
            )?;
            for row in &observations {
                equal(
                    row.attempt_saturation,
                    [0, 0],
                    "scalar boundary observation saturation",
                )?;
            }
            let last = observations
                .last()
                .ok_or("boundary final observation absent")?;
            equal(last.owned_mapped_bytes, 0, "boundary final owned mappings")?;
            equal(last.output_slabs, [0, 0], "boundary final output slabs")?;
            equal(last.output_bytes, [0, 0], "boundary final output mappings")?;
        }
    }
    Ok(())
}

#[test]
fn native_scalar_runtime_overflow_reclaims_owned_scratch_and_retries() -> Check<()> {
    let fixture = fixture()?;
    let retry = request(5, 40, 2);
    let expected = 42_i64.to_le_bytes();
    independent_success(fixture, &retry, &expected)?;
    for (value, delta) in [(i64::MAX, 1), (i64::MIN, -1)] {
        ensure(
            value.checked_add(delta).is_none(),
            "independent overflow input",
        )?;
        let input = request(5, value, delta);
        let control = ExecutionControl::uncancelled();
        let policy = NormalizedRunPolicy::foreground();
        let arguments = vec![NormalizedValue::bytes(input.as_slice())];
        let vm = NormalizedVm::for_test(&fixture.program, policy)
            .invoke(fixture.entry, arguments.clone(), None, &control)
            .err()
            .ok_or("boundary VM accepted overflow")?;
        let reference =
            NormalizedReferenceInterpreter::new(&fixture.source, &fixture.program, policy)
                .invoke(fixture.entry, arguments, None, &control)
                .err()
                .ok_or("boundary reference accepted overflow")?;
        for (error, expected_code) in [
            (vm, "normalized_integer_overflow"),
            (reference, "reference_integer_overflow"),
        ] {
            equal(
                error.class,
                ExecutionFailureClass::Trap,
                "boundary overflow class",
            )?;
            equal(error.code.as_ref(), expected_code, "boundary overflow code")?;
            ensure(
                !error.possibly_visible,
                "pure overflow reported visible effects",
            )?;
        }
        for (emitter, image) in [
            ("Rust", fixture.rust.as_slice()),
            ("maintained", fixture.maintained.as_ref()),
        ] {
            verify_observed(
                &format!("scalar-boundary/{emitter}/overflow/{value}/{delta}"),
                image,
                &input,
                &expected,
                Mode::RuntimeReject,
                phase_policy(),
                Some((&retry, &expected)),
            )?;
        }
    }
    Ok(())
}
