//! Language equality through admitted finite recursive types and explicit raw negatives.
//! Expected answers come from fixture construction, never from another comparator.

use super::super::child_storage::Values;
use super::super::reference::{
    self, CoreNormalizedReferenceHost, NormalizedReferenceHost, ReferenceSignature,
};
use super::super::value::{NormalizedRecord, RecordLayoutIndex};
use super::super::value_schema::NormalizedValueSchema;
use super::super::vm::{self, CoreNormalizedHost, NormalizedHost};
use super::*;
use crate::platform::binary64::Binary64;
use crate::platform::kernel::KernelSnapshot;
use std::sync::atomic::AtomicUsize;

type Check<T = ()> = Result<T, String>;

const SOURCE: &str = r#"declarations.begin
(units (module create equality-lifecycle
  (record create Node (visibility private)
    (field create children (type (list Node))))
  (external create same (visibility private) (implementation core.value.equal)
    (parameter create left (type Node))
    (parameter create right (type Node)) (returns Bool))
  (function create compare (visibility private) (effect pure)
    (parameter create left (type Node))
    (parameter create right (type Node)) (returns Bool)
    (body (call same (local left) (local right))))
  (external create subtract (visibility private) (implementation core.i64.subtract)
    (parameter create left (type I64))
    (parameter create right (type I64)) (returns I64))
  (external create add (visibility private) (implementation core.i64.add)
    (parameter create left (type I64))
    (parameter create right (type I64)) (returns I64))
  (external create zero (visibility private) (implementation core.i64.equal)
    (parameter create left (type I64))
    (parameter create right (type I64)) (returns Bool))
  (function create leaf (visibility private) (effect pure) (returns Node)
    (body (record Node (field Node::children (list Node)))))
  (function create chain (visibility private) (effect pure)
    (parameter create depth (type I64)) (parameter create value (type Node))
    (returns Node)
    (body (if (call zero (local depth) (i64 0)) (local value)
      (call chain (call subtract (local depth) (i64 1))
        (record Node (field Node::children (list Node (local value))))))))
  (function create compare-built (visibility private) (effect pure)
    (parameter create depth (type I64)) (parameter create delta (type I64))
    (returns Bool)
    (body (let
      (binding left (call chain (local depth) (call leaf)))
      (binding right (call chain (call add (local depth) (local delta)) (call leaf)))
      (in (call same (local left) (local right))))))))
declarations.end
"#;

fn failure(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

struct Fixture {
    source: KernelSnapshot,
    program: NormalizedProgram,
    node: RecordLayoutIndex,
}

impl Fixture {
    fn new() -> Check<Self> {
        let source = byte_buffer_tests::author_only(SOURCE)?;
        assert!(crate::platform::kernel::memory_reference::accepts(&source));
        let program = prepare_snapshot(&source);
        let declaration = declaration_named(&source, "Node");
        let (index, record) = program
            .records
            .iter()
            .enumerate()
            .find(|(_, record)| record.declaration == declaration)
            .ok_or("finite recursive Node layout is missing")?;
        assert!(!record.owned);
        assert_eq!(record.fields.len(), 1);
        assert_eq!(record.fields[0].name.as_str(), "children");
        let node_type = program.types.iter().find_map(|(identity, object)| {
            matches!(object.form, TypeForm::Named { declaration: selected } if selected == declaration)
                .then_some(*identity)
        }).ok_or("finite nominal Node identity is missing")?;
        assert!(matches!(
            program.types.get(&record.fields[0].ty).map(|object| &object.form),
            Some(TypeForm::List { item }) if *item == node_type
        ));
        let node = RecordLayoutIndex(u32::try_from(index).map_err(failure)?, program.value_origin);
        Ok(Self {
            source,
            program,
            node,
        })
    }

    fn node(&self, children: Vec<NormalizedValue>) -> Check<NormalizedValue> {
        Ok(NormalizedValue::Record(NormalizedRecord::Nominal {
            layout: self.node,
            fields: Values::try_new(vec![NormalizedValue::list(children).map_err(failure)?])
                .map_err(failure)?,
        }))
    }

    fn chain(&self, depth: usize, branching: bool) -> Check<(NormalizedValue, Values)> {
        let empty = self.node(Vec::new())?;
        let NormalizedValue::Record(NormalizedRecord::Nominal { fields, .. }) = &empty else {
            return Err("fixture leaf is not its nominal record".to_owned());
        };
        let witness = fields.clone();
        let mut value = empty.clone();
        for _ in 0..depth {
            let children = if branching {
                vec![value, empty.clone()]
            } else {
                vec![value]
            };
            value = self.node(children)?;
        }
        drop(empty);
        // This is a shared-storage observation, not an equality/type oracle.
        assert_eq!(
            witness.strong_count(),
            if branching { depth + 2 } else { 2 }
        );
        Ok((value, witness))
    }
}

fn on_small_stack(work: impl FnOnce() -> Check + Send) -> Check {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("semantic-equality-lifecycle".to_owned())
            .stack_size(512 * 1024)
            .spawn_scoped(scope, work)
            .map_err(failure)?
            .join()
            .map_err(|_| "semantic equality worker panicked".to_owned())?
    })
}

