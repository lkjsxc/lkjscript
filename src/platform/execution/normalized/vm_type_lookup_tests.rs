//! Compare prepared acceleration with the original independent structural scan.
use super::*;
use crate::platform::execution::normalized::tests::byte_buffer_tests;
use crate::platform::execution::normalized::{
    prepare::NormalizedEntryPoint,
    prepared_types::PreparedTypeLookup,
    vm::{NormalizedRunPolicy, NormalizedVm},
};
use crate::platform::kernel::{
    DeclarationReference, Name, OwnerKey, OwnerRecord, ReadScopeArgument, ReadScopeBinder,
    ReadScopeParameter, ReadScopeReference, StructuralTypeField, TypeObject, encode_type_object,
};
use crate::platform::semantic_id::{BindingId, ReadScopeParameterId};
use std::sync::{Arc, OnceLock};

const SOURCE: &str = r#"declarations.begin
(units (module create type-lookup
  (type-alias Ints (owned-sequence (option I64)))
  (type-alias Flags (owned-sequence (option Bool)))
  (type-alias IntOutcome (owned-choice (case empty Ints)
    (case item (owned-product (field rest Ints) (field value (option I64))))))
  (type-alias FlagOutcome (owned-choice (case empty Flags)
    (case item (owned-product (field rest Flags) (field value (option Bool))))))
  (function create carry (visibility private) (effect pure)
    (type-parameter create T (constraint transferable))
    (type-parameter create Tag (constraint transferable))
    (parameter create input (type (owned-choice
      (case empty (owned-sequence (option T)))
      (case item (owned-product (field rest (owned-sequence (option T)))
        (field value (option T)))))) (use consume))
    (returns (owned-choice (case empty (owned-sequence (option T)))
      (case item (owned-product (field rest (owned-sequence (option T)))
        (field value (option T)))))) (body (local input)))
  (function create other-scope (visibility private) (effect pure)
    (type-parameter create T (constraint transferable))
    (type-parameter create Tag (constraint transferable))
    (parameter create input (type (owned-choice
      (case empty (owned-sequence (option T)))
      (case item (owned-product (field rest (owned-sequence (option T)))
        (field value (option T)))))) (use consume))
    (returns (owned-choice (case empty (owned-sequence (option T)))
      (case item (owned-product (field rest (owned-sequence (option T)))
        (field value (option T)))))) (body (local input)))
  (function create root (visibility public) (effect pure) (returns Unit)
    (body (let
      (binding ints (type Ints) (sequence-empty (type Ints)))
      (binding input (type IntOutcome) (choose-owned (type IntOutcome) (case empty) (local ints)))
      (binding output (type IntOutcome) (call carry (types I64 Bool) (local input)))
      (binding output (type IntOutcome) (call other-scope (types I64 Bool) (local output)))
      (binding flags (type Flags) (sequence-empty (type Flags)))
      (binding input (type FlagOutcome) (choose-owned (type FlagOutcome) (case empty) (local flags)))
      (binding output (type FlagOutcome) (call carry (types Bool I64) (local input)))
      (in (unit)))))))
declarations.end"#;

fn fixture() -> (NormalizedProgram, BTreeMap<String, FunctionIndex>) {
    static PREPARED: OnceLock<(NormalizedProgram, BTreeMap<String, FunctionIndex>)> =
        OnceLock::new();
    PREPARED
        .get_or_init(|| {
            let source = byte_buffer_tests::author_only(SOURCE).unwrap();
            let temporary = tempfile::tempdir().unwrap();
            let repository = crate::platform::publication::GraphRepository::create(
                &temporary.path().join("lookup"),
                &source,
                None,
            )
            .unwrap()
            .repository;
            let prepared = crate::platform::normalized_lifecycle::prepare_repository(repository)
                .unwrap()
                .program;
            let functions = source
                .owners
                .iter()
                .filter_map(|(key, owner)| {
                    let (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(owner)) =
                        (key, owner)
                    else {
                        return None;
                    };
                    let function = prepared.function(DeclarationReference {
                        package: source.root.package_id,
                        declaration: *declaration,
                    })?;
                    Some((owner.name.to_string(), function))
                })
                .collect();
            ((*prepared).clone(), functions)
        })
        .clone()
}

