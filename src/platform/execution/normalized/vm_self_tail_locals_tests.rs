//! Actual local-buffer reuse, admission/cancellation, and conservative fallbacks.
use super::*;
use crate::platform::execution::normalized::{owned_i64_cell::OwnedI64Cell, value::ValueOrigin};
use crate::platform::kernel::{OwnerKey, OwnerRecord};
use crate::platform::publication::GraphRepository;
use std::cell::Cell;
use std::mem::size_of;

type TestResult<T = ()> = Result<T, ExecutionError>;

thread_local! {
    static FRESH_LOCALS: Cell<bool> = const { Cell::new(false) };
    static FRESH_CALL_ARGUMENTS: Cell<bool> = const { Cell::new(false) };
    static CALL_ARGUMENT_STORAGE: Cell<(u64, u64)> = const { Cell::new((0, 0)) };
}

pub(super) fn force_fresh_call_arguments() -> bool {
    FRESH_CALL_ARGUMENTS.with(Cell::get)
}

pub(super) fn observe_call_arguments(reused: bool, count: usize) {
    if count != 0 {
        CALL_ARGUMENT_STORAGE.with(|storage| {
            let (fresh, reuse) = storage.get();
            storage.set((fresh + u64::from(!reused), reuse + u64::from(reused)));
        });
    }
}

fn with_call_arguments<T>(fresh: bool, run: impl FnOnce() -> T) -> (T, (u64, u64)) {
    struct Restore(bool, (u64, u64));
    impl Drop for Restore {
        fn drop(&mut self) {
            FRESH_CALL_ARGUMENTS.with(|flag| flag.set(self.0));
            CALL_ARGUMENT_STORAGE.with(|storage| storage.set(self.1));
        }
    }
    let restore = Restore(
        FRESH_CALL_ARGUMENTS.with(|flag| flag.replace(fresh)),
        CALL_ARGUMENT_STORAGE.with(|storage| storage.replace((0, 0))),
    );
    let result = run();
    let storage = CALL_ARGUMENT_STORAGE.with(Cell::get);
    drop(restore);
    (result, storage)
}

pub(super) fn force_fresh_locals() -> bool {
    FRESH_LOCALS.with(Cell::get)
}

fn with_fresh_locals<T>(run: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            FRESH_LOCALS.with(|flag| flag.set(self.0));
        }
    }
    let restore = Restore(FRESH_LOCALS.with(|flag| flag.replace(true)));
    let result = run();
    drop(restore);
    result
}

fn failure(message: impl std::fmt::Display) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Trap,
        "self_tail_locals_test",
        message.to_string(),
    )
}

fn require(condition: bool, message: &str) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(failure(message))
    }
}

const SOURCE: &str = r#"declarations.begin
(units (module create reuse
  (external create less (visibility private) (implementation core.i64.less)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns Bool))
  (external create subtract (visibility private) (implementation core.i64.subtract)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (external create add (visibility private) (implementation core.i64.add)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create finish-cell (visibility private) (implementation core.cell.extract)
    (parameter create cell (type OwnedI64Cell) (use consume)) (returns I64))
  (function create count (visibility public) (effect pure)
    (parameter create n (type I64)) (returns I64)
    (body (if (call less (local n) (i64 1)) (i64 0)
      (call count (call subtract (local n) (i64 1))))))
  (function create even (visibility public) (effect pure)
    (parameter create n (type I64)) (returns Bool)
    (body (if (call less (local n) (i64 1)) (bool true)
      (call odd (call subtract (local n) (i64 1))))))
  (function create odd (visibility private) (effect pure)
    (parameter create n (type I64)) (returns Bool)
    (body (if (call less (local n) (i64 1)) (bool false)
      (call even (call subtract (local n) (i64 1))))))
  (function create step (visibility private) (effect pure)
    (parameter create n (type I64)) (returns I64)
    (body (call add (local n) (i64 5))))
  (function create callback-loop (visibility public) (effect pure)
    (parameter create n (type I64))
    (parameter create callback (type (function (I64) I64)))
    (parameter create seed (type I64)) (returns I64)
    (body (if (call less (local n) (i64 1))
      (invoke (local callback) (local seed))
      (call callback-loop (call subtract (local n) (i64 1))
        (local callback) (local seed)))))
  (function create slots (visibility public) (effect pure)
    (parameter create n (type I64))
    (parameter create callback (type (function (I64) I64))) (returns I64)
    (body (if (call less (local n) (i64 1))
      (let (binding dirty (type Text) (text "old branch"))
        (in (sequence (local dirty) (invoke (local callback) (i64 7)))))
      (call slots (call subtract (local n) (i64 1)) (local callback)))))
  (function create owned-result (visibility public) (effect pure)
    (parameter create n (type I64)) (returns OwnedI64Cell)
    (body (if (call less (local n) (i64 1)) (call new-cell (i64 137))
      (call owned-result (call subtract (local n) (i64 1))))))
  (function create owned-main (visibility public) (effect pure)
    (parameter create n (type I64)) (returns I64)
    (body (let (binding cell (type OwnedI64Cell) (call owned-result (local n)))
      (in (call finish-cell (local cell))))))))
declarations.end"#;

type SharedFixture = (
    Arc<NormalizedProgram>,
    BTreeMap<String, FunctionIndex>,
    Arc<crate::platform::kernel::KernelSnapshot>,
);

fn shared_fixture() -> TestResult<&'static SharedFixture> {
    static FIXTURE: std::sync::OnceLock<TestResult<SharedFixture>> = std::sync::OnceLock::new();
    match FIXTURE.get_or_init(build_fixture) {
        Ok(fixture) => Ok(fixture),
        Err(error) => Err(error.clone()),
    }
}

fn fixture() -> TestResult<(Arc<NormalizedProgram>, BTreeMap<String, FunctionIndex>)> {
    let (program, functions, _) = shared_fixture()?;
    Ok((Arc::clone(program), functions.clone()))
}