struct Observed {
    value: Result<NormalizedValue, ExecutionError>,
    bytes: u64,
    external_calls: u64,
}

fn invoke(
    fixture: &Fixture,
    reference: bool,
    name: &str,
    arguments: Vec<NormalizedValue>,
    policy: NormalizedRunPolicy,
    control: &ExecutionControl,
    host: Option<&ComparisonHost>,
) -> Check<Observed> {
    let declaration = declaration_named(&fixture.source, name);
    if reference {
        let sink = Mutex::new(None);
        let interpreter =
            NormalizedReferenceInterpreter::new(&fixture.source, &fixture.program, policy);
        let interpreter = if let Some(host) = host {
            interpreter.observing(&sink, host)
        } else {
            interpreter.observing_checked(&sink)
        };
        let value = interpreter
            .invoke(declaration, arguments, None, control)
            .map(|pair| pair.0);
        let observation = sink
            .into_inner()
            .map_err(failure)?
            .ok_or("missing reference cleanup observation")?;
        assert_eq!(
            observation.live_call_frames_after
                + observation.live_control_frames_after
                + observation.live_local_scopes_after
                + observation.live_type_scopes_after
                + observation.live_effect_scopes_after
                + observation.live_allowances_after
                + observation.live_transactions_after
                + observation.live_handles_after,
            0
        );
        assert!(!observation.allocated_bytes_saturated && !observation.collection_items_saturated);
        Ok(Observed {
            value,
            bytes: observation.allocated_bytes,
            external_calls: observation.external_calls,
        })
    } else {
        let sink = Mutex::new(None);
        let interpreter = NormalizedVm::for_test(&fixture.program, policy);
        let interpreter = if let Some(host) = host {
            interpreter.observing(&sink, host)
        } else {
            interpreter.observing_checked(&sink)
        };
        let value = interpreter
            .invoke(declaration, arguments, None, control)
            .map(|pair| pair.0);
        let observation = sink
            .into_inner()
            .map_err(failure)?
            .ok_or("missing VM cleanup observation")?;
        assert_eq!(
            observation.live_call_frames_after
                + observation.live_locals_after
                + observation.live_type_bindings_after
                + observation.live_operands_after
                + observation.live_transactions_after
                + observation.live_handles_after,
            0
        );
        assert!(!observation.allocated_bytes_saturated && !observation.collection_items_saturated);
        Ok(Observed {
            value,
            bytes: observation.allocated_bytes,
            external_calls: observation.external_calls,
        })
    }
}

fn expect_bool(observed: Observed, expected: bool) -> Check<u64> {
    assert!(
        observed.external_calls > 0,
        "the equality external must execute"
    );
    match observed.value {
        Ok(NormalizedValue::Bool(actual)) => assert_eq!(actual, expected),
        other => return Err(format!("expected language Bool({expected}), got {other:?}")),
    }
    Ok(observed.bytes)
}

struct ComparisonHost {
    entered: AtomicUsize,
}

impl ComparisonHost {
    fn selected_control(&self, implementation: &ImplementationName) -> Option<ExecutionControl> {
        if implementation.as_str() == "core.value.equal" {
            self.entered.fetch_add(1, Ordering::Relaxed);
            Some(ExecutionControl::cancel_after_checks(64))
        } else {
            None
        }
    }
}