fn scalar(program: &NormalizedProgram, form: TypeForm) -> TypeObjectDigest {
    program
        .types
        .iter()
        .find_map(|(ty, object)| (object.form == form).then_some(*ty))
        .unwrap()
}

fn bindings(
    program: &NormalizedProgram,
    function: FunctionIndex,
    data: TypeForm,
    tag: TypeForm,
) -> BTreeMap<TypeParameterId, TypeObjectDigest> {
    let parameters = &program.functions[function.0 as usize].type_parameters;
    BTreeMap::from([
        (parameters[0], scalar(program, data)),
        (parameters[1], scalar(program, tag)),
    ])
}

#[test]
fn prepared_type_lookup_matches_scan_for_nested_composites_and_distinct_scopes() {
    let (program, functions) = fixture();
    let control = ExecutionControl::uncancelled();
    let mut integer_results = Vec::new();
    for (name, data, tag) in [
        ("carry", TypeForm::I64, TypeForm::Bool),
        ("carry", TypeForm::Bool, TypeForm::I64),
        ("other-scope", TypeForm::I64, TypeForm::Bool),
    ] {
        let function = functions[name];
        let template = program.functions[function.0 as usize].result.ty;
        let context = bindings(&program, function, data.clone(), tag);
        let hit = program
            .prepared_type_lookup
            .get(program.value_origin, template, &context)
            .unwrap()
            .expect("normal admitted generic signature must be prepared");
        let mut cached = Work {
            control: &control,
            nodes: 0,
        };
        let mut scanned = Work {
            control: &control,
            nodes: 0,
        };
        assert_eq!(
            resolve_in(&program, template, &context, &mut cached).unwrap(),
            hit
        );
        assert_eq!(
            scan(&program, template, &context, &mut scanned).unwrap(),
            hit
        );
        assert!(cached.nodes < scanned.nodes);
        if data == TypeForm::I64 {
            integer_results.push(hit);
        }
    }
    assert_eq!(integer_results.len(), 2);
    assert_eq!(integer_results[0], integer_results[1]);
}

#[test]
fn prepared_type_lookup_full_context_includes_unused_bindings_without_cross_scope_reuse() {
    let (program, functions) = fixture();
    let function = functions["carry"];
    let template = program.functions[function.0 as usize].result.ty;
    let original = bindings(&program, function, TypeForm::I64, TypeForm::Bool);
    assert!(
        program
            .prepared_type_lookup
            .get(program.value_origin, template, &original)
            .unwrap()
            .is_some()
    );
    let changed_unused = bindings(&program, function, TypeForm::I64, TypeForm::I64);
    assert!(
        program
            .prepared_type_lookup
            .get(program.value_origin, template, &changed_unused)
            .unwrap()
            .is_none()
    );
    let foreign_scope = bindings(
        &program,
        functions["other-scope"],
        TypeForm::I64,
        TypeForm::Bool,
    );
    assert!(
        program
            .prepared_type_lookup
            .get(program.value_origin, template, &foreign_scope)
            .unwrap()
            .is_none()
    );
    let mut missing = original.clone();
    missing.remove(&program.functions[function.0 as usize].type_parameters[0]);
    assert!(
        program
            .prepared_type_lookup
            .get(program.value_origin, template, &missing)
            .unwrap()
            .is_none()
    );
    let mut extra = original.clone();
    extra.insert(
        TypeParameterId::migrate(b"foreign-context", 0),
        scalar(&program, TypeForm::I64),
    );
    assert!(
        program
            .prepared_type_lookup
            .get(program.value_origin, template, &extra)
            .unwrap()
            .is_none()
    );
    let control = ExecutionControl::uncancelled();
    let mut original_scan = Work {
        control: &control,
        nodes: 0,
    };
    let expected = scan(&program, template, &original, &mut original_scan).unwrap();
    // An absent unused-binding context preserves the old independent fallback;
    // it never borrows the cached context by projecting away unrelated formals.
    assert_eq!(
        resolve(&program, template, &changed_unused, &control).unwrap(),
        expected
    );
    assert!(resolve(&program, template, &foreign_scope, &control).is_err());
    assert!(resolve(&program, template, &missing, &control).is_err());
}