fn build_fixture() -> TestResult<SharedFixture> {
    // Build a genuine authored canonical program through the maintained repository
    // APIs; every new fixture setup failure is propagated rather than panicking.
    use crate::platform::persistent_map::{MapContentDigest, MapRoot, PageDigest};
    let empty = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    let seed = b"self-tail-locals";
    let source = crate::platform::kernel::KernelSnapshot {
        root: crate::platform::kernel::SemanticRoot {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: crate::platform::semantic_id::RepositoryId::migrate(seed, 0),
            package_id: crate::platform::kernel::PackageId::migrate(seed, 0),
            package_name: Name::new("self_tail_locals").map_err(failure)?,
            owners: empty,
            dependencies: empty,
            retirements: empty,
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        representation_dependencies: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    };
    let temporary = tempfile::tempdir().map_err(failure)?;
    let repository = GraphRepository::create(&temporary.path().join("reuse"), &source, None)
        .map_err(failure)?
        .repository;
    let base = repository.view_current().map_err(failure)?.revision();
    let input = format!("request base={base}\n{SOURCE}\n{NESTED_SOURCE}\n{CHILD_SOURCE}");
    let decoded = crate::platform::control::decode_compact_change("reuse", input.as_bytes())
        .map_err(|diagnostics| failure(format!("{diagnostics:?}")))?;
    let plan = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .map_err(|diagnostics| failure(format!("{diagnostics:?}")))?;
    repository.publish(&plan.publication).map_err(failure)?;
    let source = repository
        .view_current()
        .map_err(failure)?
        .reconstruct_full_oracle()
        .map_err(failure)?
        .value;
    // This fixture has no package dependencies. Exercise ordinary compilation,
    // linking, strict artifact loading and dense preparation directly; exporting a
    // package transport would add an unrelated embedded native-planner dependency.
    let compilation = crate::platform::compiler::build_clean(
        &repository,
        crate::platform::compiler::OptimizationPolicy::DeterministicBaseline,
    )
    .map_err(failure)?;
    let linked =
        crate::platform::compiler::link_artifact(&repository, compilation.manifest_digest, &[])
            .map_err(failure)?;
    let loaded =
        crate::platform::compiler::load_artifact(&linked.artifact.bytes).map_err(failure)?;
    let program = Arc::new(NormalizedProgram::prepare(loaded).map_err(failure)?);
    let functions = source
        .owners
        .iter()
        .filter_map(|(key, owner)| {
            let (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(owner)) =
                (key, owner)
            else {
                return None;
            };
            program
                .function(DeclarationReference {
                    package: source.root.package_id,
                    declaration: *declaration,
                })
                .map(|function| (owner.name.to_string(), function))
        })
        .collect();
    Ok((program, functions, Arc::new(source)))
}

fn function(functions: &BTreeMap<String, FunctionIndex>, name: &str) -> TestResult<FunctionIndex> {
    functions
        .get(name)
        .copied()
        .ok_or_else(|| failure(format!("missing {name}")))
}

fn code(program: &NormalizedProgram, function: FunctionIndex) -> TestResult<NormalizedCode> {
    match &program
        .functions
        .get(function.0 as usize)
        .ok_or_else(|| failure("missing canonical function"))?
        .body
    {
        NormalizedFunctionBody::Code(code) => Ok(code.clone()),
        NormalizedFunctionBody::External(_) => Err(failure("expected graph code")),
    }
}

fn scalar(program: &NormalizedProgram, n: i64) -> TestResult<CheckedValue> {
    CheckedValue::scalar(program, NormalizedValue::I64(n))
}

fn callback(program: &NormalizedProgram, step: FunctionIndex) -> TestResult<CheckedValue> {
    CheckedValue::function(
        program,
        step,
        Arc::from([]),
        &ExecutionControl::uncancelled(),
    )
}

pub(super) fn machine<'a>(
    program: &'a Arc<NormalizedProgram>,
    executor: &'a StructuredExecutorHandle,
    resources: &'a NormalizedResourceScope,
    control: &'a ExecutionControl,
    regions: &'a super::super::region_value::Arena,
) -> TestResult<Machine<'a>> {
    Ok(Machine {
        regions,
        region_scopes: Vec::new(),
        shared_budget: None,
        structured_depth: 0,
        ancestor_depth: 0,
        memory_domain: ValueOrigin::fresh().ok_or_else(|| failure("identity exhausted"))?,
        root_allowance: None,
        program,
        program_owner: program,
        executor,
        policy: NormalizedRunPolicy::foreground(),
        host: None,
        capabilities: None,
        resources,
        control,
        remaining_steps: None,
        stack: Vec::new(),
        call_arguments: Vec::new(),
        call_type_facts: [None; 32],
        next_call_type_fact: 0,
        frames: Vec::new(),
        incoming_view_scopes: Vec::new(),
        next_frame: 0,
        next_transaction: 0,
        transactions: BTreeMap::new(),
        calls_by_requirement: BTreeMap::new(),
        observation: NormalizedRunObservation::default(),
    })
}

fn invoke(
    program: &NormalizedProgram,
    target: FunctionIndex,
    arguments: Vec<NormalizedValue>,
    fresh: bool,
) -> TestResult<NormalizedInvocation> {
    let run = || {
        NormalizedVm::for_test(program, NormalizedRunPolicy::foreground()).invoke_entry(
            NormalizedEntryPoint::Function(target),
            arguments,
            None,
            &ExecutionControl::uncancelled(),
        )
    };
    if fresh { with_fresh_locals(run) } else { run() }
}

fn equal_work_except_reuse(
    old: &NormalizedRunObservation,
    new: &NormalizedRunObservation,
) -> TestResult {
    let mut expected = old.clone();
    expected.allocated_bytes = new.allocated_bytes;
    expected.allocation_charges = new.allocation_charges;
    expected.value_work.classification_decisions = new.value_work.classification_decisions;
    require(
        expected == *new,
        "reuse changed unrelated execution/admission work",
    )
}