impl NormalizedHost for ComparisonHost {
    fn call(
        &self,
        program: &NormalizedProgram,
        function: &super::super::prepare::NormalizedFunction,
        implementation: &ImplementationName,
        types: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        let selected = self.selected_control(implementation);
        CoreNormalizedHost.call(
            program,
            function,
            implementation,
            types,
            arguments,
            selected.as_ref().unwrap_or(control),
        )
    }
}

impl NormalizedReferenceHost for ComparisonHost {
    fn call(
        &self,
        schema: &dyn NormalizedValueSchema,
        function: &ReferenceSignature,
        implementation: &ImplementationName,
        types: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        let selected = self.selected_control(implementation);
        CoreNormalizedReferenceHost.call(
            schema,
            function,
            implementation,
            types,
            arguments,
            selected.as_ref().unwrap_or(control),
        )
    }
}

#[test]
fn semantic_equality_recursive_nominal_lists_are_stack_safe_in_both_evaluators() -> Check {
    let fixture = Fixture::new()?;
    on_small_stack(|| {
        let control = ExecutionControl::uncancelled();
        for reference in [false, true] {
            // The source loop is tail recursive; value depth does not require call depth.
            for delta in [0, 1] {
                expect_bool(
                    invoke(
                        &fixture,
                        reference,
                        "compare-built",
                        vec![NormalizedValue::I64(128), NormalizedValue::I64(delta)],
                        NormalizedRunPolicy::foreground(),
                        &control,
                        None,
                    )?,
                    delta == 0,
                )?;
            }
            // Separate construction gives equal contents, not a shared-root shortcut.
            // A pending sibling at every layer also exercises a deep heap frontier.
            let (left, left_leaf) = fixture.chain(16_384, true)?;
            let (equal, equal_leaf) = fixture.chain(16_384, true)?;
            let (late_change, changed_leaf) = fixture.chain(16_385, true)?;
            assert!(!left_leaf.ptr_eq(&equal_leaf));
            expect_bool(
                invoke(
                    &fixture,
                    reference,
                    "compare",
                    vec![left.clone(), equal.clone()],
                    NormalizedRunPolicy::foreground(),
                    &control,
                    None,
                )?,
                true,
            )?;
            expect_bool(
                invoke(
                    &fixture,
                    reference,
                    "compare",
                    vec![left.clone(), late_change.clone()],
                    NormalizedRunPolicy::foreground(),
                    &control,
                    None,
                )?,
                false,
            )?;
            expect_bool(
                invoke(
                    &fixture,
                    reference,
                    "compare",
                    vec![left.clone(), left.clone()],
                    NormalizedRunPolicy::foreground(),
                    &control,
                    None,
                )?,
                true,
            )?;
            drop((left, equal, late_change));
            for witness in [left_leaf, equal_leaf, changed_leaf] {
                assert_eq!(witness.strong_count(), 1, "all deep value owners released");
            }
        }
        Ok(())
    })
}

