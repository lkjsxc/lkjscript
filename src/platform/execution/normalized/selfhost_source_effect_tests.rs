//! E1 effect/requirement owner controls. No E2 substitution or execution claim.
use super::*;
use crate::platform::semantic_id::{DeclarationId, OperationId};

#[path = "selfhost_source_effect_owner_tests.rs"]
mod owners;
#[path = "selfhost_source_effect_scope_tests.rs"]
mod scope;
#[path = "selfhost_source_effect_shape_tests.rs"]
mod shape;

fn fixture() -> KernelSnapshot {
    static SOURCE: OnceLock<KernelSnapshot> = OnceLock::new();
    SOURCE
        .get_or_init(|| {
            byte_buffer_tests::author_only(include_str!(
                "../../../../tests/fixtures/selfhost-source-effect-owners.lkjc"
            ))
            .unwrap()
        })
        .clone()
}

fn declaration(source: &KernelSnapshot, name: &str) -> DeclarationId {
    source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.name.as_str() == name =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}

fn owner_key(source: &KernelSnapshot, select: impl Fn(&OwnerRecord) -> bool) -> OwnerKey {
    source
        .owners
        .iter()
        .find_map(|(key, owner)| select(owner).then_some(*key))
        .unwrap()
}

fn operation(source: &KernelSnapshot, parent: &str, name: &str) -> OperationId {
    let parent = declaration(source, parent);
    let key = owner_key(source, |owner| {
        matches!(owner, OwnerRecord::Operation(record)
        if record.declaration == parent && record.name.as_str() == name)
    });
    let OwnerKey::Operation(id) = key else {
        panic!("operation")
    };
    id
}

fn parameter_key(source: &KernelSnapshot, parent: kernel::ParameterParent, name: &str) -> OwnerKey {
    owner_key(source, |owner| {
        matches!(owner, OwnerRecord::Parameter(record)
        if record.parent == parent && record.name.as_str() == name)
    })
}

fn parameter_type(source: &KernelSnapshot, key: OwnerKey) -> TypeObjectDigest {
    let OwnerRecord::Parameter(record) = &source.owners[&key] else {
        panic!("parameter")
    };
    record.ty
}

fn set_parameter_type(source: &mut KernelSnapshot, key: OwnerKey, ty: TypeObjectDigest) {
    let OwnerRecord::Parameter(record) = source.owners.get_mut(&key).unwrap() else {
        panic!("parameter")
    };
    record.ty = ty;
}

fn function<'a>(source: &'a KernelSnapshot, name: &str) -> &'a kernel::FunctionDeclaration {
    let id = declaration(source, name);
    let OwnerRecord::Declaration(record) = &source.owners[&OwnerKey::Declaration(id)] else {
        panic!("declaration")
    };
    let kernel::DeclarationPayload::Function(function) = &record.payload else {
        panic!("function")
    };
    function
}

fn function_mut<'a>(
    source: &'a mut KernelSnapshot,
    name: &str,
) -> &'a mut kernel::FunctionDeclaration {
    let id = declaration(source, name);
    let OwnerRecord::Declaration(record) =
        source.owners.get_mut(&OwnerKey::Declaration(id)).unwrap()
    else {
        panic!("declaration")
    };
    let kernel::DeclarationPayload::Function(function) = &mut record.payload else {
        panic!("function")
    };
    function
}

fn intern(source: &mut KernelSnapshot, form: kernel::TypeForm) -> TypeObjectDigest {
    let object = kernel::TypeObject::new(form).unwrap();
    let digest = kernel::encode_type_object(&object).unwrap().0;
    source.types.insert(digest, object);
    digest
}

fn rebind(source: &mut KernelSnapshot) {
    let entries = source
        .owners
        .iter()
        .map(|(key, record)| {
            let (object, _) = kernel::encode_owner(record).unwrap();
            (
                kernel::owner_map_key(*key).to_vec(),
                kernel::encode_owner_binding(&kernel::OwnerBinding {
                    kind: record.kind(),
                    object,
                }),
            )
        })
        .collect::<Vec<_>>();
    let content = crate::platform::persistent_map::MapContentRoot::from_sorted(entries).unwrap();
    source.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        source.root.owners.page(),
        source.owners.len() as u64,
        content.digest(),
    );
}

// Only the Rust oracle uses this projection. Raw admission retains every descriptor.
fn live_type_oracle(source: &KernelSnapshot) -> KernelSnapshot {
    let mut oracle = source.clone();
    rebind(&mut oracle);
    let state = kernel::semantic_state_digest(&oracle).unwrap();
    let mut pending: Vec<_> = oracle
        .owners
        .values()
        .flat_map(OwnerRecord::type_roots)
        .collect();
    let mut reachable = std::collections::BTreeSet::new();
    while let Some(digest) = pending.pop() {
        if reachable.insert(digest)
            && let Some(object) = oracle.types.get(&digest)
        {
            pending.extend(object.child_types());
        }
    }
    oracle.types.retain(|digest, _| reachable.contains(digest));
    assert_eq!(oracle.owners, source.owners);
    assert_eq!(kernel::semantic_state_digest(&oracle).unwrap(), state);
    oracle
}