#[test]
fn self_tail_locals_reuse_preserves_scalar_callable_and_owned_results() -> TestResult {
    let (program, functions) = fixture()?;
    let count = function(&functions, "count")?;
    let count_code = code(&program, count)?;
    // The last use of n moves its root into subtract, leaving no occupied old
    // slots at the self edge. Only the one incoming scalar is a new root query.
    require(
        count_code.parameter_count == 1 && count_code.local_count == 1,
        "count is not the independent one-slot fixture",
    )?;
    require(
        count_code
            .instructions
            .iter()
            .filter(|instruction| matches!(instruction, NormalizedInstruction::MoveLocal(0)))
            .count()
            == 1,
        "count no longer moves the sole outgoing root at its last use",
    )?;
    for n in [0_i64, 1, 64] {
        let (old_value, old) = invoke(&program, count, vec![NormalizedValue::I64(n)], true)?;
        let (new_value, new) = invoke(&program, count, vec![NormalizedValue::I64(n)], false)?;
        require(
            old_value == NormalizedValue::I64(0) && new_value == NormalizedValue::I64(0),
            "count differs from its independent scalar answer",
        )?;
        require(
            new.tail_transfers == n as u64
                && new.maximum_control_frames == 1
                && new.collection_items == 0,
            "count changed tail/collection behavior",
        )?;
        let saved = n as u64 * size_of::<Option<CheckedValue>>() as u64;
        require(
            old.allocated_bytes == new.allocated_bytes + saved
                && old.allocation_charges == new.allocation_charges + n as u64,
            "count did not remove exactly its real replacement buffers",
        )?;
        require(
            new.value_work.classification_decisions
                == old.value_work.classification_decisions + n as u64,
            "count did not report its independently derived incoming-root queries",
        )?;
        equal_work_except_reuse(&old, &new)?;
    }
    let target = function(&functions, "callback-loop")?;
    let step = function(&functions, "step")?;
    let callback_code = code(&program, target)?;
    require(
        callback_code.parameter_count == 3 && callback_code.local_count == 3,
        "callback fixture is not the independent three-root shape",
    )?;
    for (local, occurrences) in [(0, 1), (1, 2), (2, 2)] {
        require(callback_code.instructions.iter().filter(|instruction|
            matches!(instruction, NormalizedInstruction::MoveLocal(index) if *index == local))
            .count() == occurrences,
            "callback fixture no longer moves each outgoing root at its branch last use")?;
    }
    let run_callback = |fresh| {
        invoke(
            &program,
            target,
            vec![
                NormalizedValue::I64(32),
                callback(&program, step)?.into_raw(),
                NormalizedValue::I64(7),
            ],
            fresh,
        )
    };
    let (old_value, old) = run_callback(true)?;
    let (new_value, new) = run_callback(false)?;
    require(
        old_value == NormalizedValue::I64(12) && new_value == NormalizedValue::I64(12),
        "sealed ordinary Function callback was not invoked correctly",
    )?;
    let saved = 32 * callback_code.local_count as u64 * size_of::<Option<CheckedValue>>() as u64;
    require(
        old.allocated_bytes == new.allocated_bytes + saved
            && old.allocation_charges == new.allocation_charges + 32,
        "ordinary callbacks incorrectly disqualified self-tail reuse",
    )?;
    // Both branches move their final callback/seed uses; at each self edge the
    // outgoing three slots are empty and the three incoming roots are queried.
    require(
        new.value_work.classification_decisions == old.value_work.classification_decisions + 32 * 3,
        "callback self edges hid their independently derived root queries",
    )?;
    equal_work_except_reuse(&old, &new)?;
    // Fresh Owned data returns through an admitted caller and is consumed there;
    // raw root boundaries intentionally still refuse an escaping memory token.
    let owned = function(&functions, "owned-main")?;
    let (old_value, old) = invoke(&program, owned, vec![NormalizedValue::I64(32)], true)?;
    let (new_value, new) = invoke(&program, owned, vec![NormalizedValue::I64(32)], false)?;
    require(
        old_value == NormalizedValue::I64(137) && new_value == NormalizedValue::I64(137),
        "fresh Owned result differs from its independently extracted scalar",
    )?;
    require(
        old.allocated_bytes == new.allocated_bytes + 32 * size_of::<Option<CheckedValue>>() as u64,
        "fresh Owned result incorrectly disabled ordinary input buffer reuse",
    )?;
    equal_work_except_reuse(&old, &new)?;
    let even = function(&functions, "even")?;
    for n in [31_i64, 32] {
        let (old_value, old) = invoke(&program, even, vec![NormalizedValue::I64(n)], true)?;
        let (new_value, new) = invoke(&program, even, vec![NormalizedValue::I64(n)], false)?;
        require(
            old_value == NormalizedValue::Bool(n % 2 == 0) && old_value == new_value,
            "mutual tail result differs from independent parity",
        )?;
        require(
            old == new,
            "different-function tail fallback changed accounting",
        )?;
    }
    let expected = ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "normalized_value_admission",
        "invocation input value admission: raw value disagrees with its exact type or authority; supply a value for the selected boundary",
    );
    let refused = |fresh| {
        invoke(&program, count, vec![NormalizedValue::Bool(false)], fresh)
            .err()
            .ok_or_else(|| failure("foreign argument type bypassed complete admission"))
    };
    let old_error = refused(true)?;
    let new_error = refused(false)?;
    require(
        old_error == expected && new_error == expected && old_error == new_error,
        "foreign type changed its exact admission class/code/message or refusal flags",
    )?;
    Ok(())
}

#[test]
fn self_tail_locals_clear_dirty_slots_and_keep_fresh_invocation_identity() -> TestResult {
    let (program, functions) = fixture()?;
    let target = function(&functions, "slots")?;
    let step = function(&functions, "step")?;
    let code = code(&program, target)?;
    require(
        code.local_count > code.parameter_count,
        "fixture has no lexical-only slot",
    )?;
    let executor = crate::platform::runtime::structured::StructuredExecutor::for_test(1);
    let handle = executor.handle();
    let resources = NormalizedResourceScope::new()?;
    let control = ExecutionControl::uncancelled();
    let regions = super::super::region_value::Arena::new()?;
    let mut machine = machine(&program, &handle, &resources, &control, &regions)?;
    machine.call_code(
        code.clone(),
        vec![scalar(&program, 3)?, callback(&program, step)?],
        BTreeMap::new(),
        Some(target),
        false,
    )?;
    let dirty: Arc<str> = Arc::from("old lexical branch");
    let retained = Arc::downgrade(&dirty);
    let old = machine.current_frame_mut()?;
    *old.locals
        .last_mut()
        .ok_or_else(|| failure("no dirty slot"))? = Some(CheckedValue::scalar(
        &program,
        NormalizedValue::Text(dirty),
    )?);
    let state = (
        old.id,
        old.application_identity,
        old.invocation_identity,
        old.locals.as_ptr(),
        old.locals.capacity(),
    );
    let before = machine.observation.clone();
    machine.policy.maximum_allocated_bytes = Some(before.allocated_bytes);
    machine.call_code(
        code.clone(),
        vec![scalar(&program, 2)?, callback(&program, step)?],
        BTreeMap::new(),
        Some(target),
        true,
    )?;
    let next = machine.current_frame()?;
    require(
        next.locals.as_ptr() == state.3 && next.locals.capacity() == state.4,
        "eligible transition allocated a different locals buffer",
    )?;
    require(
        next.id != state.0
            && next.application_identity != state.1
            && next.invocation_identity != state.2
            && machine.next_frame == 2,
        "buffer reuse reused activation/application/invocation identity",
    )?;
    require(
        next.locals
            .iter()
            .skip(code.parameter_count as usize)
            .all(Option::is_none)
            && retained.upgrade().is_none(),
        "dirty lexical slot or its old value survived",
    )?;
    require(
        machine.observation.allocated_bytes == before.allocated_bytes
            && machine.observation.allocation_charges == before.allocation_charges,
        "zero-growth buffer reuse incurred a physical allocation charge",
    )?;
    // Three occupied old slots and two incoming roots, in authored slot/argument order.
    require(
        machine.observation.value_work.classification_decisions
            == before.value_work.classification_decisions + 5,
        "occupied slot and callback root queries were hidden",
    )?;
    Ok(())
}