#[test]
fn semantic_equality_frontier_refusal_keeps_errors_and_exact_policy_retry() -> Check {
    let fixture = Fixture::new()?;
    on_small_stack(|| {
        let control = ExecutionControl::uncancelled();
        for reference in [false, true] {
            let (left, left_leaf) = fixture.chain(128, true)?;
            let (right, right_leaf) = fixture.chain(128, true)?;
            // The hooks fail individual temporary reservations, not physical OOM.
            // Their attempt counts are deliberately not equated across engines.
            for allowed in [0, 8] {
                let observed = if reference {
                    let _refusal = reference::semantic_equality::refuse_frontier_after(allowed);
                    invoke(
                        &fixture,
                        true,
                        "compare",
                        vec![left.clone(), right.clone()],
                        NormalizedRunPolicy::foreground(),
                        &control,
                        None,
                    )?
                } else {
                    let _refusal = vm::semantic_equality::refuse_frontier_after(allowed);
                    invoke(
                        &fixture,
                        false,
                        "compare",
                        vec![left.clone(), right.clone()],
                        NormalizedRunPolicy::foreground(),
                        &control,
                        None,
                    )?
                };
                assert_eq!(observed.external_calls, 1);
                let error = observed
                    .value
                    .err()
                    .ok_or("frontier refusal became a value")?;
                assert_eq!(error.class, ExecutionFailureClass::Resource);
                assert_eq!(
                    error.code,
                    if reference {
                        "normalized_reference_semantic_comparison_allocation"
                    } else {
                        "normalized_semantic_comparison_allocation"
                    }
                );
                expect_bool(
                    invoke(
                        &fixture,
                        reference,
                        "compare",
                        vec![left.clone(), right.clone()],
                        NormalizedRunPolicy::foreground(),
                        &control,
                        None,
                    )?,
                    true,
                )?;
            }
            let bytes = expect_bool(
                invoke(
                    &fixture,
                    reference,
                    "compare",
                    vec![left.clone(), right.clone()],
                    NormalizedRunPolicy::foreground(),
                    &control,
                    None,
                )?,
                true,
            )?;
            assert!(bytes > 0);
            for allowance in [bytes, bytes - 1] {
                let observed = invoke(
                    &fixture,
                    reference,
                    "compare",
                    vec![left.clone(), right.clone()],
                    NormalizedRunPolicy {
                        maximum_allocated_bytes: Some(allowance),
                        ..NormalizedRunPolicy::foreground()
                    },
                    &control,
                    None,
                )?;
                if allowance == bytes {
                    assert_eq!(expect_bool(observed, true)?, bytes);
                } else {
                    assert_eq!(
                        observed
                            .value
                            .err()
                            .ok_or("one-short policy returned a value")?
                            .class,
                        ExecutionFailureClass::Resource
                    );
                }
            }
            expect_bool(
                invoke(
                    &fixture,
                    reference,
                    "compare",
                    vec![left.clone(), right.clone()],
                    NormalizedRunPolicy::foreground(),
                    &control,
                    None,
                )?,
                true,
            )?;
            drop((left, right));
            assert_eq!(left_leaf.strong_count(), 1);
            assert_eq!(right_leaf.strong_count(), 1);
        }
        Ok(())
    })
}

#[test]
fn semantic_equality_cancels_inside_core_after_admission_and_retries_cleanly() -> Check {
    let fixture = Fixture::new()?;
    on_small_stack(|| {
        for reference in [false, true] {
            let (left, left_leaf) = fixture.chain(4_096, true)?;
            let (right, right_leaf) = fixture.chain(4_096, true)?;
            let host = ComparisonHost {
                entered: AtomicUsize::new(0),
            };
            // Ingress uses an uncancelled control. The host delegates to the
            // ordinary Core implementation with a fresh comparison-only budget;
            // 64 successful checks exceed its single entry checkpoint.
            let observed = invoke(
                &fixture,
                reference,
                "compare",
                vec![left.clone(), right.clone()],
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled(),
                Some(&host),
            )?;
            assert_eq!(host.entered.load(Ordering::Relaxed), 1);
            assert_eq!(observed.external_calls, 1);
            let error = observed
                .value
                .err()
                .ok_or("comparison cancellation became a value")?;
            assert_eq!(error.class, ExecutionFailureClass::Cancelled);
            assert_eq!(error.code, "execution_cancelled");
            expect_bool(
                invoke(
                    &fixture,
                    reference,
                    "compare",
                    vec![left.clone(), right.clone()],
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::uncancelled(),
                    None,
                )?,
                true,
            )?;
            drop((left, right));
            assert_eq!(left_leaf.strong_count(), 1);
            assert_eq!(right_leaf.strong_count(), 1);
        }
        Ok(())
    })
}

fn compare_raw(
    reference: bool,
    observation: bool,
    left: &NormalizedValue,
    right: &NormalizedValue,
) -> Result<bool, ExecutionError> {
    let control = ExecutionControl::uncancelled();
    match (reference, observation) {
        (false, false) => vm::normalized_equal_with_control(left, right, &control),
        (false, true) => vm::normalized_observation_equal_with_control(left, right, &control),
        (true, false) => reference::reference_equal_with_control(left, right, &control),
        (true, true) => reference::reference_observation_equal_with_control(left, right, &control),
    }
}

