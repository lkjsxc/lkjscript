//! Separate-package regressions for the independent canonical ownership oracle.
use super::super::*;
use crate::platform::persistent_map::{MapContentDigest, MapRoot, PageDigest};
use crate::platform::publication::{ExportedPackageTransport, GraphRepository};
use crate::platform::semantic_id::{DeclarationId, ExpressionId, RepositoryId};
use std::collections::BTreeMap;

const ABSTRACT: &str = include_str!("../../../tests/fixtures/owned-parameters-abstract.lkjc");
const WITNESSES: &str = include_str!("../../../tests/fixtures/owned-witness-library.lkjc");
const CELL: &str = include_str!("../../../tests/fixtures/owned-witness-cell.lkjc");
const TASK: &str = r#"declarations.begin
(units (module create task-library
  (function create relay-task (visibility public) (effect (task))
    (type-parameter create T (constraint owned))
    (parameter create value (type T) (use consume)) (returns T)
    (body (local value)))
  (function create read-task (visibility public) (effect (task))
    (type-parameter create T (constraint owned))
    (parameter create value (type T) (use borrow)) (returns Unit)
    (body (unit)))
  (function create mixed (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create a (type T) (use borrow))
    (parameter create b (type T) (use consume)) (returns Unit) (body (unit)))
  (function create mixed-reverse (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create a (type T) (use consume))
    (parameter create b (type T) (use borrow)) (returns Unit) (body (unit)))))
declarations.end
"#;

fn empty(seed: &[u8]) -> KernelSnapshot {
    let map = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(seed, 0),
            package_id: PackageId::migrate(seed, 0),
            package_name: Name::new("imported_oracle").unwrap(),
            owners: map,
            dependencies: map,
            retirements: map,
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    }
}

fn author(repository: &GraphRepository, source: &str) {
    let input = format!(
        "request base={}\n{source}",
        repository.view_current().unwrap().revision()
    );
    let request =
        crate::platform::control::decode_compact_change("imported-oracle", input.as_bytes())
            .unwrap();
    let prepared = repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    repository.publish(&prepared.publication).unwrap();
}

fn dependency(export: &ExportedPackageTransport) -> String {
    format!(
        "add.dependency package={} semantic-revision={} package-revision={}\n",
        export.revision.package,
        export.revision.revision.revision_id().unwrap(),
        export.revision_digest,
    )
}