#[test]
fn self_tail_locals_charge_real_growth_and_cancel_before_retiring_old_frame() -> TestResult {
    let (program, functions) = fixture()?;
    let target = function(&functions, "count")?;
    let code = code(&program, target)?;
    let bytes = code.local_count as u64 * size_of::<Option<CheckedValue>>() as u64;
    let executor = crate::platform::runtime::structured::StructuredExecutor::for_test(1);
    let handle = executor.handle();
    let resources = NormalizedResourceScope::new()?;
    let healthy = ExecutionControl::uncancelled();
    for (maximum, accepted) in [(bytes - 1, false), (bytes, true)] {
        let regions = super::super::region_value::Arena::new()?;
        let mut machine = machine(&program, &handle, &resources, &healthy, &regions)?;
        machine.policy.maximum_allocated_bytes = Some(maximum);
        let result = machine.call_code(
            code.clone(),
            vec![scalar(&program, 3)?],
            BTreeMap::new(),
            Some(target),
            false,
        );
        require(
            result.is_ok() == accepted,
            "first buffer exact-fit boundary changed",
        )?;
        require(
            machine.frames.len() == usize::from(accepted),
            "refused first call installed a frame",
        )?;
        if let Err(error) = result {
            require(
                error.code == "normalized_allocation",
                "first buffer refused at a foreign guard",
            )?;
        }
    }
    // The first call consumes the sole permitted checkpoint. The next call fails
    // specifically at call_code's final check; no scope/loan loops consume it.
    let callee = program
        .functions
        .get(target.0 as usize)
        .ok_or_else(|| failure("missing count scope inventory"))?;
    require(
        callee.read_scope_sources.is_empty()
            && callee.read_scope_parameters.is_empty()
            && callee.read_scope_arguments.is_empty(),
        "cancellation fixture has intermediate view-scope checkpoints",
    )?;
    let cancelled = ExecutionControl::cancel_after_checks(1);
    let regions = super::super::region_value::Arena::new()?;
    let mut machine = machine(&program, &handle, &resources, &cancelled, &regions)?;
    machine.call_code(
        code.clone(),
        vec![scalar(&program, 3)?],
        BTreeMap::new(),
        Some(target),
        false,
    )?;
    let old = machine.current_frame()?;
    let state = (
        old.id,
        old.application_identity,
        old.invocation_identity,
        old.locals.as_ptr(),
    );
    let before = machine.observation.allocated_bytes;
    let error = machine
        .call_code(
            code.clone(),
            vec![scalar(&program, 2)?],
            BTreeMap::new(),
            Some(target),
            true,
        )
        .err()
        .ok_or_else(|| failure("final checkpoint did not cancel"))?;
    require(
        error.class == ExecutionFailureClass::Cancelled && error.code == "execution_cancelled",
        "final checkpoint lost its original cancellation diagnostic",
    )?;
    let old = machine.current_frame()?;
    require(
        (
            old.id,
            old.application_identity,
            old.invocation_identity,
            old.locals.as_ptr(),
        ) == state
            && old
                .locals
                .first()
                .and_then(Option::as_ref)
                .map(CheckedValue::raw)
                == Some(&NormalizedValue::I64(3)),
        "cancellation destroyed the outgoing activation",
    )?;
    require(
        machine.observation.allocated_bytes == before,
        "cancelled reuse allocated a locals buffer",
    )?;
    machine.control = &healthy;
    machine.call_code(
        code,
        vec![scalar(&program, 1)?],
        BTreeMap::new(),
        Some(target),
        true,
    )?;
    require(
        machine.current_frame()?.locals.as_ptr() == state.3,
        "healthy retry could not reuse the preserved buffer",
    )?;
    Ok(())
}