#[test]
fn prepared_type_lookup_rejects_foreign_origin_and_divergent_template_or_result_shapes() {
    let (base, functions) = fixture();
    let function = functions["carry"];
    let template = base.functions[function.0 as usize].result.ty;
    let context = bindings(&base, function, TypeForm::I64, TypeForm::Bool);
    let result = base
        .prepared_type_lookup
        .get(base.value_origin, template, &context)
        .unwrap()
        .unwrap();
    let control = ExecutionControl::uncancelled();
    for fault in 0..4 {
        let mut program = base.clone();
        match fault {
            0 => program.value_origin = ValueOrigin::fresh().unwrap(),
            1 => program.types.get_mut(&template).unwrap().form = TypeForm::I64,
            2 => program.types.get_mut(&result).unwrap().form = TypeForm::Bool,
            _ => {
                program.types.remove(&result);
            }
        }
        assert!(
            resolve(&program, template, &context, &control).is_err(),
            "fault={fault}"
        );
    }
    assert_eq!(
        resolve(&base, template, &context, &control).unwrap(),
        result
    );
}

#[test]
fn prepared_type_lookup_resolution_work_is_independent_of_unrelated_type_universe() {
    let (mut program, functions) = fixture();
    let function = functions["carry"];
    let template = program.functions[function.0 as usize].result.ty;
    let context = bindings(&program, function, TypeForm::I64, TypeForm::Bool);
    let control = ExecutionControl::uncancelled();
    let mut before = Work {
        control: &control,
        nodes: 0,
    };
    let result = resolve_in(&program, template, &context, &mut before).unwrap();
    let mut prior_before = Work {
        control: &control,
        nodes: 0,
    };
    assert_eq!(
        scan(&program, template, &context, &mut prior_before).unwrap(),
        result
    );
    let initial_types = program.types.len();
    let mut inserted_before_result = 0_u64;
    let integer = scalar(&program, TypeForm::I64);
    for number in 0..4096 {
        let object = TypeObject::new(TypeForm::StructuralRecord {
            fields: vec![StructuralTypeField {
                name: crate::platform::kernel::Name::new(format!("unrelated_{number:04}")).unwrap(),
                ty: integer,
            }],
        })
        .unwrap();
        let ty = encode_type_object(&object).unwrap().0;
        inserted_before_result += u64::from(ty < result);
        assert!(program.types.insert(ty, object).is_none());
    }
    assert_eq!(program.types.len(), initial_types + 4096);
    let mut after = Work {
        control: &control,
        nodes: 0,
    };
    assert_eq!(
        resolve_in(&program, template, &context, &mut after).unwrap(),
        result
    );
    assert_eq!(before.nodes, after.nodes);
    assert!(after.nodes < 20);
    let mut scanned = Work {
        control: &control,
        nodes: 0,
    };
    assert_eq!(
        scan(&program, template, &context, &mut scanned).unwrap(),
        result
    );
    assert_eq!(scanned.nodes, prior_before.nodes + inserted_before_result);
    assert!(scanned.nodes > after.nodes);
    println!(
        "prepared-type-lookup types-before={initial_types} types-after={} hit-shape-visits={} scan-before={} inserted-before-result={inserted_before_result} scan-after={}",
        program.types.len(),
        after.nodes,
        prior_before.nodes,
        scanned.nodes
    );
}

#[test]
fn prepared_type_lookup_hits_remain_cancellable_and_leave_subsequent_resolution_healthy() {
    let (program, functions) = fixture();
    let function = functions["carry"];
    let template = program.functions[function.0 as usize].result.ty;
    let context = bindings(&program, function, TypeForm::I64, TypeForm::Bool);
    let healthy = ExecutionControl::uncancelled();
    let expected = resolve(&program, template, &context, &healthy).unwrap();
    let mut failures = 0;
    let mut successes = 0;
    for checks in 0..24 {
        match resolve(
            &program,
            template,
            &context,
            &ExecutionControl::cancel_after_checks(checks),
        ) {
            Ok(actual) => {
                successes += 1;
                assert_eq!(actual, expected);
            }
            Err(error) => {
                failures += 1;
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
            }
        }
        assert_eq!(
            resolve(&program, template, &context, &healthy).unwrap(),
            expected
        );
    }
    assert!(failures > 0 && successes > 0);
}