fn imported() -> KernelSnapshot {
    let temporary = tempfile::tempdir().unwrap();
    let producer = GraphRepository::create(
        &temporary.path().join("producer"),
        &empty(b"oracle-producer"),
        None,
    )
    .unwrap()
    .repository;
    author(&producer, &format!("{ABSTRACT}\n{WITNESSES}\n{TASK}"));
    let producer_source = producer
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    assert!(super::accepts(&producer_source));
    let export = producer.export_package_transport().unwrap();
    let consumer = GraphRepository::create(
        &temporary.path().join("consumer"),
        &empty(b"oracle-consumer"),
        None,
    )
    .unwrap()
    .repository;
    consumer
        .stage_package_transport(export.transport_digest, &export.container)
        .unwrap();
    let imported_modules = ["abstract-memory", "abstraction", "task-library"]
        .map(|name| {
            format!(
                "(use {name} {} {})",
                export.revision.package, export.revision_digest
            )
        })
        .join(" ");
    let mut functions = String::new();
    for (name, ty) in [
        ("buffer-flow", "ByteBuffer"),
        ("cell-flow", "OwnedI64Cell"),
        (
            "product-flow",
            "(owned-product (field owner OwnedI64Cell) (field tag I64))",
        ),
        (
            "choice-flow",
            "(owned-choice (case accepted Unit) (case rejected OwnedI64Cell))",
        ),
    ] {
        functions.push_str(&format!(
            r#"
  (function create {name} (visibility public) (effect pure)
    (parameter create owner (type {ty}) (use consume)) (returns {ty})
    (body (sequence
      (call abstract-memory::read-twice (types {ty}) (local owner))
      (call abstract-memory::relay (types {ty}) (local owner)))))"#
        ));
    }
    author(
        &consumer,
        &format!(
            r#"{}declarations.begin
(units {imported_modules})
declarations.end
{CELL}
declarations.begin
(units (module create application
{functions}
  (function create symbolic-flow (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create owner (type T) (use consume)) (returns T)
    (body (sequence
      (call abstract-memory::read-twice (types T) (local owner))
      (call abstract-memory::relay (types T) (local owner)))))
  (function create task-flow (visibility public) (effect (task))
    (type-parameter create T (constraint owned))
    (parameter create owner (type T) (use consume)) (returns T)
    (body (sequence
      (call task-library::read-task (types T) (local owner))
      (call task-library::relay-task (types T) (local owner)))))
  (function create distinct-discard (visibility public) (effect pure)
    (parameter create a (type OwnedI64Cell) (use consume))
    (parameter create b (type OwnedI64Cell) (use consume)) (returns Unit)
    (body (sequence
      (call abstract-memory::discard (types OwnedI64Cell) (local a))
      (call abstract-memory::discard (types OwnedI64Cell) (local b)))))
  (function create distinct-mixed (visibility public) (effect pure)
    (parameter create a (type OwnedI64Cell) (use consume))
    (parameter create b (type OwnedI64Cell) (use consume)) (returns Unit)
    (body (call task-library::mixed (types OwnedI64Cell) (local a) (local b))))
  (function create distinct-mixed-reverse (visibility public) (effect pure)
    (parameter create a (type OwnedI64Cell) (use consume))
    (parameter create b (type OwnedI64Cell) (use consume)) (returns Unit)
    (body (call task-library::mixed-reverse (types OwnedI64Cell) (local a) (local b))))
  (function create selected-witness (visibility public) (effect pure) (returns I64)
    (body (let
      (binding owner (type OwnedI64Cell)
        (implementation-call abstraction::produce (types OwnedI64Cell)
          (implementations concrete@cell::Scalar) (i64 7)))
      (binding updated (type OwnedI64Cell)
        (implementation-call abstraction::transform (types OwnedI64Cell)
          (implementations concrete@cell::Scalar) (i64 11) (local owner)))
      (in (implementation-call abstraction::read-helper (types OwnedI64Cell)
        (implementations concrete@cell::Scalar) (local updated))))))
  (function create forwarded-witness (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_33000000000000000000000000000001 ops abstraction::Storage T)
    (parameter create owner (type T) (use consume)) (returns I64)
    (body (implementation-call abstraction::consume (types T)
      (implementations parameter@forwarded-witness@implparam_33000000000000000000000000000001) (local owner))))))
declarations.end
"#,
            dependency(&export)
        ),
    );
    let source = consumer
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    assert_ne!(source.root.package_id, export.revision.package);
    assert!(!source.dependency_interfaces.is_empty());
    for interface in source.dependency_interfaces.values() {
        for (key, owner) in interface {
            if matches!(owner, PackageInterfaceRecord::TypeParameter(p) if p.constraints == TypeParameterConstraints::Owned)
            {
                assert!(
                    !source.owners.contains_key(key),
                    "foreign formals must not be root owners"
                );
            }
        }
    }
    validate_full(&source).unwrap();
    source
}

fn declaration(source: &KernelSnapshot, name: &str) -> DeclarationId {
    source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Declaration(d) if d.name.as_str() == name => match d.header.owner {
                OwnerKey::Declaration(id) => Some(id),
                _ => None,
            },
            _ => None,
        })
        .unwrap()
}