#[test]
fn self_tail_locals_fallback_preserves_application_and_borrow_custody() -> TestResult {
    let (program, functions) = fixture()?;
    let target = function(&functions, "count")?;
    let original = code(&program, target)?;
    let bytes = original.local_count as u64 * size_of::<Option<CheckedValue>>() as u64;
    let executor = crate::platform::runtime::structured::StructuredExecutor::for_test(1);
    let handle = executor.handle();
    let resources = NormalizedResourceScope::new()?;
    let control = ExecutionControl::uncancelled();
    for case in 0..5 {
        let regions = super::super::region_value::Arena::new()?;
        let mut machine = machine(&program, &handle, &resources, &control, &regions)?;
        machine.call_code(
            original.clone(),
            vec![scalar(&program, 3)?],
            BTreeMap::new(),
            Some(target),
            false,
        )?;
        let source = BorrowSource {
            frame: machine.current_frame()?.id,
            local: 0,
        };
        let mut next_code = original.clone();
        let mut types = BTreeMap::new();
        let mut argument = scalar(&program, 2)?;
        // Private adversarial placements test the physical fallback boundary;
        // production complete type/scope admission is exercised separately above.
        match case {
            0 => next_code.instructions = original.instructions.iter().cloned().collect(),
            1 => {
                let parameter = TypeParameterId::migrate(b"different-application", 0);
                let ty = program
                    .functions
                    .get(target.0 as usize)
                    .ok_or_else(|| failure("missing count"))?
                    .result
                    .ty;
                types.insert(parameter, ty);
            }
            2 => machine
                .current_frame_mut()?
                .locals
                .first_mut()
                .and_then(Option::as_mut)
                .ok_or_else(|| failure("missing old scalar"))?
                .set_borrow_source(Some(source)),
            3 => argument.set_borrow_source(Some(source)),
            4 => {
                let cell = OwnedI64Cell::new(machine.memory_domain, 137);
                cell.establish_admission(program.value_origin)?;
                let read = cell.borrow()?;
                let mut leaf = scalar(&program, 3)?;
                leaf.set_borrow_source(Some(source));
                leaf.reserve_borrow_guards(1)?;
                leaf.retain_borrow_guard_from(|| {
                    CheckedValue::memory(&program, NormalizedValue::OwnedI64Cell(read))
                })?;
                *machine
                    .current_frame_mut()?
                    .locals
                    .first_mut()
                    .ok_or_else(|| failure("missing custody slot"))? = Some(leaf);
                // Keep the real owner outside the frame until the fallback retry
                // releases the retained read, then confirm ownership is usable.
                let state = machine.current_frame()?.locals.as_ptr();
                let before = machine.observation.allocated_bytes;
                machine.policy.maximum_allocated_bytes = Some(before + bytes - 1);
                let error = machine
                    .call_code(
                        next_code.clone(),
                        vec![scalar(&program, 2)?],
                        BTreeMap::new(),
                        Some(target),
                        true,
                    )
                    .err()
                    .ok_or_else(|| failure("custody fallback escaped allocation refusal"))?;
                require(
                    error.code == "normalized_allocation"
                        && machine.current_frame()?.locals.as_ptr() == state,
                    "refused fallback retired custody before allocation admission",
                )?;
                require(
                    cell.borrow().is_ok(),
                    "refused fallback invalidated the retained ancestor",
                )?;
                machine.policy.maximum_allocated_bytes = Some(before + bytes);
                machine.call_code(
                    next_code,
                    vec![scalar(&program, 1)?],
                    BTreeMap::new(),
                    Some(target),
                    true,
                )?;
                require(
                    machine.current_frame()?.locals.as_ptr() != state
                        && machine.observation.allocated_bytes == before + bytes,
                    "custody fallback reused the old buffer",
                )?;
                require(
                    cell.extract()? == 137,
                    "fallback failed to release the old read guard",
                )?;
                continue;
            }
            _ => return Err(failure("unknown fallback case")),
        }
        let state = machine.current_frame()?.locals.as_ptr();
        let before = machine.observation.clone();
        machine.policy.maximum_allocated_bytes = Some(before.allocated_bytes + bytes - 1);
        let error = machine
            .call_code(
                next_code.clone(),
                vec![argument],
                types.clone(),
                Some(target),
                true,
            )
            .err()
            .ok_or_else(|| failure("fallback bypassed real growth charge"))?;
        require(
            error.code == "normalized_allocation"
                && machine.current_frame()?.locals.as_ptr() == state
                && machine.current_frame()?.id == source.frame,
            "fallback refused after destroying the outgoing activation",
        )?;
        let queries = match case {
            0 | 1 => 0,
            2 => 1,
            3 => 2,
            _ => 0,
        };
        require(
            machine.observation.value_work.classification_decisions
                == before.value_work.classification_decisions + queries,
            "fallback short-circuit root queries were counted incorrectly",
        )?;
        machine.policy.maximum_allocated_bytes = Some(before.allocated_bytes + bytes);
        let mut argument = scalar(&program, 1)?;
        if case == 3 {
            argument.set_borrow_source(Some(source));
        }
        machine.call_code(next_code, vec![argument], types, Some(target), true)?;
        require(
            machine.current_frame()?.locals.as_ptr() != state
                && machine.observation.allocated_bytes == before.allocated_bytes + bytes
                && machine.observation.allocation_charges == before.allocation_charges + 1,
            "fallback did not allocate/charge exactly one fresh local buffer",
        )?;
        if case == 3 {
            require(
                machine
                    .current_frame()?
                    .locals
                    .first()
                    .and_then(Option::as_ref)
                    .and_then(CheckedValue::borrow_source)
                    == Some(source),
                "fallback stripped incoming borrow metadata",
            )?;
        }
    }
    Ok(())
}

#[test]
fn call_argument_reuse_preserves_results_traps_quotas_and_complete_observations() -> TestResult {
    let (program, functions) = fixture()?;
    let count = function(&functions, "count")?;
    let even = function(&functions, "even")?;
    let owned = function(&functions, "owned-main")?;
    let step = function(&functions, "step")?;
    let mut fresh_buffers = 0;
    let mut reused_buffers = 0;
    for (target, input, expected) in [
        (count, 64, Ok(NormalizedValue::I64(0))),
        (even, 31, Ok(NormalizedValue::Bool(false))),
        (owned, 32, Ok(NormalizedValue::I64(137))),
        (step, i64::MAX, Err("normalized_integer_overflow")),
    ] {
        for fuel in [None, Some(17)] {
            let run = |fresh| {
                with_call_arguments(fresh, || {
                    let observer = std::sync::Mutex::new(None);
                    let result = NormalizedVm::for_test(
                        &program,
                        NormalizedRunPolicy {
                            instruction_steps: fuel,
                            ..NormalizedRunPolicy::foreground()
                        },
                    )
                    .observing_checked(&observer)
                    .invoke_entry(
                        NormalizedEntryPoint::Function(target),
                        vec![NormalizedValue::I64(input)],
                        None,
                        &ExecutionControl::uncancelled(),
                    )
                    .map(|(value, _)| value);
                    Ok::<_, ExecutionError>((
                        result,
                        observer
                            .into_inner()
                            .map_err(failure)?
                            .ok_or_else(|| failure("missing observation"))?,
                    ))
                })
            };
            let (before, (allocations, _)) = run(true);
            let (after, (growth, reuse)) = run(false);
            let (before_value, before_work) = before?;
            let (after_value, after_work) = after?;
            require(
                before_value == after_value,
                "argument storage changed result or refusal",
            )?;
            require(
                before_work == after_work,
                "argument storage changed semantic work or quota accounting",
            )?;
            require(
                before_work.live_call_frames_after == 0
                    && before_work.live_handles_after == 0
                    && before_work.live_locals_after == 0
                    && before_work.live_operands_after == 0
                    && before_work.live_transactions_after == 0
                    && before_work.live_type_bindings_after == 0,
                "argument storage retained execution custody",
            )?;
            if fuel.is_none() {
                match &expected {
                    Ok(expected) => {
                        require(after_value? == *expected, "independent result differs")?
                    }
                    Err(code) => require(
                        after_value.err().is_some_and(|error| error.code == *code),
                        "independent trap differs",
                    )?,
                }
            }
            require(
                growth <= allocations,
                "reuse increased argument vector growth",
            )?;
            fresh_buffers += allocations;
            reused_buffers += reuse;
        }
    }
    require(
        fresh_buffers > 200 && reused_buffers > 200,
        "comparison did not exercise repeated argument transports",
    )
}