// The actual fallback used before ordinary prepared entries were consulted.
// Callers separately assert that the exact application entry is absent.
fn prior_application_scan(
    program: &NormalizedProgram,
    template: TypeObjectDigest,
    context: &PreparedApplicationContext<'_>,
    work: &mut Work<'_>,
) -> Result<TypeObjectDigest, ExecutionError> {
    work.control.check()?;
    if contains_associated(program, template, 0, work)? {
        return Err(reject());
    }
    scan_context(
        program,
        template,
        TypeContext {
            ordinary: OrdinaryBindings::Map(context.ordinary()),
            associated: *context.environment(),
            scopes: application_scopes(program, context)?,
        },
        work,
    )
}

#[test]
fn application_ordinary_lookup_bounds_real_prior_scan_work_without_type_universe_growth() {
    let (mut program, functions) = fixture();
    let function = functions["carry"];
    let template = program.functions[function.0 as usize].result.ty;
    let ordinary = bindings(&program, function, TypeForm::I64, TypeForm::Bool);
    let control = ExecutionControl::uncancelled();
    assert_eq!(
        program.prepared_type_lookup.get_application(
            program.value_origin,
            function,
            template,
            &ordinary
        ),
        Ok(None)
    );
    let expected = program
        .prepared_type_lookup
        .get(program.value_origin, template, &ordinary)
        .unwrap()
        .unwrap();
    let mut before = Work {
        control: &control,
        nodes: 0,
    };
    let mut prior_before = Work {
        control: &control,
        nodes: 0,
    };
    {
        let context =
            PreparedApplicationContext::new(&program, function, &ordinary, &control).unwrap();
        assert_eq!(
            resolve_application_in(&program, template, &context, &mut before).unwrap(),
            expected
        );
        assert_eq!(
            prior_application_scan(&program, template, &context, &mut prior_before).unwrap(),
            expected
        );
    }
    let initial_types = program.types.len();
    let mut inserted_before_expected = 0_u64;
    let integer = scalar(&program, TypeForm::I64);
    for number in 0..4096 {
        let object = TypeObject::new(TypeForm::StructuralRecord {
            fields: vec![StructuralTypeField {
                name: Name::new(format!("application_unrelated_{number:04}")).unwrap(),
                ty: integer,
            }],
        })
        .unwrap();
        let digest = encode_type_object(&object).unwrap().0;
        inserted_before_expected += u64::from(digest < expected);
        assert!(program.types.insert(digest, object).is_none());
    }
    let retained_types = program.types.len();
    assert_eq!(retained_types, initial_types + 4096);
    let preparation_work = program.work;
    let context = PreparedApplicationContext::new(&program, function, &ordinary, &control).unwrap();
    let mut after = Work {
        control: &control,
        nodes: 0,
    };
    let mut scanned = Work {
        control: &control,
        nodes: 0,
    };
    assert_eq!(
        resolve_application_in(&program, template, &context, &mut after).unwrap(),
        expected
    );
    assert_eq!(
        prior_application_scan(&program, template, &context, &mut scanned).unwrap(),
        expected
    );
    assert_eq!(before.nodes, after.nodes);
    assert!(after.nodes < 40);
    // Canonical digest order determines how many added candidates the real
    // scan reaches. Each unrelated record before the winning owned choice
    // contributes exactly one failed root-form comparison; later keys add none.
    assert_eq!(scanned.nodes, prior_before.nodes + inserted_before_expected);
    assert!(scanned.nodes > after.nodes);
    assert_eq!(program.types.len(), retained_types);
    assert_eq!(program.work, preparation_work);
    println!(
        "application-ordinary-lookup types={retained_types} hit-visits={} prior-scan-before={} inserted-before-result={inserted_before_expected} prior-scan-after={}",
        after.nodes, prior_before.nodes, scanned.nodes
    );
}