#[test]
fn semantic_equality_visits_incomparable_suffixes_and_keeps_numeric_observation_distinct() -> Check
{
    let fixture = Fixture::new()?;
    on_small_stack(|| {
        let callable = NormalizedValue::Function {
            function: fixture
                .program
                .function(declaration_named(&fixture.source, "leaf"))
                .ok_or("leaf function index")?,
            type_arguments: Arc::from([]),
            effect_arguments: Arc::from([]),
            requirement_arguments: Arc::from([]),
            read_scope_arguments: Arc::from([]),
            bound_arguments: None,
        };
        let mut hidden = callable.clone();
        for _ in 0..4_096 {
            hidden = NormalizedValue::list(vec![hidden]).map_err(failure)?;
        }
        let left = NormalizedValue::list(vec![NormalizedValue::I64(1), hidden]).map_err(failure)?;
        let right = NormalizedValue::list(vec![NormalizedValue::I64(2), NormalizedValue::I64(0)])
            .map_err(failure)?;
        let short = NormalizedValue::list(vec![NormalizedValue::I64(2)]).map_err(failure)?;
        for reference in [false, true] {
            // Raw comparability probes intentionally bypass typed ingress:
            // a Function in Node would correctly fail before reaching equality.
            for (a, b) in [
                (&callable, &callable),
                (&left, &right),
                (&right, &left),
                (&left, &short),
                (&short, &left),
            ] {
                let error = compare_raw(reference, false, a, b)
                    .err()
                    .ok_or("an incomparable Function became a Boolean")?;
                assert_eq!(error.class, ExecutionFailureClass::Trap);
                assert_eq!(
                    error.code,
                    if reference {
                        "normalized_reference_value_not_comparable"
                    } else {
                        "normalized_value_not_comparable"
                    }
                );
            }
            for (left_bits, right_bits, semantic, observation) in [
                (0_u64, 1_u64 << 63, true, false),
                (0x7ff8_0000_0000_0000, 0x7ff8_0000_0000_0000, false, true),
            ] {
                let a = NormalizedValue::F64(
                    Binary64::from_bits(left_bits).ok_or("left F64 bits must be canonical")?,
                );
                let b = NormalizedValue::F64(
                    Binary64::from_bits(right_bits).ok_or("right F64 bits must be canonical")?,
                );
                assert_eq!(compare_raw(reference, false, &a, &b), Ok(semantic));
                assert_eq!(compare_raw(reference, true, &a, &b), Ok(observation));
            }
            assert_eq!(
                compare_raw(
                    reference,
                    false,
                    &NormalizedValue::I64(7),
                    &NormalizedValue::I64(7)
                ),
                Ok(true)
            );
        }
        drop((left, right, short, callable));
        Ok(())
    })
}