#[test]
fn call_argument_buffer_reuses_actual_storage_and_preserves_stack_boundaries() -> TestResult {
    let (program, functions) = fixture()?;
    let count = function(&functions, "count")?;
    let add = function(&functions, "add")?;
    let executor = crate::platform::runtime::structured::StructuredExecutor::for_test(1);
    let resources = NormalizedResourceScope::new()?;
    let control = ExecutionControl::uncancelled();
    let handle = executor.handle();
    let regions = super::super::region_value::Arena::new()?;
    let mut machine = machine(&program, &handle, &resources, &control, &regions)?;
    machine.call_code(
        code(&program, count)?,
        vec![scalar(&program, 0)?],
        BTreeMap::new(),
        Some(count),
        false,
    )?;
    machine.push(scalar(&program, 111)?)?;
    let mut previous = None;
    for _ in 0..64 {
        machine.push(scalar(&program, 4)?)?;
        machine.push(scalar(&program, 9)?)?;
        let arguments = machine.pop_call_arguments(2)?;
        let storage = (arguments.as_ptr(), arguments.capacity());
        if let Some(previous) = previous {
            require(storage == previous, "call argument storage was replaced")?;
        }
        previous = Some(storage);
        machine.call(add, Arc::from([]), arguments)?;
        require(
            machine.pop()?.into_raw() == NormalizedValue::I64(13),
            "argument order changed",
        )?;
        require(
            machine.call_arguments.is_empty(),
            "reusable buffer retained a value",
        )?;
    }
    require(
        machine.pop_call_arguments(2).is_err(),
        "argument transport crossed its stack boundary",
    )?;
    require(
        machine.pop()?.into_raw() == NormalizedValue::I64(111),
        "refused pop altered the retained operand",
    )?;
    machine.push(CheckedValue::scalar(
        &program,
        NormalizedValue::Bool(false),
    )?)?;
    machine.push(scalar(&program, 9)?)?;
    let arguments = machine.pop_call_arguments(2)?;
    require(
        machine.call(add, Arc::from([]), arguments).is_err(),
        "foreign scalar bypassed intrinsic admission",
    )?;
    require(
        machine.call_arguments.is_empty(),
        "failed intrinsic retained an argument",
    )?;
    Ok(())
}

#[test]
#[ignore = "matched argument-storage cost control; run explicitly"]
fn call_argument_storage_cost_control() -> TestResult {
    let (program, functions) = fixture()?;
    let target = function(&functions, "count")?;
    let input = 100_000;
    let mut observations = Vec::new();
    for fresh in [true, false] {
        let started = std::time::Instant::now();
        let (result, (growths, reuses)) = with_call_arguments(fresh, || {
            invoke(&program, target, vec![NormalizedValue::I64(input)], false)
        });
        let elapsed = started.elapsed();
        let (value, work) = result?;
        require(
            value == NormalizedValue::I64(0),
            "cost control independent result differs",
        )?;
        observations.push(work.clone());
        println!(
            "{}",
            serde_json::json!({
                "control":"call-argument-storage", "fresh_buffers":fresh, "iterations":input,
                "elapsed_nanoseconds":elapsed.as_nanos(), "buffer_growths":growths,
                "buffer_reuses":reuses, "observation":work,
            })
        );
    }
    require(
        observations[0] == observations[1],
        "cost control changed semantic work",
    )
}

#[test]
fn call_template_facts_require_complete_admission_and_poll_cached_reads() -> TestResult {
    let (program, _) = fixture()?;
    let scalar = program
        .types
        .iter()
        .find_map(|(ty, object)| matches!(object.form, TypeForm::I64).then_some(*ty))
        .ok_or_else(|| failure("missing I64 type"))?;
    let executor = crate::platform::runtime::structured::StructuredExecutor::for_test(1);
    let resources = NormalizedResourceScope::new()?;
    let control = ExecutionControl::uncancelled();
    let handle = executor.handle();
    let regions = super::super::region_value::Arena::new()?;
    let mut machine = machine(&program, &handle, &resources, &control, &regions)?;
    require(
        !machine.call_type_has_associated(scalar)?,
        "I64 acquired an associated type occurrence",
    )?;
    let facts = machine.call_type_facts;
    require(
        facts
            .iter()
            .flatten()
            .any(|(ty, associated)| *ty == scalar && !associated),
        "complete successful fact was not retained",
    )?;
    require(
        !machine.call_type_has_associated(scalar)? && machine.call_type_facts == facts,
        "cache hit changed the derived fact",
    )?;
    let missing = TypeObjectDigest::from_bytes([0xa7; 32]);
    require(
        !program.types.contains_key(&missing),
        "missing-type test identity exists",
    )?;
    require(
        machine.call_type_has_associated(missing).is_err(),
        "missing template received a cached fact",
    )?;
    require(
        machine.call_type_facts == facts,
        "failed admission populated the proof cache",
    )?;
    control.cancel();
    require(
        machine
            .call_type_has_associated(scalar)
            .err()
            .is_some_and(|error| error.class == ExecutionFailureClass::Cancelled),
        "cached template bypassed cancellation",
    )
}

const NESTED_SOURCE: &str = r#"declarations.begin
(units (module create nested
  (type-alias Payload (map I64 (list I64)))
  (type-alias Packet (owned-product (field cell OwnedI64Cell) (field data Payload) (field tag I64)))
  (type-alias Values (owned-sequence Payload))
  (type-alias Replaced (owned-product (field rest Values) (field value Payload)))
  (type-alias Outcome (owned-choice (case accepted Payload) (case empty OwnedI64Cell)))
  (external create nested-less (visibility private) (implementation core.i64.less)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns Bool))
  (external create nested-subtract (visibility private) (implementation core.i64.subtract)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (external create nested-new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create nested-finish-cell (visibility private) (implementation core.cell.extract)
    (parameter create cell (type OwnedI64Cell) (use consume)) (returns I64))
  (function create nested-product-loop (visibility public) (effect pure)
    (parameter create n (type I64)) (parameter create payload (type Payload)) (returns I64)
    (body (if (call nested-less (local n) (i64 1)) (i64 0)
      (let (binding cell (type OwnedI64Cell) (call nested-new-cell (i64 19)))
        (binding packet (type Packet) (pack-owned (type Packet)
        (field cell (local cell)) (field data (local payload)) (field tag (local n))))
        (in (unpack-owned (type Packet) (local packet)
          (field cell (binding cell (type OwnedI64Cell)))
          (field data (binding restored (type Payload))) (field tag (binding remaining (type I64)))
          (in (sequence (call nested-finish-cell (local cell)) (call nested-product-loop (call nested-subtract (local remaining) (i64 1))
            (local restored))))))))))
  (function create nested-sequence-loop (visibility private) (effect pure)
    (parameter create n (type I64)) (parameter create payload (type Payload))
    (parameter create values (type Values) (use consume)) (returns Values)
    (body (if (call nested-less (local n) (i64 1)) (local values)
      (let (binding next (type Values)
        (sequence-push (type Values) (local payload) (local values)))
        (in (call nested-sequence-loop (call nested-subtract (local n) (i64 1))
          (local payload) (local next)))))))
  (function create nested-sequence-main (visibility public) (effect pure)
    (parameter create n (type I64)) (parameter create payload (type Payload)) (returns I64)
    (body (let
      (binding empty (type Values) (sequence-empty (type Values)))
      (binding values (type Values) (call nested-sequence-loop (local n) (local payload) (local empty)))
      (in (sequence-length (type Values) (local values))))))
  (function create nested-replace (visibility public) (effect pure)
    (parameter create payload (type Payload)) (parameter create replacement (type Payload)) (returns Payload)
    (body (let
      (binding empty (type Values) (sequence-empty (type Values)))
      (binding values (type Values) (sequence-push (type Values) (local payload) (local empty)))
      (binding replaced (type Replaced)
        (sequence-replace (type Values) (index (i64 0)) (local replacement) (local values)))
      (in (unpack-owned (type Replaced) (local replaced)
        (field rest (binding rest (type Values))) (field value (binding displaced (type Payload)))
        (in (sequence (sequence-length (type Values) (local rest)) (local displaced))))))))
  (function create nested-choice (visibility public) (effect pure)
    (parameter create payload (type Payload)) (returns I64)
    (body (let (binding chosen (type Outcome)
      (choose-owned (type Outcome) (case accepted) (local payload)))
      (in (match-owned (type Outcome) (local chosen)
        (case accepted (binding payload (type Payload)) (in (sequence (local payload) (i64 31))))
        (case empty (binding ignored (type OwnedI64Cell)) (in (i64 -1))))))))))