#[test]
fn application_ordinary_lookup_preserves_binding_origin_and_advisory_shape_checks() {
    let (base, functions) = fixture();
    let function = functions["carry"];
    let template = base.functions[function.0 as usize].result.ty;
    let ordinary = bindings(&base, function, TypeForm::I64, TypeForm::Bool);
    let control = ExecutionControl::uncancelled();
    let cached = base
        .prepared_type_lookup
        .get(base.value_origin, template, &ordinary)
        .unwrap()
        .unwrap();
    let changed_unused = bindings(&base, function, TypeForm::I64, TypeForm::I64);
    let context =
        PreparedApplicationContext::new(&base, function, &changed_unused, &control).unwrap();
    assert_eq!(
        base.prepared_type_lookup
            .get(base.value_origin, template, &changed_unused),
        Ok(None)
    );
    assert_eq!(
        resolve_application(&base, template, &context, &control).unwrap(),
        cached
    );
    let mut missing = ordinary.clone();
    missing.remove(&base.functions[function.0 as usize].type_parameters[0]);
    assert!(PreparedApplicationContext::new(&base, function, &missing, &control).is_err());
    let foreign_bindings = bindings(
        &base,
        functions["other-scope"],
        TypeForm::I64,
        TypeForm::Bool,
    );
    assert!(PreparedApplicationContext::new(&base, function, &foreign_bindings, &control).is_err());
    let context = PreparedApplicationContext::new(&base, function, &ordinary, &control).unwrap();
    assert!(resolve_application(&base.clone(), template, &context, &control).is_err());
    let mut foreign = base.clone();
    foreign.value_origin = ValueOrigin::fresh().unwrap();
    assert!(PreparedApplicationContext::new(&foreign, function, &ordinary, &control).is_err());
    let foreign_function = FunctionIndex(function.0, foreign.value_origin);
    let foreign_context =
        PreparedApplicationContext::new(&foreign, foreign_function, &ordinary, &control).unwrap();
    assert!(resolve_application(&foreign, template, &foreign_context, &control).is_err());

    // A current shape inconsistent with the retained candidate must use the
    // original scan, which may find a different current structural solution.
    let mut changed = base.clone();
    let integer = scalar(&changed, TypeForm::I64);
    let replacement = TypeObject::new(TypeForm::OwnedSequence { item: integer }).unwrap();
    let expected = encode_type_object(&replacement).unwrap().0;
    changed.types.insert(expected, replacement);
    let parameter = changed.functions[function.0 as usize].type_parameters[0];
    let symbolic = scalar(&changed, TypeForm::TypeParameter { parameter });
    changed.types.get_mut(&template).unwrap().form = TypeForm::OwnedSequence { item: symbolic };
    let context = PreparedApplicationContext::new(&changed, function, &ordinary, &control).unwrap();
    let mut scanned = Work {
        control: &control,
        nodes: 0,
    };
    assert_ne!(expected, cached);
    assert_eq!(
        prior_application_scan(&changed, template, &context, &mut scanned).unwrap(),
        expected
    );
    assert_eq!(
        resolve_application(&changed, template, &context, &control).unwrap(),
        expected
    );
}

#[test]
fn application_ordinary_candidate_never_substitutes_its_own_read_scope() {
    let (mut program, functions) = fixture();
    let function = functions["carry"];
    let declaration = program.functions[function.0 as usize].declaration;
    let ordinary = bindings(&program, function, TypeForm::I64, TypeForm::Bool);
    let parameter = ReadScopeParameter {
        id: ReadScopeParameterId::migrate(b"ordinary-advisory-scope", 0),
        name: Name::new("loan").unwrap(),
    };
    let formal = ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(declaration),
        parameter: parameter.id,
    });
    let scopes = [0, 1].map(|number| ReadScopeArgument::Lexical {
        callable: declaration,
        binding: BindingId::migrate(b"ordinary-advisory-scope", number),
    });
    let integer = scalar(&program, TypeForm::I64);
    let mut intern = |read_scope| {
        let object = TypeObject::new(TypeForm::ReadViewApplied {
            declaration,
            arguments: vec![integer],
            read_scope,
        })
        .unwrap();
        let digest = encode_type_object(&object).unwrap().0;
        program.types.insert(digest, object);
        digest
    };
    let template = intern(formal);
    let retained = intern(scopes[0]);
    let alternate = intern(scopes[1]);
    let control = ExecutionControl::uncancelled();
    // An ordinary advisory key intentionally has no scope component. The
    // runtime reader must validate it against the actual function's scope.
    program.prepared_type_lookup = PreparedTypeLookup::ordinary_for_test(
        program.value_origin,
        template,
        retained,
        &ordinary,
        &control,
    )
    .unwrap();
    for (scope, expected) in [(scopes[0], retained), (scopes[1], alternate)] {
        let target = &mut Arc::make_mut(&mut program.functions)[function.0 as usize];
        target.read_scope_parameters = Arc::from([parameter.clone()]);
        target.read_scope_arguments = Arc::from([scope]);
        let context =
            PreparedApplicationContext::new(&program, function, &ordinary, &control).unwrap();
        let mut scanned = Work {
            control: &control,
            nodes: 0,
        };
        assert_eq!(
            prior_application_scan(&program, template, &context, &mut scanned).unwrap(),
            expected
        );
        assert_eq!(
            resolve_application(&program, template, &context, &control).unwrap(),
            expected
        );
    }
    Arc::make_mut(&mut program.functions)[function.0 as usize].read_scope_parameters =
        Arc::from([ReadScopeParameter {
            id: ReadScopeParameterId::migrate(b"foreign-advisory-scope", 0),
            name: Name::new("foreign").unwrap(),
        }]);
    let context = PreparedApplicationContext::new(&program, function, &ordinary, &control).unwrap();
    assert!(resolve_application(&program, template, &context, &control).is_err());
}