#[test]
fn semantic_equality_region_pointers_compare_coordinates_and_preserve_custody() -> Check {
    check_typed_region_pointer_equality()?;
    use crate::platform::execution::region::runtime::Arena;
    use crate::platform::kernel::RegionMode;

    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/region-expressions.lkjc"
    ))?;
    let program = prepare_snapshot(&source);
    let schema =
        super::super::NormalizedReferenceSchema::reconstruct([&source]).map_err(failure)?;
    let scalar = declaration_named(&source, "scalar");
    let brand = program
        .types
        .iter()
        .find_map(|(identity, object)| {
            matches!(object.form, TypeForm::RegionBrand { callable, .. } if callable == scalar)
                .then_some(*identity)
        })
        .ok_or("scalar's lexical region brand is missing")?;
    let cap_type = program.types.iter().find_map(|(identity, object)| {
        matches!(object.form, TypeForm::RegionCap { region, mode: RegionMode::Unique } if region == brand)
            .then_some(*identity)
    }).ok_or("scalar's unique capability type is missing")?;
    let (pointer_type, item) = program
        .types
        .iter()
        .find_map(|(identity, object)| match object.form {
            TypeForm::RelPtr { region, item } if region == brand => Some((*identity, item)),
            _ => None,
        })
        .ok_or("scalar's allocated pointer type is missing")?;
    assert!(matches!(
        program.types.get(&item).map(|object| &object.form),
        Some(TypeForm::I64)
    ));
    let other_type = program
        .types
        .iter()
        .find_map(|(identity, object)| match object.form {
            TypeForm::RelPtr {
                region,
                item: other_item,
            } if region != brand
                && other_item == item
                && matches!(
                    program.types.get(&region).map(|object| &object.form),
                    Some(TypeForm::RegionBrand { .. })
                ) =>
            {
                Some(*identity)
            }
            _ => None,
        })
        .ok_or("another lexical pointer type is missing")?;
    for properties in [&program.comparable_types, &schema.comparable_types] {
        assert!(properties.contains(&pointer_type));
        assert!(properties.contains(&other_type));
        assert!(!properties.contains(&cap_type));
    }

    on_small_stack(|| {
        let control = ExecutionControl::uncancelled();
        let origin = program.value_origin;
        let layout = super::super::region_value::Layout::derive(
            &program.types,
            item,
            brand,
            &control,
            &mut |_| Ok(()),
        )
        .map_err(failure)?;
        let arena = Arena::new().map_err(failure)?;
        let foreign_arena = Arena::new().map_err(failure)?;
        let allocate = |arena: &Arena<_>, token| {
            arena
                .allocate(
                    token,
                    origin,
                    layout.storage,
                    &73_i64.to_le_bytes(),
                    &control,
                )
                .map_err(failure)
        };
        let cap = arena
            .open(origin, 512, &control, |_| Ok(()))
            .map_err(failure)?;
        let identity = cap.identity();
        let pointer = allocate(&arena, &cap)?;
        let resolved = arena
            .resolve(&cap, origin, layout.storage, pointer.offset(), &control)
            .map_err(failure)?;
        let next_cell = allocate(&arena, &cap)?;
        assert_ne!(pointer.offset(), next_cell.offset());

        let other_cap = arena
            .open(origin, 512, &control, |_| Ok(()))
            .map_err(failure)?;
        let other_identity = other_cap.identity();
        let other_slot = allocate(&arena, &other_cap)?;
        assert_ne!(identity, other_identity);
        assert_eq!(pointer.offset(), other_slot.offset());
        let foreign_cap = foreign_arena
            .open(origin, 512, &control, |_| Ok(()))
            .map_err(failure)?;
        let foreign_identity = foreign_cap.identity();
        let foreign = allocate(&foreign_arena, &foreign_cap)?;
        assert_ne!(identity, foreign_identity);
        assert_eq!(pointer.offset(), foreign.offset());

        // Reopening the only freed slot creates a fresh generation. The old
        // pointer remains an inert coordinate observation and is never read.
        arena.close(other_identity).map_err(failure)?;
        let replacement_cap = arena
            .open(origin, 512, &control, |_| Ok(()))
            .map_err(failure)?;
        let replacement_identity = replacement_cap.identity();
        let replacement = allocate(&arena, &replacement_cap)?;
        assert_ne!(other_identity, replacement_identity);
        assert_eq!(other_slot.offset(), replacement.offset());
        let address = |pointer| NormalizedValue::RegionPointer {
            ty: pointer_type,
            pointer,
        };
        let capability = NormalizedValue::RegionCap {
            ty: cap_type,
            token: cap,
        };
        let first = address(pointer);
        let resolved = address(resolved);
        let next_cell = address(next_cell);
        let other_slot = address(other_slot);
        let replacement = address(replacement);
        let foreign = address(foreign);
        // This deliberate raw type mismatch is not an admitted alternate read.
        let other_type = NormalizedValue::RegionPointer {
            ty: other_type,
            pointer,
        };

        for reference in [false, true] {
            for observation in [false, true] {
                for (left, right, expected) in [
                    (&first, &first, true),
                    (&first, &resolved, true),
                    (&first, &next_cell, false),
                    (&first, &other_slot, false),
                    (&first, &foreign, false),
                    (&other_slot, &replacement, false),
                    (&first, &other_type, false),
                ] {
                    assert_eq!(
                        compare_raw(reference, observation, left, right),
                        Ok(expected)
                    );
                    assert_eq!(
                        compare_raw(reference, observation, right, left),
                        Ok(expected)
                    );
                }
                let error = compare_raw(reference, observation, &capability, &capability)
                    .err()
                    .ok_or("a unique capability became a Boolean")?;
                assert_eq!(error.class, ExecutionFailureClass::Trap);
                assert_eq!(
                    error.code,
                    if reference {
                        "normalized_reference_value_not_comparable"
                    } else {
                        "normalized_value_not_comparable"
                    }
                );
            }
        }
        let NormalizedValue::RegionCap { token, .. } = &capability else {
            return Err("live capability changed kind".to_owned());
        };
        arena.validate(token, origin, true).map_err(failure)?;
        arena
            .write(
                token,
                origin,
                layout.storage,
                pointer,
                &74_i64.to_le_bytes(),
                &control,
            )
            .map_err(failure)?;
        arena
            .read(token, origin, layout.storage, pointer, &control, |bytes| {
                assert_eq!(bytes, &74_i64.to_le_bytes());
                Ok(())
            })
            .map_err(failure)?;
        arena
            .validate(&replacement_cap, origin, true)
            .map_err(failure)?;
        foreign_arena
            .validate(&foreign_cap, origin, true)
            .map_err(failure)?;
        arena.close(identity).map_err(failure)?;
        arena.close(replacement_identity).map_err(failure)?;
        foreign_arena.close(foreign_identity).map_err(failure)?;
        assert!(arena.is_empty() && foreign_arena.is_empty());
        Ok(())
    })
}

