//! Canonical prerequisite admission has no dependency on producer applications.
use super::*;
use crate::platform::execution::normalized::tests::{
    byte_buffer_tests, owned_implementation_scheme_tests,
};
use crate::platform::kernel::ImplementationOperand;
use std::sync::OnceLock;

const UNUSED: &str = r#"
  (owned-implementation create UnusedWrapper (visibility public)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_c3000000000000000000000000000001 ops Keep T)
    (contract Keep) (self T)
    (method method_c1000000000000000000000000000001 keep-through (types T)
      (implementations parameter@UnusedWrapper@implparam_c3000000000000000000000000000001)))
  (function create unused-application (visibility private) (effect pure)
    (type-parameter create T (constraint owned transferable))
    (implementation-parameter implparam_c3000000000000000000000000000002 ops Keep T)
    (parameter create value (type T) (use consume)) (returns T)
    (body (implementation-call outer (types T)
      (implementations (implementation Wrapper (types T)
        (implementations parameter@unused-application@implparam_c3000000000000000000000000000002)))
      (local value))))
"#;

fn fixture() -> KernelSnapshot {
    static SOURCE: OnceLock<KernelSnapshot> = OnceLock::new();
    SOURCE
        .get_or_init(|| {
            let input = owned_implementation_scheme_tests::prerequisite_source().replace(
                "  (function create prerequisite-main",
                &format!("{UNUSED}  (function create prerequisite-main"),
            );
            byte_buffer_tests::author_only(&input).unwrap()
        })
        .clone()
}

fn named(source: &KernelSnapshot, name: &str) -> DeclarationReference {
    source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(owner))
                if owner.name.as_str() == name =>
            {
                Some(DeclarationReference {
                    package: source.root.package_id,
                    declaration: *declaration,
                })
            }
            _ => None,
        })
        .unwrap()
}

fn payload(
    source: &mut KernelSnapshot,
    reference: DeclarationReference,
) -> &mut DeclarationPayload {
    let OwnerRecord::Declaration(owner) = source
        .owners
        .get_mut(&OwnerKey::Declaration(reference.declaration))
        .unwrap()
    else {
        panic!("declaration owner");
    };
    &mut owner.payload
}

fn reject(source: &KernelSnapshot) {
    assert!(NormalizedReferenceSchema::reconstruct([source]).is_err());
}

#[test]
fn canonical_prerequisite_closure_derives_nested_and_symbolic_applications_without_preparation() {
    let source = fixture();
    NormalizedReferenceSchema::reconstruct([&source]).unwrap();
}

#[test]
fn canonical_prerequisite_duplicate_dag_admission_scales_with_unique_nodes() {
    let source =
        byte_buffer_tests::author_only(&owned_implementation_scheme_tests::duplicate_dag_source(3))
            .unwrap();
    let (shallow_nodes, shallow_work) =
        super::super::reference_types::check_duplicate_witness_dag(&source, 12).unwrap();
    let (deep_nodes, deep_work) =
        super::super::reference_types::check_duplicate_witness_dag(&source, 24).unwrap();
    assert_eq!(shallow_nodes, 13);
    assert_eq!(deep_nodes, 25);
    // Doubling source depth permits polynomial admission overhead. Expanding
    // the duplicate-child tree would multiply work by thousands instead.
    assert!(deep_work < shallow_work.saturating_mul(6));
}

#[test]
fn canonical_prerequisite_depth_checks_longer_paths_before_memo_reuse() {
    let source =
        byte_buffer_tests::author_only(&owned_implementation_scheme_tests::duplicate_dag_source(3))
            .unwrap();
    let maximum = crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH;
    assert!(
        super::super::reference_types::check_duplicate_witness_reuse_depth(&source, maximum, 0,)
            .is_ok()
    );
    assert!(
        super::super::reference_types::check_duplicate_witness_reuse_depth(&source, maximum, 1,)
            .is_err()
    );
    assert!(super::super::reference_types::check_duplicate_witness_reuse_depth(
        &source, maximum + 1, 0,
    ).is_err());
}

#[test]
fn canonical_prerequisite_inventory_rejects_unused_missing_scoped_or_nominal_obligations() {
    let original = fixture();
    let unused = named(&original, "UnusedWrapper");
    let foreign = named(&original, "Wrapper");
    let read = named(&original, "Read");

    let mut source = original.clone();
    let DeclarationPayload::OwnedImplementation(scheme) = payload(&mut source, unused) else {
        panic!("scheme");
    };
    scheme.methods[0].implementations.clear();
    reject(&source);

    let mut source = original.clone();
    let DeclarationPayload::OwnedImplementation(scheme) = payload(&mut source, unused) else {
        panic!("scheme");
    };
    let ImplementationOperand::Parameter { scope, .. } = &mut scheme.methods[0].implementations[0]
    else {
        panic!("scoped prerequisite");
    };
    *scope = foreign;
    reject(&source);

    let mut source = original.clone();
    let DeclarationPayload::OwnedImplementation(scheme) = payload(&mut source, unused) else {
        panic!("scheme");
    };
    scheme.implementation_parameters[0].contract = read;
    reject(&source);

    let mut source = original.clone();
    let DeclarationPayload::OwnedImplementation(foreign_scheme) = payload(&mut source, foreign)
    else {
        panic!("scheme");
    };
    let foreign_self = foreign_scheme.self_type;
    let DeclarationPayload::OwnedImplementation(scheme) = payload(&mut source, unused) else {
        panic!("scheme");
    };
    scheme.implementation_parameters[0].self_type = foreign_self;
    reject(&source);
}

#[test]
fn canonical_prerequisite_closure_rejects_wrong_nested_leaf_and_lexical_scope() {
    let original = fixture();
    let plain = named(&original, "Plain");
    let read = named(&original, "PlainRead");
    let unused = named(&original, "unused-application");
    let outer = named(&original, "outer");
    let mut source = original.clone();
    let mut replaced = false;
    for owner in source.owners.values_mut() {
        let OwnerRecord::Expression(expression) = owner else {
            continue;
        };
        let ExpressionOperation::ImplementationCall {
            implementations, ..
        } = &mut expression.operation
        else {
            continue;
        };
        let Some(ImplementationOperand::Concrete {
            implementations: children,
            ..
        }) = implementations.first_mut()
        else {
            continue;
        };
        let Some(ImplementationOperand::Concrete {
            implementations: grandchildren,
            ..
        }) = children.first_mut()
        else {
            continue;
        };
        let Some(ImplementationOperand::Concrete { implementation, .. }) =
            grandchildren.first_mut()
        else {
            continue;
        };
        if *implementation == plain {
            *implementation = read;
            replaced = true;
            break;
        }
    }
    assert!(replaced);
    reject(&source);

    let mut source = original;
    let mut replaced = false;
    for owner in source.owners.values_mut() {
        let OwnerRecord::Expression(expression) = owner else {
            continue;
        };
        let ExpressionOperation::ImplementationCall {
            implementations, ..
        } = &mut expression.operation
        else {
            continue;
        };
        let Some(ImplementationOperand::Concrete {
            implementations: children,
            ..
        }) = implementations.first_mut()
        else {
            continue;
        };
        let Some(ImplementationOperand::Parameter { scope, .. }) = children.first_mut() else {
            continue;
        };
        if *scope == unused {
            *scope = outer;
            replaced = true;
            break;
        }
    }
    assert!(replaced);
    reject(&source);
}
