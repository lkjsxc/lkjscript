//! Ordinary nominal construction and exhaustive dispatch against literal results.
//! Canonical fixtures are publicly authored and strictly loaded before the native
//! emitter runs. Counterfeit prepared-code mutations are confined to guard tests.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use super::build;
use crate::platform::compiler::{OptimizationPolicy, build_clean, link_artifact, load_artifact};
use crate::platform::execution::ExecutionControl;
use crate::platform::execution::normalized::prepare::{
    NormalizedCode, NormalizedFunctionBody, NormalizedInstruction as I, NormalizedVariantJump,
};
use crate::platform::execution::normalized::{
    NormalizedCommandPolicy, NormalizedProgram, run_pure_command,
};
use crate::platform::kernel::{
    DeclarationReference, KernelSnapshot, Name, OwnerKey, OwnerRecord, TypeForm,
};
use crate::platform::publication::GraphRepository;
use std::fs::{self, File};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

type Check<T> = std::result::Result<T, String>;

const SOURCE: &str = r#"declarations.begin
(units
  (module create native-nominal
    (external create add (visibility private) (implementation core.i64.add)
      (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
    (external create sub (visibility private) (implementation core.i64.subtract)
      (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
    (external create eq (visibility private) (implementation core.i64.equal)
      (parameter create a (type I64)) (parameter create b (type I64)) (returns Bool))
    (external create divide (visibility private) (implementation core.i64.divide)
      (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
    (external create empty (visibility private) (implementation core.buffer.empty)
      (returns ByteBuffer))
    (record create Point (visibility private)
      (field create left (type I64)) (field create right (type I64)))
    (record create OtherPoint (visibility private)
      (field create left (type I64)) (field create right (type I64)))
    (variant create Choice (visibility private)
      (case create point (payload Point)) (case create none)
      (case create marker (payload Unit)) (case create number (payload I64)))
    (variant create OtherChoice (visibility private)
      (case create point (payload Point)) (case create none)
      (case create marker (payload Unit)) (case create number (payload I64)))
    (variant create Outer (visibility private)
      (case create present (payload Choice)) (case create absent))
    (function create make-point (visibility private) (effect pure) (returns Choice)
      (body (variant Choice::point
        (record Point (field Point::right (i64 49)) (field Point::left (i64 7))))))
    (function create make-none (visibility private) (effect pure) (returns Choice)
      (body (variant Choice::none)))
    (function create make-other (visibility private) (effect pure) (returns OtherChoice)
      (body (variant OtherChoice::point
        (record Point (field Point::right (i64 49)) (field Point::left (i64 7))))))
    (function create choose (visibility private) (effect pure)
      (parameter create selector (type I64)) (returns Choice)
      (body (if (call eq (local selector) (i64 0)) (call make-point)
        (if (call eq (local selector) (i64 1)) (call make-none)
          (if (call eq (local selector) (i64 2))
            (variant Choice::marker (unit)) (variant Choice::number (i64 -17)))))))
    (function create inspect (visibility private) (effect pure)
      (parameter create choice (type Choice)) (returns I64)
      (body (match (local choice)
        (arm Choice::point (payload p Point)
          (call sub (field (local p) Point::right) (field (local p) Point::left)))
        (arm Choice::none (i64 7))
        (arm Choice::marker (payload u Unit) (sequence (local u) (i64 11)))
        (arm Choice::number (payload n I64) (local n)))))
    (function create nested (visibility private) (effect pure) (returns I64)
      (body (call inspect
        (match (variant Outer::present (call make-point))
          (arm Outer::present (payload value Choice) (local value))
          (arm Outer::absent (call make-none))))))
    (function create twice (visibility private) (effect pure) (returns I64)
      (body (let (binding value (type Choice) (call make-point))
        (in (call add (call inspect (local value)) (call inspect (local value)))))))
    (function create selected-only (visibility private) (effect pure) (returns I64)
      (body (match (call make-none)
        (arm Choice::point (payload p Point) (call divide (i64 1) (i64 0)))
        (arm Choice::none (i64 7))
        (arm Choice::marker (payload u Unit) (call divide (i64 1) (i64 0)))
        (arm Choice::number (payload n I64) (call divide (i64 1) (i64 0))))))
    (function create result (visibility private) (effect pure)
      (parameter create value (type I64)) (returns I64) (body (local value)))
    (function create live-buffer (visibility private) (effect pure) (returns I64)
      (body (let (binding scratch (type ByteBuffer) (call empty))
        (in (match (call make-point)
          (arm Choice::point (payload p Point)
            (call result (call sub (field (local p) Point::right) (field (local p) Point::left))))
          (arm Choice::none (call result (i64 7)))
          (arm Choice::marker (payload u Unit)
            (sequence (local u) (call result (i64 11))))
          (arm Choice::number (payload n I64) (call result (local n))))))))
    (function create right-division (visibility private) (effect pure) (returns I64)
      (body (call inspect (variant Choice::point
        (record Point
          (field Point::right (call divide (i64 1) (i64 0)))
          (field Point::left (call add (i64 9223372036854775807) (i64 1))))))))
    (function create left-overflow (visibility private) (effect pure) (returns I64)
      (body (call inspect (variant Choice::point
        (record Point
          (field Point::left (call add (i64 9223372036854775807) (i64 1)))
          (field Point::right (call divide (i64 1) (i64 0))))))))
    (function create main (visibility public) (effect pure)
      (parameter create selector (type I64)) (returns I64)
      (body (if (call eq (local selector) (i64 4)) (call nested)
        (if (call eq (local selector) (i64 5)) (call twice)
          (if (call eq (local selector) (i64 6)) (call selected-only)
            (if (call eq (local selector) (i64 7)) (call live-buffer)
              (call inspect (call choose (local selector)))))))))
    (component create app (visibility private)
      (port create run (type (function (I64) I64)) (function main)))
    (component create app-right-division (visibility private)
      (port create run (type (function () I64)) (function right-division)))
    (component create app-left-overflow (visibility private)
      (port create run (type (function () I64)) (function left-overflow))))
  (target create nominal (component native-nominal::app)
    (runner command) (port native-nominal::app::run))
  (target create nominal-right-division (component native-nominal::app-right-division)
    (runner command) (port native-nominal::app-right-division::run))
  (target create nominal-left-overflow (component native-nominal::app-left-overflow)
    (runner command) (port native-nominal::app-left-overflow::run)))
declarations.end"#;

// A real resource-bearing nominal layout, but no resource is acquired. The
// selected ordinary value is discarded before the scalar command returns.
const RESOURCE_SOURCE: &str = r#"declarations.begin
(units
  (module create native-resource
    (interface create Handle (visibility private)
      (operation create noop (returns Unit)
        (idempotency idempotent) (external-visibility none)))
    (variant create Envelope (visibility private)
      (case create ordinary (payload I64))
      (case create held (payload (resource Handle))))
    (function create main (visibility public) (effect pure) (returns I64)
      (body (sequence (variant Envelope::ordinary (i64 41)) (i64 42))))
    (component create app (visibility private)
      (port create run (type (function () I64)) (function main))))
  (target create resource-layout (component native-resource::app)
    (runner command) (port native-resource::app::run)))
declarations.end"#;

struct Fixture {
    source: KernelSnapshot,
    repository: GraphRepository,
    program: Arc<NormalizedProgram>,
}

fn prepare_fixture(input: &str) -> Check<Fixture> {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(input)?;
    let memory = crate::platform::MemProjectStore::new(None)
        .map_err(|error| format!("native nominal store: {error:?}"))?;
    let repository = GraphRepository::create_memory(&memory, &source, None)
        .map_err(|error| format!("native nominal repository: {error:?}"))?
        .repository;
    let compiled = build_clean(&repository, OptimizationPolicy::DeterministicBaseline)
        .map_err(|error| format!("native nominal compilation: {error:?}"))?;
    let artifact = link_artifact(&repository, compiled.manifest_digest, &[])
        .map_err(|error| format!("native nominal artifact: {error:?}"))?;
    let loaded = load_artifact(&artifact.artifact.bytes)
        .map_err(|error| format!("native nominal strict load: {error:?}"))?;
    let program = Arc::new(
        NormalizedProgram::prepare(loaded)
            .map_err(|error| format!("native nominal preparation: {error:?}"))?,
    );
    Ok(Fixture {
        source,
        repository,
        program,
    })
}

fn function_index(
    source: &KernelSnapshot,
    program: &NormalizedProgram,
    name: &str,
) -> Check<usize> {
    let declaration = source
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.name.as_str() == name =>
            {
                Some(*id)
            }
            _ => None,
        })
        .ok_or_else(|| format!("fixture declaration {name} is absent"))?;
    let reference = DeclarationReference {
        package: source.root.package_id,
        declaration,
    };
    let index = program
        .function_by_declaration
        .get(&reference)
        .ok_or_else(|| format!("prepared fixture declaration {name} is absent"))?;
    usize::try_from(index.0).map_err(|_| "fixture function index is not addressable".to_owned())
}

fn code_mut<'a>(
    source: &KernelSnapshot,
    program: &'a mut NormalizedProgram,
    name: &str,
) -> Check<&'a mut NormalizedCode> {
    let index = function_index(source, program, name)?;
    let function = Arc::make_mut(&mut program.functions)
        .get_mut(index)
        .ok_or_else(|| format!("prepared fixture function {name} is absent"))?;
    match &mut function.body {
        NormalizedFunctionBody::Code(code) => Ok(code),
        _ => Err(format!("fixture function {name} has no graph code")),
    }
}

fn variant_instruction(
    source: &KernelSnapshot,
    program: &NormalizedProgram,
    name: &str,
) -> Check<I> {
    let function = program
        .functions
        .get(function_index(source, program, name)?)
        .ok_or_else(|| format!("prepared fixture function {name} is absent"))?;
    let NormalizedFunctionBody::Code(code) = &function.body else {
        return Err(format!("fixture function {name} has no graph code"));
    };
    code.instructions
        .iter()
        .find(|instruction| matches!(instruction, I::Variant { .. }))
        .cloned()
        .ok_or_else(|| format!("fixture function {name} has no Variant"))
}

fn first_variant_mut(code: &mut NormalizedCode) -> Check<&mut I> {
    Arc::make_mut(&mut code.instructions)
        .iter_mut()
        .find(|instruction| matches!(instruction, I::Variant { .. }))
        .ok_or_else(|| "fixture code has no Variant".to_owned())
}

fn first_switch_mut(code: &mut NormalizedCode) -> Check<&mut Arc<[NormalizedVariantJump]>> {
    Arc::make_mut(&mut code.instructions)
        .iter_mut()
        .find_map(|instruction| match instruction {
            I::SwitchVariant(jumps) => Some(jumps),
            _ => None,
        })
        .ok_or_else(|| "fixture code has no SwitchVariant".to_owned())
}

fn reject_native(program: &NormalizedProgram, target: &str, fault: &str) -> Check<()> {
    match build(program, Some(target)) {
        Ok(_) => Err(format!("native emitter accepted {fault}")),
        Err(error) => {
            assert_eq!(error.code, "native_elf_unsupported", "{fault}: {error:?}");
            Ok(())
        }
    }
}

struct Joined(Child);
impl Drop for Joined {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct NativeOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run_image(image: &[u8], arguments: &[&str]) -> Check<NativeOutput> {
    let temporary = tempfile::tempdir().map_err(|error| error.to_string())?;
    let binary = temporary.path().join("nominal");
    let stdout = temporary.path().join("stdout");
    let stderr = temporary.path().join("stderr");
    crate::platform::owned_output::publish_executable_create_new(
        &binary,
        image,
        None,
        "native nominal control",
    )
    .map_err(|error| format!("native nominal publication: {error:?}"))?;
    let mut child = Joined(
        Command::new("/usr/bin/prlimit")
            .env_clear()
            .args(["--stack=1048576", "--core=0", "--fsize=4096", "--"])
            .arg(&binary)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(File::create(&stdout).map_err(|error| error.to_string())?)
            .stderr(File::create(&stderr).map_err(|error| error.to_string())?)
            .spawn()
            .map_err(|error| error.to_string())?,
    );
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait().map_err(|error| error.to_string())? {
            break status;
        }
        if started.elapsed() >= Duration::from_secs(5) {
            return Err("native nominal control exceeded its explicit test deadline".to_owned());
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    Ok(NativeOutput {
        status,
        stdout: fs::read(stdout).map_err(|error| error.to_string())?,
        stderr: fs::read(stderr).map_err(|error| error.to_string())?,
    })
}

#[test]
fn native_elf_nominal_variants_match_literal_results_and_reference() -> Check<()> {
    let fixture = prepare_fixture(SOURCE)?;
    let view = fixture
        .repository
        .view_current()
        .map_err(|error| format!("view: {error:?}"))?;
    let target = Name::new("nominal").map_err(|error| format!("target: {error:?}"))?;
    let image = build(&fixture.program, Some("nominal"))
        .map_err(|error| format!("native nominal build: {error:?}"))?;
    // Constants encode independent source expectations: swapped Point fields
    // would give -42, None and Some(Unit) are 7 and 11, and copying yields 84.
    // Selector 7 keeps a ByteBuffer live around a tail-position Match arm call;
    // the native owner guard must preserve its ordinary cleanup continuation.
    for (selector, expected) in [
        (0, 42),
        (1, 7),
        (2, 11),
        (3, -17),
        (4, 42),
        (5, 84),
        (6, 7),
        (7, 42),
    ] {
        let arguments = format!("[{selector}]");
        let expected = expected.to_string().into_bytes();
        let oracle = run_pure_command(
            &view,
            &fixture.program,
            &target,
            arguments.as_bytes(),
            NormalizedCommandPolicy::default(),
            &ExecutionControl::uncancelled(),
        )
        .map_err(|error| format!("nominal oracle {selector}: {error:?}"))?;
        assert_eq!(oracle.differential, "equal");
        assert_eq!(oracle.result_json, expected, "nominal oracle {selector}");
        assert_eq!(oracle.production.live_call_frames_after, 0);
        assert_eq!(oracle.production.live_operands_after, 0);
        assert_eq!(oracle.reference.live_call_frames_after, 0);
        let argument = selector.to_string();
        let actual = run_image(&image.bytes, &[&argument])?;
        assert_eq!(
            actual.status.code(),
            Some(0),
            "selector {selector}: {:?}",
            actual.stderr
        );
        assert_eq!(actual.stdout, expected, "native selector {selector}");
        assert!(
            actual.stderr.is_empty(),
            "native selector {selector}: {:?}",
            actual.stderr
        );
    }
    Ok(())
}

fn accepted_constructor_order(source: &KernelSnapshot, name: &str) -> Check<(Vec<String>, bool)> {
    use crate::platform::kernel::{
        BindingKind, DeclarationPayload, ExpressionOperation as E, FieldSelector,
        LocalValueReference,
    };

    let body = source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Declaration(record) if record.name.as_str() == name => {
                match &record.payload {
                    DeclarationPayload::Function(function) => Some(function.body),
                    _ => None,
                }
            }
            _ => None,
        })
        .ok_or_else(|| format!("canonical fixture function {name} is absent"))?;
    let operation = |id| match source.owners.get(&OwnerKey::Expression(id)) {
        Some(OwnerRecord::Expression(record)) => Ok(&record.operation),
        _ => Err(format!("canonical fixture expression {id} is absent")),
    };
    let E::Call { arguments, .. } = operation(body)? else {
        return Err(format!("{name} does not call inspect"));
    };
    if arguments.len() != 1 {
        return Err(format!("{name} inspect arity differs"));
    }
    let E::Variant {
        payload: Some(payload),
        ..
    } = operation(arguments[0])?
    else {
        return Err(format!("{name} lacks its variant payload"));
    };
    // Public authoring retains initializer order with lexical temporaries when
    // the authored selector order differs from the canonical layout order.
    let (bindings, record) = match operation(*payload)? {
        E::Let { bindings, body } => (Some(bindings), *body),
        _ => (None, *payload),
    };
    let E::Record {
        nominal_type: Some(_),
        fields,
        ..
    } = operation(record)?
    else {
        return Err(format!("{name} lacks its nominal record"));
    };
    if fields.len() != 2
        || fields
            .windows(2)
            .any(|pair| pair[0].selector >= pair[1].selector)
    {
        return Err(format!(
            "{name} record is not in exact canonical selector order"
        ));
    }
    let mut names = Vec::with_capacity(fields.len());
    for field in fields {
        let FieldSelector::Nominal(reference) = &field.selector else {
            return Err(format!("{name} selector is structural"));
        };
        if reference.package != source.root.package_id {
            return Err(format!("{name} field is foreign"));
        }
        match source.owners.get(&OwnerKey::Field(reference.field)) {
            Some(OwnerRecord::Field(record)) => names.push(record.name.as_str().to_owned()),
            _ => return Err(format!("{name} field is absent")),
        }
    }
    let Some(bindings) = bindings else {
        return Ok((names, false));
    };
    if bindings.len() != 2 || bindings[0] == bindings[1] {
        return Err(format!("{name} needs two distinct initializer bindings"));
    }
    let mut ordered = Vec::with_capacity(bindings.len());
    for binding in bindings {
        match source.owners.get(&OwnerKey::Binding(*binding)) {
            Some(OwnerRecord::Binding(record))
                if record.kind == BindingKind::Let && record.value.is_some() => {}
            _ => return Err(format!("{name} initializer binding is absent or not a Let")),
        }
        let field = fields
            .iter()
            .position(|field| {
                matches!(operation(field.value),
                    Ok(E::Local { value: LocalValueReference::LexicalBinding(retained) })
                        if retained == binding
                )
            })
            .ok_or_else(|| format!("{name} record does not read its retained initializer"))?;
        ordered.push(names[field].clone());
    }
    Ok((ordered, true))
}

#[test]
fn native_elf_nominal_record_construction_keeps_canonical_trap_order() -> Check<()> {
    let fixture = prepare_fixture(SOURCE)?;
    let view = fixture
        .repository
        .view_current()
        .map_err(|error| format!("view: {error:?}"))?;
    let accepted = view
        .reconstruct_full_oracle()
        .map_err(|error| format!("canonical fixture readback: {error:?}"))?
        .value;
    let mut wrapped = 0;
    for (target, function, order, expected) in [
        (
            "nominal-right-division",
            "right-division",
            ["right", "left"],
            "normalized_integer_division",
        ),
        (
            "nominal-left-overflow",
            "left-overflow",
            ["left", "right"],
            "normalized_integer_overflow",
        ),
    ] {
        // Both functions attach division to right and overflow to left, but
        // author the initializers in opposite orders. Literal trap expectations
        // are independent of semantic-ID ordering and the generated code.
        let (accepted_order, temporaries) = accepted_constructor_order(&accepted, function)?;
        assert_eq!(
            accepted_order, order,
            "{target}: accepted initializer order"
        );
        wrapped += usize::from(temporaries);
        let name = Name::new(target).map_err(|error| format!("target: {error:?}"))?;
        // The command runner stops on the VM trap, so this is deliberately a
        // accepted-order + literal + VM + native control, not a reference-trap claim.
        let oracle = run_pure_command(
            &view,
            &fixture.program,
            &name,
            b"[]",
            NormalizedCommandPolicy::default(),
            &ExecutionControl::uncancelled(),
        );
        let error = match oracle {
            Ok(_) => return Err(format!("expected canonical-order trap in {target}")),
            Err(error) => error,
        };
        assert_eq!(error.code, expected, "{target}: {error:?}");
        let image = build(&fixture.program, Some(target))
            .map_err(|error| format!("native trap build {target}: {error:?}"))?;
        let actual = run_image(&image.bytes, &[])?;
        assert_eq!(actual.status.code(), Some(70), "{target}");
        assert!(actual.stdout.is_empty(), "{target}");
        assert_eq!(
            actual.stderr,
            format!("{expected}\n").as_bytes(),
            "{target}"
        );
    }
    assert_eq!(
        wrapped, 1,
        "one opposite selector order must retain initializer temporaries"
    );
    Ok(())
}

#[test]
fn native_elf_nominal_variant_rejects_unselected_noncopyable_case() -> Check<()> {
    let fixture = prepare_fixture(RESOURCE_SOURCE)?;
    let _ = variant_instruction(&fixture.source, &fixture.program, "main")?;
    let view = fixture
        .repository
        .view_current()
        .map_err(|error| format!("view: {error:?}"))?;
    let target = Name::new("resource-layout").map_err(|error| format!("target: {error:?}"))?;
    let oracle = run_pure_command(
        &view,
        &fixture.program,
        &target,
        b"[]",
        NormalizedCommandPolicy::default(),
        &ExecutionControl::uncancelled(),
    )
    .map_err(|error| format!("unselected resource-case oracle: {error:?}"))?;
    assert_eq!(oracle.differential, "equal");
    assert_eq!(oracle.result_json, b"42");
    reject_native(
        &fixture.program,
        "resource-layout",
        "unselected resource case",
    )?;

    let fixture = prepare_fixture(SOURCE)?;
    let mut program = fixture.program.as_ref().clone();
    let buffer = program
        .types
        .iter()
        .find_map(|(ty, object)| matches!(object.form, TypeForm::ByteBuffer).then_some(*ty))
        .ok_or_else(|| "fixture ByteBuffer type is absent".to_owned())?;
    let I::Variant { layout, .. } = variant_instruction(&fixture.source, &program, "make-point")?
    else {
        return Err("fixture constructor is not Variant".to_owned());
    };
    let layout_index = usize::try_from(layout.0)
        .map_err(|_| "fixture layout index is not addressable".to_owned())?;
    let layout = Arc::make_mut(&mut program.variants)
        .get_mut(layout_index)
        .ok_or_else(|| "fixture layout is absent".to_owned())?;
    let case = Arc::make_mut(&mut layout.cases)
        .iter_mut()
        .find(|case| case.name.as_str() == "number")
        .ok_or_else(|| "fixture number case is absent".to_owned())?;
    // Deliberately corrupt derived metadata after strict load. This is malformed
    // containment, not evidence that canonical ByteBuffer variants are accepted.
    case.payload = Some(buffer);
    reject_native(&program, "nominal", "malformed ByteBuffer case containment")
}

#[test]
fn native_elf_nominal_fixtures_require_exact_canonical_payloads() -> Check<()> {
    let foreign_record = SOURCE.replacen(
        "(record Point (field Point::right (i64 49)) (field Point::left (i64 7)))",
        "(record OtherPoint (field OtherPoint::right (i64 49)) (field OtherPoint::left (i64 7)))",
        1,
    );
    let foreign_binder = SOURCE.replacen(
        "(arm Choice::point (payload p Point)",
        "(arm Choice::point (payload p OtherPoint)",
        1,
    );
    assert_ne!(
        foreign_record, SOURCE,
        "record payload mutation must change the fixture"
    );
    assert_ne!(
        foreign_binder, SOURCE,
        "payload binder mutation must change the fixture"
    );
    for (source, code) in [
        (foreign_record.as_str(), "kernel_type_payload"),
        (foreign_binder.as_str(), "kernel_type_match_binding"),
    ] {
        let error =
            match crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
                source,
            ) {
                Ok(_) => return Err(format!("canonical authoring accepted {code}")),
                Err(error) => error,
            };
        assert!(error.contains(code), "{code}: {error}");
    }
    Ok(())
}

#[test]
fn native_elf_nominal_variant_rejects_constructor_shape_and_foreign_origin() -> Check<()> {
    let fixture = prepare_fixture(SOURCE)?;
    for (function, fault) in [
        ("make-point", "omitted payload"),
        ("make-none", "invented payload"),
    ] {
        let mut program = fixture.program.as_ref().clone();
        let I::Variant { has_payload, .. } =
            first_variant_mut(code_mut(&fixture.source, &mut program, function)?)?
        else {
            return Err("selected constructor is not Variant".to_owned());
        };
        *has_payload = !*has_payload;
        reject_native(&program, "nominal", fault)?;
    }
    let mut program = fixture.program.as_ref().clone();
    let I::Variant { case, .. } =
        first_variant_mut(code_mut(&fixture.source, &mut program, "make-point")?)?
    else {
        return Err("selected constructor is not Variant".to_owned());
    };
    *case = u32::MAX;
    reject_native(&program, "nominal", "constructor case outside its layout")?;

    let mut program = fixture.program.as_ref().clone();
    let i64_type = program
        .types
        .iter()
        .find_map(|(ty, object)| matches!(object.form, TypeForm::I64).then_some(*ty))
        .ok_or_else(|| "fixture I64 type is absent".to_owned())?;
    let I::Variant { type_arguments, .. } =
        first_variant_mut(code_mut(&fixture.source, &mut program, "make-point")?)?
    else {
        return Err("selected constructor is not Variant".to_owned());
    };
    *type_arguments = Arc::from([i64_type]);
    reject_native(
        &program,
        "nominal",
        "closed constructor with a type argument",
    )?;

    let independently_prepared = prepare_fixture(SOURCE)?;
    let foreign = variant_instruction(
        &independently_prepared.source,
        &independently_prepared.program,
        "make-point",
    )?;
    let mut program = fixture.program.as_ref().clone();
    *first_variant_mut(code_mut(&fixture.source, &mut program, "make-point")?)? = foreign;
    reject_native(&program, "nominal", "foreign prepared origin")
}

#[test]
fn native_elf_nominal_match_rejects_case_binder_and_target_corruption() -> Check<()> {
    let fixture = prepare_fixture(SOURCE)?;
    let I::Variant {
        layout: other_layout,
        ..
    } = variant_instruction(&fixture.source, &fixture.program, "make-other")?
    else {
        return Err("other constructor is not Variant".to_owned());
    };
    for fault in [
        "missing-case",
        "duplicate-case",
        "reordered-cases",
        "foreign-layout",
        "missing-binder",
        "invented-binder",
        "parameter-binder",
        "out-of-frame-binder",
        "duplicate-binder",
        "backward-target",
        "out-of-code-target",
        "duplicate-target",
    ] {
        let mut program = fixture.program.as_ref().clone();
        let code = code_mut(&fixture.source, &mut program, "inspect")?;
        let local_count = code.local_count;
        let instruction_count = u32::try_from(code.instructions.len())
            .map_err(|_| "fixture instruction count exceeds u32".to_owned())?;
        let jumps = first_switch_mut(code)?;
        if fault == "missing-case" {
            let count = jumps
                .len()
                .checked_sub(1)
                .ok_or_else(|| "empty fixture switch".to_owned())?;
            *jumps = jumps.iter().take(count).cloned().collect::<Vec<_>>().into();
        } else {
            let rows = Arc::make_mut(jumps);
            let payload = rows
                .iter()
                .position(|jump| jump.binding_local.is_some())
                .ok_or_else(|| "fixture switch has no payload arm".to_owned())?;
            let absent = rows
                .iter()
                .position(|jump| jump.binding_local.is_none())
                .ok_or_else(|| "fixture switch has no payloadless arm".to_owned())?;
            match fault {
                "duplicate-case" => rows[1].case = rows[0].case,
                "reordered-cases" => rows.swap(0, 1),
                "foreign-layout" => rows[0].layout = other_layout,
                "missing-binder" => rows[payload].binding_local = None,
                "invented-binder" => rows[absent].binding_local = Some(1),
                "parameter-binder" => rows[payload].binding_local = Some(0),
                "out-of-frame-binder" => rows[payload].binding_local = Some(local_count),
                "duplicate-binder" => {
                    let other = rows
                        .iter()
                        .enumerate()
                        .find_map(|(index, jump)| {
                            (index != payload && jump.binding_local.is_some()).then_some(index)
                        })
                        .ok_or_else(|| "fixture switch needs two payload binders".to_owned())?;
                    rows[other].binding_local = rows[payload].binding_local;
                }
                "backward-target" => rows[0].target = 0,
                "out-of-code-target" => rows[0].target = instruction_count,
                "duplicate-target" => rows[1].target = rows[0].target,
                _ => return Err(format!("unknown test mutation {fault}")),
            }
        }
        reject_native(&program, "nominal", fault)?;
    }
    Ok(())
}

#[test]
fn native_elf_nominal_match_checks_runtime_nominal_identity() -> Check<()> {
    let fixture = prepare_fixture(SOURCE)?;
    let replacement = variant_instruction(&fixture.source, &fixture.program, "make-other")?;
    let mut program = fixture.program.as_ref().clone();
    // Keep both layouts and instructions individually valid. This deliberately
    // counterfeits the derived return value after canonical admission. It is not
    // a source-language fixture or a claim that the emitter rechecks all typing.
    *first_variant_mut(code_mut(&fixture.source, &mut program, "make-point")?)? = replacement;
    let image = build(&program, Some("nominal"))
        .map_err(|error| format!("counterfeit nominal build: {error:?}"))?;
    let actual = run_image(&image.bytes, &["0"])?;
    assert_eq!(actual.status.code(), Some(70));
    assert!(actual.stdout.is_empty());
    assert_eq!(actual.stderr, b"native_variant_shape\n");
    Ok(())
}

// A nonconstructing owner result can pass host semantic admission without an
// owned-result port. These recursive bodies must NEVER be executed. Their
// prepared code contains only a self call and Return, so an unsupported owner
// constructor cannot accidentally supply the expected native rejection.
const RESULT_CATEGORY_SOURCE: &str = r#"declarations.begin
(units
  (module create native-result-category
    (owned-record create OwnedEmpty (visibility private))
    (owned-record create OwnedApplied (visibility private)
      (type-parameter create T (constraint owned)))
    (record create Ordinary (visibility private)
      (field create value (type I64)))
    (record create OrdinaryApplied (visibility private)
      (type-parameter create T)
      (field create value (type T)))
    (function create spin-owned (visibility private) (effect pure)
      (returns OwnedEmpty) (body (call spin-owned)))
    (function create spin-owned-applied (visibility private) (effect pure)
      (returns (OwnedApplied ByteBuffer)) (body (call spin-owned-applied)))
    (function create spin-ordinary (visibility private) (effect pure)
      (returns Ordinary) (body (call spin-ordinary)))
    (function create spin-ordinary-applied (visibility private) (effect pure)
      (returns (OrdinaryApplied I64)) (body (call spin-ordinary-applied)))
    (function create main (visibility public) (effect pure)
      (returns I64) (body (i64 42)))
    (component create app (visibility private)
      (port create run (type (function () I64)) (function main))))
  (target create ordinary-root (component native-result-category::app)
    (runner command) (port native-result-category::app::run)))
declarations.end"#;

#[test]
fn native_elf_nominal_result_gate_rejects_recursive_owned_helpers() -> Check<()> {
    let fixture = prepare_fixture(RESULT_CATEGORY_SOURCE)?;
    for (name, applied, owned) in [
        ("spin-owned", false, true),
        ("spin-owned-applied", true, true),
        ("spin-ordinary", false, false),
        ("spin-ordinary-applied", true, false),
    ] {
        let index = function_index(&fixture.source, &fixture.program, name)?;
        let function = fixture
            .program
            .functions
            .get(index)
            .ok_or_else(|| format!("result-category helper {name} is absent"))?;
        assert!(function.type_parameters.is_empty(), "closed helper {name}");
        assert_eq!(function.parameter_count, 0, "helper {name}");
        let NormalizedFunctionBody::Code(code) = &function.body else {
            return Err(format!("result-category helper {name} has no graph code"));
        };
        assert_eq!(code.parameter_count, 0, "helper {name}");
        assert_eq!(
            code.local_count, 0,
            "no owned local supplies the rejection: {name}"
        );
        let target = u32::try_from(index)
            .map_err(|_| "result-category function index exceeds u32".to_owned())?;
        assert!(
            code.instructions
                .iter()
                .any(|instruction| matches!(instruction,
                    I::Call { function, .. } | I::TailCall { function, .. } if function.0 == target
                )),
            "helper {name} must be self-recursive"
        );
        assert!(
            code.instructions
                .iter()
                .all(|instruction| match instruction {
                    I::Call { function, .. } | I::TailCall { function, .. } => function.0 == target,
                    I::Return => true,
                    _ => false,
                }),
            "helper {name} must contain no constructor or ownership operation"
        );
        let object = fixture
            .program
            .types
            .get(&function.result.ty)
            .ok_or_else(|| format!("result-category type for {name} is absent"))?;
        let (declaration, arguments) = match &object.form {
            TypeForm::Named { declaration } if !applied => (declaration, &[][..]),
            TypeForm::Applied {
                declaration,
                arguments,
            } if applied => {
                assert_eq!(arguments.len(), 1, "applied helper {name}");
                (declaration, arguments.as_slice())
            }
            _ => {
                return Err(format!(
                    "helper {name} did not retain its intended type form"
                ));
            }
        };
        let record = fixture
            .program
            .records
            .iter()
            .find(|record| {
                record.declaration == *declaration && record.arguments.as_ref() == arguments
            })
            .ok_or_else(|| format!("result-category layout for {name} is absent"))?;
        assert_eq!(record.owned, owned, "exact result category for {name}");
        if owned {
            assert!(record.fields.is_empty(), "empty owner helper {name}");
            let error = match super::compile_function(&fixture.program, index) {
                Ok(_) => return Err(format!("native compiler accepted owned result {name}")),
                Err(error) => error,
            };
            assert_eq!(error.code, "native_elf_unsupported", "{name}: {error:?}");
            assert_eq!(
                error.message,
                "native result must not contain OwnedRecord values"
            );
        } else {
            // Preserve the previously supported ordinary Named/Applied result
            // predicate. Compile only: neither nonterminating image is run.
            let (image, _, _) = super::compile_function(&fixture.program, index)
                .map_err(|error| format!("ordinary result {name}: {error:?}"))?;
            assert!(image.starts_with(b"\x7fELF"), "ordinary result {name}");
        }
    }
    Ok(())
}

#[test]
fn native_elf_nominal_owned_result_ports_reject_during_authoring() -> Check<()> {
    // This is the independent public graph-admission boundary, not a backend
    // rejection. Keep the same helpers and alter only the exact port signature
    // and its backing function; no invalid source is passed to native emission.
    let baseline = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        RESULT_CATEGORY_SOURCE,
    )?;
    crate::platform::kernel::validate_full(&baseline)
        .map_err(|error| format!("owned-result port baseline must be fully valid: {error:?}"))?;
    let ordinary = "(port create run (type (function () I64)) (function main))";
    assert_eq!(RESULT_CATEGORY_SOURCE.matches(ordinary).count(), 1);
    for owned in [
        "(port create run (type (function () OwnedEmpty)) (function spin-owned))",
        "(port create run (type (function () (OwnedApplied ByteBuffer))) (function spin-owned-applied))",
    ] {
        let source = RESULT_CATEGORY_SOURCE.replacen(ordinary, owned, 1);
        let error =
            match crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
                &source,
            ) {
                Ok(_) => return Err("public authoring accepted an owned-result port".to_owned()),
                Err(error) => error,
            };
        assert!(error.contains("kernel_buffer_ownership"), "{error}");
        assert!(
            error.contains("buffer adapter port is forbidden"),
            "{error}"
        );
    }
    Ok(())
}