const REGION_EQUALITY_SOURCE: &str = r#"declarations.begin
(units (module create region-equality
  (external create same-pointer (visibility private) (implementation core.value.equal)
    (type-parameter create T)
    (parameter create left (type T))
    (parameter create right (type T)) (returns Bool))
  (function create pointer-choice (visibility private) (effect pure)
    (parameter create different (type Bool)) (returns I64)
    (body (region-with
      (binding arena (as $region)
        (type (region-cap (region-brand pointer-choice $region) unique)))
      (i64 512)
      (in (let
        (binding first (type (rel-ptr (region-brand pointer-choice $region) I64))
          (region-alloc (type (rel-ptr (region-brand pointer-choice $region) I64)) arena (i64 73)))
        (binding second (type (rel-ptr (region-brand pointer-choice $region) I64))
          (region-alloc (type (rel-ptr (region-brand pointer-choice $region) I64)) arena (i64 73)))
        (binding selected (if (local different) (local second) (local first)))
        (binding equal (call same-pointer (types (rel-ptr (region-brand pointer-choice $region) I64))
          (local first) (local selected)))
        (in (sequence
          (region-write (type (rel-ptr (region-brand pointer-choice $region) I64)) arena (local second) (i64 74))
          (if (local equal)
            (region-read (type (rel-ptr (region-brand pointer-choice $region) I64)) arena (local first))
            (region-read (type (rel-ptr (region-brand pointer-choice $region) I64)) arena (local second))))))))))))
declarations.end
"#;

fn check_typed_region_pointer_equality() -> Check {
    let source = byte_buffer_tests::author_only(REGION_EQUALITY_SOURCE)?;
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let declaration = declaration_named(&source, "pointer-choice");
    on_small_stack(|| {
        for reference in [false, true] {
            for different in [false, true] {
                let arguments = vec![NormalizedValue::Bool(different)];
                let policy = NormalizedRunPolicy::foreground();
                let control = ExecutionControl::uncancelled();
                let (value, calls) = if reference {
                    let (value, observed) =
                        NormalizedReferenceInterpreter::new(&source, &program, policy)
                            .invoke(declaration, arguments, None, &control)
                            .map_err(failure)?;
                    (value, observed.external_calls)
                } else {
                    let (value, observed) = NormalizedVm::for_test(&program, policy)
                        .invoke(declaration, arguments, None, &control)
                        .map_err(failure)?;
                    (value, observed.external_calls)
                };
                assert_eq!(calls, 1, "typed source must execute pointer equality");
                let NormalizedValue::I64(value) = value else {
                    return Err("region equality program did not return its live cell".to_owned());
                };
                // Both cells held 73 at comparison time. The second is written
                // afterward, so this checks coordinate equality and live custody.
                assert_eq!(value, if different { 74 } else { 73 });
            }
        }
        Ok(())
    })
}