fn packet_result(name: &str, bytes: &[u8], code: i64) {
    assert_eq!(
        result(name, bytes),
        (
            code,
            if code == 0 {
                bytes[48..80].to_vec()
            } else {
                vec![]
            }
        ),
        "{name}"
    );
    emitted_result(name, bytes, code);
}

fn expect(source: &KernelSnapshot, code: i64, oracle_code: Option<&str>) {
    let mut source = source.clone();
    rebind(&mut source);
    let bytes = graph::packet(&source);
    let oracle = live_type_oracle(&source);
    if code == 0 {
        kernel::validate_full(&oracle).unwrap();
    } else {
        let errors = kernel::validate_full(&oracle).unwrap_err();
        assert!(
            errors.iter().any(
                |error| error.class == crate::platform::DiagnosticClass::Semantic
                    && oracle_code.is_none_or(|expected| error.code == expected)
            ),
            "{errors:?}"
        );
        assert!(
            errors.iter().all(
                |error| error.class != crate::platform::DiagnosticClass::Resource
                    && error.code != "kernel_full_map_count"
                    && error.code != "kernel_full_type_unreachable"
            ),
            "{errors:?}"
        );
    }
    let target = match code {
        0 => "read-nominal",
        13 | 14 => {
            packet_result("read-carrier", &bytes, 0);
            "read-graph"
        }
        15 => {
            packet_result("read-graph", &bytes, 0);
            "read-syntax"
        }
        16 => {
            packet_result("read-syntax", &bytes, 0);
            "read-namespace"
        }
        17 => {
            packet_result("read-namespace", &bytes, 0);
            packet_result("read-type-structure", &bytes, 0);
            "read-type-scope"
        }
        _ => panic!("unexpected E1 control"),
    };
    packet_result(target, &bytes, code);
}

fn name_mut(record: &mut OwnerRecord) -> &mut kernel::Name {
    match record {
        OwnerRecord::Module(value) => &mut value.name,
        OwnerRecord::Declaration(value) => &mut value.name,
        OwnerRecord::TypeParameter(value) => &mut value.name,
        OwnerRecord::EffectParameter(value) => &mut value.name,
        OwnerRecord::RequirementParameter(value) => &mut value.name,
        OwnerRecord::Field(value) => &mut value.name,
        OwnerRecord::Case(value) => &mut value.name,
        OwnerRecord::Operation(value) => &mut value.name,
        OwnerRecord::Parameter(value) => &mut value.name,
        OwnerRecord::Requirement(value) => &mut value.name,
        OwnerRecord::Port(value) => &mut value.name,
        OwnerRecord::Target(value) => &mut value.name,
        _ => panic!("namespace owner"),
    }
}

#[test]
fn selfhost_source_effect_owners_admit_independent_contextual_fixtures() {
    let source = fixture();
    assert_eq!(function(&source, "first").requirement_parameters.len(), 2);
    assert_eq!(function(&source, "first").effect_parameters.len(), 2);
    assert_eq!(function(&source, "generic-task").type_parameters.len(), 1);
    assert!(matches!(
        function(&source, "empty-task").effect,
        kernel::FunctionEffect::Task { .. }
    ));
    expect(&source, 0, None);
    let seed = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-function-binders.lkjc"
    ))
    .unwrap();
    expect(&seed, 0, None);
}

#[test]
fn selfhost_source_effect_owners_preserve_exact_composed_work_boundary() {
    exact_budget_boundary("read-type-scope-budget", &fixture());
}

#[test]
fn selfhost_source_effect_applications_remain_explicitly_unsupported() {
    for effect in [false, true] {
        for function_value in [false, true] {
            let binder = if effect {
                "(effect-parameter create E)"
            } else {
                "(requirement-parameter create R (interface Clock) (operations Clock::tick))"
            };
            let argument = if effect {
                "(effects (row))"
            } else {
                "(requirements R)"
            };
            let form = if function_value {
                "function-value"
            } else {
                "call"
            };
            let result = if function_value {
                "(function () I64)"
            } else {
                "I64"
            };
            let proposal = format!(
                "declarations.begin\n(units (module create application-boundary \
                (interface create Clock (visibility public) \
                  (operation create tick (returns I64) \
                    (idempotency idempotent) (external-visibility none))) \
                (function create callee (visibility public) (effect pure) \
                  {binder} (returns I64) (body (i64 42))) \
                (function create caller (visibility public) (effect pure) \
                  {binder} (returns {result}) (body ({form} callee {argument})))))\n\
                declarations.end\n"
            );
            let source = byte_buffer_tests::author_only(&proposal).unwrap();
            kernel::validate_full(&source).unwrap();
            let bytes = graph::packet(&source);
            packet_result("read-carrier", &bytes, 13);
        }
    }
}