#[test]
fn application_ordinary_lookup_keeps_control_bounds_and_modeled_vm_accounting() {
    let (program, functions) = fixture();
    let function = functions["carry"];
    let template = program.functions[function.0 as usize].result.ty;
    let ordinary = bindings(&program, function, TypeForm::I64, TypeForm::Bool);
    let healthy = ExecutionControl::uncancelled();
    let context = PreparedApplicationContext::new(&program, function, &ordinary, &healthy).unwrap();
    let mut measured = Work {
        control: &healthy,
        nodes: 0,
    };
    let expected = resolve_application_in(&program, template, &context, &mut measured).unwrap();
    for short in [false, true] {
        let mut work = Work {
            control: &healthy,
            nodes: MAXIMUM_ADMISSION_ITEMS - measured.nodes + u64::from(short),
        };
        let result = resolve_application_in(&program, template, &context, &mut work);
        if short {
            assert_eq!(result.unwrap_err().class, ExecutionFailureClass::Resource);
        } else {
            assert_eq!(result.unwrap(), expected);
        }
    }
    let mut refused = 0;
    let mut accepted = 0;
    for checks in 0..128 {
        match resolve_application(
            &program,
            template,
            &context,
            &ExecutionControl::cancel_after_checks(checks),
        ) {
            Ok(actual) => {
                accepted += 1;
                assert_eq!(actual, expected);
            }
            Err(error) => {
                refused += 1;
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
            }
        }
        assert_eq!(
            resolve_application(&program, template, &context, &healthy).unwrap(),
            expected
        );
    }
    assert!(refused > 0 && accepted > 0);
    let mut scanned_program = program.clone();
    scanned_program.prepared_type_lookup = PreparedTypeLookup::empty(program.value_origin);
    let invoke = |prepared: &NormalizedProgram, policy| {
        NormalizedVm::for_test(prepared, policy).invoke_entry(
            NormalizedEntryPoint::Function(functions["root"]),
            vec![],
            None,
            &healthy,
        )
    };
    let (actual, counts) = invoke(&program, NormalizedRunPolicy::default()).unwrap();
    let (scanned, scanned_counts) =
        invoke(&scanned_program, NormalizedRunPolicy::default()).unwrap();
    assert_eq!(actual, NormalizedValue::Unit);
    assert_eq!(actual, scanned);
    assert_eq!(counts, scanned_counts);
    let policy = NormalizedRunPolicy {
        instruction_steps: Some(counts.instructions - 1),
        ..NormalizedRunPolicy::default()
    };
    let optimized_error = invoke(&program, policy).unwrap_err();
    let scanned_error = invoke(&scanned_program, policy).unwrap_err();
    assert_eq!(optimized_error.class, ExecutionFailureClass::Resource);
    assert_eq!(optimized_error.code, scanned_error.code);
    assert_eq!(optimized_error.class, scanned_error.class);
}
