//! Compare prepared acceleration with the original independent structural scan.
use super::*;
use crate::platform::execution::normalized::tests::byte_buffer_tests;
use crate::platform::kernel::{
    DeclarationReference, OwnerKey, OwnerRecord, StructuralTypeField, TypeObject,
    encode_type_object,
};
use std::sync::OnceLock;

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
        let template = program.functions[function.0 as usize].result;
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
    let template = program.functions[function.0 as usize].result;
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
    let template = base.functions[function.0 as usize].result;
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
    let template = program.functions[function.0 as usize].result;
    let context = bindings(&program, function, TypeForm::I64, TypeForm::Bool);
    let control = ExecutionControl::uncancelled();
    let mut before = Work {
        control: &control,
        nodes: 0,
    };
    let result = resolve_in(&program, template, &context, &mut before).unwrap();
    let initial_types = program.types.len();
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
        program.types.insert(ty, object);
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
    assert!(scanned.nodes > 1000);
    println!(
        "prepared-type-lookup types-before={initial_types} types-after={} hit-shape-visits={} scan-visits={}",
        program.types.len(),
        after.nodes,
        scanned.nodes
    );
}

#[test]
fn prepared_type_lookup_hits_remain_cancellable_and_leave_subsequent_resolution_healthy() {
    let (program, functions) = fixture();
    let function = functions["carry"];
    let template = program.functions[function.0 as usize].result;
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