fn calls(source: &KernelSnapshot, name: &str) -> Vec<ExpressionId> {
    source
        .owners
        .values()
        .filter_map(|owner| match owner {
            OwnerRecord::Expression(e) => match &e.operation {
                ExpressionOperation::Call { function, .. }
                | ExpressionOperation::ImplementationCall { function, .. } => {
                    if function.package == source.root.package_id {
                        return None;
                    }
                    let d = &source.dependency_interfaces
                        [&source.dependencies[&function.package].package_revision]
                        [&OwnerKey::Declaration(function.declaration)];
                    matches!(d, PackageInterfaceRecord::Declaration(d) if d.name.as_str() == name)
                        .then_some(e.id)
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn imported_memory_oracle_accepts_distinct_package_owned_consume_borrow_return_and_witnesses() {
    assert!(super::accepts(&imported()));
}

#[test]
fn imported_memory_oracle_rejects_duplicate_consumption_and_borrowed_return() {
    let source = imported();
    assert!(super::accepts(&source));
    let mut duplicate = source.clone();
    let discard = calls(&source, "discard");
    assert_eq!(discard.len(), 2);
    let OwnerRecord::Expression(first) = &source.owners[&OwnerKey::Expression(discard[0])] else {
        unreachable!()
    };
    let ExpressionOperation::Call { arguments, .. } = &first.operation else {
        unreachable!()
    };
    let OwnerRecord::Expression(second) = duplicate
        .owners
        .get_mut(&OwnerKey::Expression(discard[1]))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::Call {
        arguments: changed, ..
    } = &mut second.operation
    else {
        unreachable!()
    };
    *changed = arguments.clone();
    assert!(
        !super::accepts(&duplicate),
        "imported consume must invalidate parent rights"
    );

    let mut loan = source.clone();
    let id = declaration(&source, "cell-flow");
    let OwnerRecord::Declaration(d) = &source.owners[&OwnerKey::Declaration(id)] else {
        unreachable!()
    };
    let DeclarationPayload::Function(f) = &d.payload else {
        unreachable!()
    };
    let OwnerRecord::Parameter(p) = loan
        .owners
        .get_mut(&OwnerKey::Parameter(f.parameters[0]))
        .unwrap()
    else {
        unreachable!()
    };
    p.use_mode = ParameterUse::Borrow;
    assert!(
        !super::accepts(&loan),
        "borrowed callers cannot return an imported generic owner's result"
    );
}

#[test]
fn imported_memory_oracle_rejects_foreign_constraints_modes_and_open_actuals() {
    let source = imported();
    assert!(super::accepts(&source));
    for fault in [
        "unrestricted",
        "borrowed-result",
        "parameter-generation",
        "function-generation",
    ] {
        let mut invalid = source.clone();
        for interface in invalid.dependency_interfaces.values_mut() {
            let function = interface
                .values()
                .find_map(|owner| match owner {
                    PackageInterfaceRecord::Declaration(d) if d.name.as_str() == "read-task" => {
                        match &d.payload {
                            PackageInterfaceDeclarationPayload::Function(f) => Some(f.clone()),
                            _ => None,
                        }
                    }
                    _ => None,
                })
                .unwrap();
            let PackageInterfaceRecord::Parameter(p) = interface
                .get_mut(&OwnerKey::Parameter(function.parameters[0]))
                .unwrap()
            else {
                unreachable!()
            };
            match fault {
                "unrestricted" => p.use_mode = ParameterUse::Unrestricted,
                "parameter-generation" => {
                    p.header.contract_version = contract::SHARE_GRAPH_CONTRACT_VERSION - 1;
                }
                "borrowed-result" | "function-generation" => {
                    let result = p.ty;
                    let parameter = function.parameters[0];
                    let d = interface
                        .values_mut()
                        .find_map(|owner| match owner {
                            PackageInterfaceRecord::Declaration(d)
                                if d.name.as_str() == "read-task" =>
                            {
                                Some(d)
                            }
                            _ => None,
                        })
                        .unwrap();
                    if fault == "function-generation" {
                        d.header.contract_version = contract::SHARE_GRAPH_CONTRACT_VERSION - 1;
                    } else {
                        let PackageInterfaceDeclarationPayload::Function(f) = &mut d.payload else {
                            unreachable!()
                        };
                        f.result = result;
                        f.result_borrow = Some(parameter);
                    }
                }
                _ => unreachable!(),
            }
        }
        assert!(
            !super::accepts(&invalid),
            "imported task loans must retain valid modes, generations and result boundary: {fault}"
        );
    }
    for bound in [
        TypeParameterConstraints::OwnedTransferable,
        TypeParameterConstraints::OwnedShareable,
        TypeParameterConstraints::OwnedTransferableShareable,
    ] {
        let mut stronger = source.clone();
        for interface in stronger.dependency_interfaces.values_mut() {
            let parameter = interface
                .values()
                .find_map(|owner| match owner {
                    PackageInterfaceRecord::Declaration(d) if d.name.as_str() == "read-twice" => {
                        match &d.payload {
                            PackageInterfaceDeclarationPayload::Function(f) => {
                                Some(f.type_parameters[0])
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                })
                .unwrap();
            let PackageInterfaceRecord::TypeParameter(p) = interface
                .get_mut(&OwnerKey::TypeParameter(parameter))
                .unwrap()
            else {
                unreachable!()
            };
            p.constraints = bound;
        }
        assert!(
            !super::accepts(&stronger),
            "the symbolic owned-only actual cannot prove the imported {bound:?} bound"
        );
        let symbolic = declaration(&stronger, "symbolic-flow");
        for owner in stronger.owners.values_mut() {
            if let OwnerRecord::TypeParameter(p) = owner
                && p.declaration == symbolic
            {
                p.constraints = bound;
            }
        }
        assert!(
            super::accepts(&stronger),
            "an exact stronger caller proof must satisfy the imported {bound:?} bound"
        );
    }
    let mut constraint = source.clone();
    for interface in constraint.dependency_interfaces.values_mut() {
        for owner in interface.values_mut() {
            if let PackageInterfaceRecord::TypeParameter(p) = owner {
                p.constraints = TypeParameterConstraints::None;
            }
        }
    }
    assert!(
        !super::accepts(&constraint),
        "an actual owner cannot satisfy an ordinary imported formal"
    );

    let mut open = source.clone();
    let id = calls(&source, "relay")[0];
    let foreign = source
        .dependency_types
        .iter()
        .find_map(|(ty, object)| {
            matches!(object.form, TypeForm::TypeParameter { .. }).then_some(*ty)
        })
        .unwrap();
    let OwnerRecord::Expression(e) = open.owners.get_mut(&OwnerKey::Expression(id)).unwrap() else {
        unreachable!()
    };
    let ExpressionOperation::Call { type_arguments, .. } = &mut e.operation else {
        unreachable!()
    };
    *type_arguments = vec![foreign];
    assert!(
        !super::accepts(&open),
        "foreign formal identities are not caller-scoped actuals"
    );
}

#[test]
fn imported_memory_oracle_rejects_borrow_consume_alias_in_both_orders() {
    let source = imported();
    assert!(super::accepts(&source));
    for name in ["mixed", "mixed-reverse"] {
        let mut invalid = source.clone();
        let selected = calls(&source, name);
        assert_eq!(selected.len(), 1);
        let OwnerRecord::Expression(e) = invalid
            .owners
            .get_mut(&OwnerKey::Expression(selected[0]))
            .unwrap()
        else {
            unreachable!()
        };
        let ExpressionOperation::Call { arguments, .. } = &mut e.operation else {
            unreachable!()
        };
        arguments[1] = arguments[0];
        assert!(
            !super::accepts(&invalid),
            "imported alias must reject {name}"
        );
    }
}

#[test]
fn owned_parallel_memory_oracle_accepts_all_result_modes_and_rejects_double_moves() {
    for (left_owned, right_owned) in [(false, false), (false, true), (true, false), (true, true)] {
        let result = |owned| if owned { "OwnedI64Cell" } else { "I64" };
        let body = |owned| if owned { "(local owner)" } else { "(i64 7)" };
        let pair = if left_owned || right_owned {
            "owned-product (field left"
        } else {
            "record (left"
        };
        let right_field = if left_owned || right_owned {
            "field right"
        } else {
            "right"
        };
        let literal = format!(
            r#"declarations.begin
(units (module create result-proof
  (function create left (visibility public) (effect (task))
    (parameter create owner (type OwnedI64Cell) (use consume))
    (returns {}) (body {}))
  (function create right (visibility public) (effect (task))
    (parameter create owner (type OwnedI64Cell) (use consume))
    (returns {}) (body {}))
  (function create pair (visibility public) (effect (task))
    (parameter create a (type OwnedI64Cell) (use consume))
    (parameter create b (type OwnedI64Cell) (use consume))
    (returns ({pair} {}) ({right_field} {})))
    (body (parallel (call left (local a)) (call right (local b)))))))
declarations.end"#,
            result(left_owned),
            body(left_owned),
            result(right_owned),
            body(right_owned),
            result(left_owned),
            result(right_owned)
        );
        let source =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(&literal)
                .unwrap();
        assert!(
            super::accepts(&source),
            "left={left_owned}, right={right_owned}"
        );
        let mut invalid = source.clone();
        let (left, right) = source
            .owners
            .values()
            .find_map(|owner| match owner {
                OwnerRecord::Expression(e) => match e.operation {
                    ExpressionOperation::Parallel { left, right } => Some((left, right)),
                    _ => None,
                },
                _ => None,
            })
            .unwrap();
        let OwnerRecord::Expression(e) = &source.owners[&OwnerKey::Expression(left)] else {
            unreachable!()
        };
        let ExpressionOperation::Call { arguments, .. } = &e.operation else {
            unreachable!()
        };
        let OwnerRecord::Expression(e) = invalid
            .owners
            .get_mut(&OwnerKey::Expression(right))
            .unwrap()
        else {
            unreachable!()
        };
        let ExpressionOperation::Call {
            arguments: changed, ..
        } = &mut e.operation
        else {
            unreachable!()
        };
        *changed = arguments.clone();
        assert!(
            !super::accepts(&invalid),
            "parallel child arguments share parent affine rights"
        );
    }
}

#[test]
fn owned_parallel_inferred_pair_preserves_complete_wrapper_depth_admission() {
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create inferred-depth
  (function create relay (visibility private) (effect (task))
    (parameter create owner (type OwnedI64Cell) (use consume))
    (returns OwnedI64Cell) (body (local owner)))
  (function create ordinary (visibility private) (effect (task))
    (returns I64) (body (i64 7)))
  (function create discard-pair (visibility private) (effect (task))
    (parameter create owner (type OwnedI64Cell) (use consume))
    (returns Unit)
    (body (sequence
      (parallel (call relay (local owner)) (call ordinary))
      (unit))))))
declarations.end"#,
    )
    .unwrap();
    let scalar = source
        .types
        .iter()
        .find_map(|(digest, object)| {
            matches!(object.form, TypeForm::OwnedI64Cell).then_some(*digest)
        })
        .unwrap();
    let ordinary = source
        .types
        .iter()
        .find_map(|(digest, object)| matches!(object.form, TypeForm::I64).then_some(*digest))
        .unwrap();
    let relay = declaration(&source, "relay");
    for child_depth in [
        contract::MAXIMUM_TYPE_DEPTH - 1,
        contract::MAXIMUM_TYPE_DEPTH,
    ] {
        let mut candidate = source.clone();
        let mut child = scalar;
        for _ in 0..child_depth {
            let object = TypeObject::new(TypeForm::OwnedProduct {
                fields: vec![StructuralTypeField {
                    name: Name::new("owner").unwrap(),
                    ty: child,
                }],
            })
            .unwrap();
            child = encode_type_object(&object).unwrap().0;
            candidate.types.insert(child, object);
        }
        for owner in candidate.owners.values_mut() {
            match owner {
                OwnerRecord::Parameter(parameter) => parameter.ty = child,
                OwnerRecord::Declaration(d) if d.header.owner == OwnerKey::Declaration(relay) => {
                    let DeclarationPayload::Function(function) = &mut d.payload else {
                        unreachable!()
                    };
                    function.result = child;
                }
                _ => {}
            }
        }
        let pair = TypeObject::new(TypeForm::OwnedProduct {
            fields: vec![
                StructuralTypeField {
                    name: Name::new("left").unwrap(),
                    ty: child,
                },
                StructuralTypeField {
                    name: Name::new("right").unwrap(),
                    ty: ordinary,
                },
            ],
        })
        .unwrap();
        assert!(
            !candidate
                .types
                .contains_key(&encode_type_object(&pair).unwrap().0),
            "the pair must be inferred rather than an existing annotation"
        );
        let checked = validate_full(&candidate);
        if child_depth == contract::MAXIMUM_TYPE_DEPTH - 1 {
            checked.unwrap();
        } else {
            let errors = checked.unwrap_err();
            assert!(
                errors
                    .iter()
                    .any(|error| error.code == "kernel_owned_product"
                        && error.message.contains("structural type depth bound")),
                "{errors:?}"
            );
        }
    }
}