declarations.end"#;

const CHILD_SOURCE: &str = r#"declarations.begin
(units (module create child-controls
  (record create Alpha (visibility public) (field create tag (type I64)))
  (record create Beta (visibility public) (field create tag (type I64)))
  (record create Phantom (visibility public) (type-parameter create T)
    (field create tag (type I64)))
  (type-alias BoolChoice (owned-choice (case accepted (list Bool)) (case empty OwnedI64Cell)))
  (type-alias BetaChoice (owned-choice (case accepted Beta) (case empty OwnedI64Cell)))
  (type-alias PhantomChoice (owned-choice (case accepted (Phantom Bool)) (case empty OwnedI64Cell)))
  (function create child-shapes (visibility public) (effect pure)
    (parameter create ints (type (list I64))) (parameter create bools (type (list Bool)))
    (parameter create alpha (type Alpha)) (parameter create beta (type Beta))
    (parameter create phantom-int (type (Phantom I64)))
    (parameter create phantom-bool (type (Phantom Bool)))
    (parameter create bool-choice (type BoolChoice) (use borrow))
    (parameter create beta-choice (type BetaChoice) (use borrow))
    (parameter create phantom-choice (type PhantomChoice) (use borrow))
    (returns Unit) (body (unit)))))
declarations.end"#;

#[test]
fn checked_owned_child_instruction_refuses_wrong_list_nominal_and_generic_proofs() -> TestResult {
    let (program, functions) = fixture()?;
    let target = function(&functions, "child-shapes")?;
    let parameters = &program.functions[target.0 as usize].parameters;
    let executor = crate::platform::runtime::structured::StructuredExecutor::for_test(1);
    let resources = NormalizedResourceScope::new()?;
    let control = ExecutionControl::uncancelled();
    let handle = executor.handle();
    for (source, destination, choice, empty) in [
        (0, 1, 6, false),
        (0, 1, 6, true),
        (2, 3, 7, false),
        (4, 5, 8, false),
    ] {
        let source_type = parameters[source].ty;
        let expected_type = parameters[destination].ty;
        let raw = if source == 0 {
            NormalizedValue::list(if empty {
                vec![]
            } else {
                vec![NormalizedValue::I64(37)]
            })?
        } else {
            let layout = *program
                .record_instances
                .get(&source_type)
                .ok_or_else(|| failure("missing genuine nominal layout"))?;
            NormalizedValue::Record(NormalizedRecord::Nominal {
                layout,
                fields: Arc::new(vec![NormalizedValue::I64(37)]),
            })
        };
        let mut outcomes = Vec::new();
        for strict in [true, false] {
            let run = || -> TestResult<_> {
                let storage = super::super::owned_product::StorageObservation::start();
                let regions = super::super::region_value::Arena::new()?;
                let mut machine = machine(&program, &handle, &resources, &control, &regions)?;
                let admitted = machine.admit(raw.clone(), source_type, None, true)?;
                require(
                    strict
                        || admitted.has_exact_ordinary_type(
                            &program,
                            source_type,
                            &mut machine.observation.value_work,
                        )?,
                    "input lacks its genuine exact proof",
                )?;
                require(
                    !admitted.has_exact_ordinary_type(
                        &program,
                        expected_type,
                        &mut machine.observation.value_work,
                    )?,
                    "foreign expected type reused an exact proof",
                )?;
                // Deliberately substitute a different, genuinely prepared choice
                // operand into a tiny instruction stream. The production VM must
                // re-admit the child rather than trust the previous root proof.
                machine.call_code(
                    NormalizedCode {
                        parameter_count: 1,
                        local_count: 1,
                        instructions: Arc::from([
                            NormalizedInstruction::LoadLocal {
                                local: 0,
                                use_mode: ParameterUse::Unrestricted,
                            },
                            NormalizedInstruction::ChooseOwned {
                                choice_type: parameters[choice].ty,
                                case: 0,
                            },
                            NormalizedInstruction::Return,
                        ]),
                    },
                    vec![admitted],
                    BTreeMap::new(),
                    None,
                    false,
                )?;
                let before = machine.observation.value_work.raw_result_admission_nodes;
                let result = machine.run();
                require(
                    machine.observation.value_work.raw_result_admission_nodes > before,
                    "mismatched proof bypassed raw fallback admission",
                )?;
                let outcome = match result {
                    Ok(value) => {
                        require(
                            empty && matches!(value.raw(), NormalizedValue::OwnedChoice(_)),
                            "wrong child escaped as an owned choice",
                        )?;
                        drop(value);
                        None
                    }
                    Err(error) => {
                        require(
                            !empty,
                            "compatible empty list lost full structural fallback",
                        )?;
                        Some((error.class, error.code))
                    }
                };
                drop(machine);
                require(
                    storage.live() == (0, 0),
                    "refused child retained owned storage",
                )?;
                Ok(outcome)
            };
            outcomes.push(if strict {
                checked::force_strict_ordinary_children(run)
            } else {
                run()
            }?);
        }
        require(
            outcomes[0] == outcomes[1],
            "exact proof changed strict instruction refusal",
        )?;
    }
    Ok(())
}

fn nested_payload(width: i64, length: i64) -> TestResult<NormalizedValue> {
    let control = ExecutionControl::uncancelled();
    let mut entries = BTreeMap::new();
    for key in 0..width {
        let mut reserve = super::super::value::raw_map_reservation(&control);
        let list = super::super::list::List::from_items(
            (0..length)
                .map(|index| NormalizedValue::I64(key * 100 + index))
                .collect(),
            super::super::value::MAXIMUM_ADMISSION_ITEMS,
            &mut |charge| {
                reserve(super::super::map::Charge {
                    slots: charge.slots,
                    bytes: charge.bytes,
                })
            },
        )?;
        entries.insert(NormalizedMapKey::I64(key), NormalizedValue::List(list));
    }
    let mut reserve = super::super::value::raw_map_reservation(&control);
    super::super::map::Map::from_items(
        entries,
        super::super::value::MAXIMUM_ADMISSION_ITEMS,
        &mut reserve,
    )
    .map(NormalizedValue::Map)
}

fn nested_run(
    program: &Arc<NormalizedProgram>,
    target: FunctionIndex,
    arguments: Vec<NormalizedValue>,
    policy: NormalizedRunPolicy,
    strict: bool,
) -> TestResult<(
    Result<NormalizedValue, ExecutionError>,
    NormalizedRunObservation,
)> {
    let run = || {
        let observer = std::sync::Mutex::new(None);
        let result = NormalizedVm::for_test(program, policy)
            .observing_checked(&observer)
            .invoke_entry(
                NormalizedEntryPoint::Function(target),
                arguments,
                None,
                &ExecutionControl::uncancelled(),
            )
            .map(|(value, _)| value);
        Ok((
            result,
            observer
                .into_inner()
                .map_err(failure)?
                .ok_or_else(|| failure("missing nested observation"))?,
        ))
    };
    if strict {
        checked::force_strict_ordinary_children(run)
    } else {
        run()
    }
}

#[test]
fn checked_owned_children_preserve_nested_data_reference_results_quotas_and_cleanup() -> TestResult
{
    let (program, functions, source) = shared_fixture()?;
    let payload = nested_payload(8, 16)?;
    for name in [
        "nested-product-loop",
        "nested-sequence-main",
        "nested-replace",
        "nested-choice",
    ] {
        let target = function(functions, name)?;
        let (arguments, expected) = match name {
            "nested-replace" => (
                vec![payload.clone(), nested_payload(2, 3)?],
                payload.clone(),
            ),
            "nested-choice" => (vec![payload.clone()], NormalizedValue::I64(31)),
            "nested-product-loop" => (
                vec![NormalizedValue::I64(3), payload.clone()],
                NormalizedValue::I64(0),
            ),
            _ => (
                vec![NormalizedValue::I64(3), payload.clone()],
                NormalizedValue::I64(3),
            ),
        };
        let mut retained = None;
        for strict in [true, false] {
            let (result, work) = nested_run(
                program,
                target,
                arguments.clone(),
                NormalizedRunPolicy::foreground(),
                strict,
            )?;
            require(
                result? == expected,
                "checked child changed independent nested result",
            )?;
            require(
                work.live_call_frames_after
                    + work.live_handles_after
                    + work.live_locals_after
                    + work.live_operands_after
                    + work.live_transactions_after
                    + work.live_type_bindings_after
                    == 0,
                "checked child retained custody",
            )?;
            if let Some(before) = retained {
                let before: NormalizedRunObservation = before;
                require(
                    before.instructions == work.instructions
                        && before.calls == work.calls
                        && before.tail_transfers == work.tail_transfers
                        && before.capability_calls == work.capability_calls,
                    "checked child changed language execution",
                )?;
                require(
                    work.value_work.raw_result_admission_nodes
                        < before.value_work.raw_result_admission_nodes,
                    "nested fixture did not remove repeated subtree admission",
                )?;
            }
            retained = Some(work.clone());
            let (refused, stopped) = nested_run(
                program,
                target,
                arguments.clone(),
                NormalizedRunPolicy {
                    instruction_steps: Some(1),
                    ..NormalizedRunPolicy::foreground()
                },
                strict,
            )?;
            require(
                refused
                    .err()
                    .is_some_and(|error| error.code == "normalized_instruction_steps"),
                "finite instruction refusal changed",
            )?;
            require(
                stopped.live_call_frames_after
                    + stopped.live_handles_after
                    + stopped.live_locals_after
                    + stopped.live_operands_after
                    == 0,
                "quota exit retained checked child custody",
            )?;
        }
        let reference = super::super::reference::NormalizedReferenceInterpreter::new(
            source,
            program,
            NormalizedRunPolicy::foreground(),
        )
        .invoke(
            program.functions[target.0 as usize].declaration,
            arguments,
            None,
            &ExecutionControl::uncancelled(),
        )?;
        require(
            reference.0 == expected,
            "independent source reference differs",
        )?;
    }
    Ok(())
}

#[test]
#[ignore = "matched nested ordinary-child admission cost control; run explicitly"]
fn checked_owned_children_nested_cost_control() -> TestResult {
    let (program, functions, _) = shared_fixture()?;
    let payload = nested_payload(128, 32)?;
    for name in ["nested-product-loop", "nested-sequence-main"] {
        let target = function(functions, name)?;
        let mut before: Option<NormalizedRunObservation> = None;
        for strict in [true, false] {
            let started = std::time::Instant::now();
            let (result, work) = nested_run(
                program,
                target,
                vec![NormalizedValue::I64(256), payload.clone()],
                NormalizedRunPolicy::foreground(),
                strict,
            )?;
            let elapsed = started.elapsed();
            require(
                result?
                    == NormalizedValue::I64(if name == "nested-product-loop" {
                        0
                    } else {
                        256
                    }),
                "nested cost independent answer differs",
            )?;
            if let Some(before) = &before {
                require(
                    before.instructions == work.instructions
                        && before.calls == work.calls
                        && before.tail_transfers == work.tail_transfers
                        && before.capability_calls == work.capability_calls,
                    "nested cost execution changed",
                )?;
                require(
                    work.value_work.raw_result_admission_nodes
                        < before.value_work.raw_result_admission_nodes,
                    "nested cost did not reuse sealed evidence",
                )?;
            }
            println!(
                "{}",
                serde_json::json!({"control":"checked-owned-children", "target":name, "strict_admission":strict, "iterations":256, "map_entries":128, "list_items":32, "elapsed_nanoseconds":elapsed.as_nanos(), "observation":work})
            );
            before = Some(work);
        }
    }
    Ok(())
}
