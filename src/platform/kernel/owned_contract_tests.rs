//! Independent expected failures for owned contracts and symbolic-only admission.
use super::*;
const ORDINARY_CONTRACT: &str = r#"declarations.begin
(units (module create owned
  (owned-contract create Marker (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_20000000000000000000000000000001 inspect (parameters (I64 unrestricted)) (returns I64)))
  (function create identity (visibility public) (parameter create n (type I64))
    (returns I64) (effect pure) (body (local n)))
  (owned-implementation create MarkerCell (visibility public) (contract Marker) (self OwnedI64Cell)
    (method method_20000000000000000000000000000001 identity))))
declarations.end
"#;
fn author(source: &str) -> Result<KernelSnapshot, String> {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source)
}

#[test]
fn owned_contract_owner_domains_are_canonical_and_exact() {
    let source = author(ORDINARY_CONTRACT).unwrap();
    let mut seen = 0;
    for record in source.owners.values() {
        if !matches!(
            record.kind(),
            OwnerKind::OwnedContract | OwnerKind::OwnedImplementation
        ) {
            continue;
        }
        seen += 1;
        let (digest, bytes) = encode_owner(record).unwrap();
        assert_eq!(&bytes[..8], b"LKJOWN28");
        assert_eq!(
            decode_owner(&bytes, record.owner(), record.kind(), digest).unwrap(),
            *record
        );
        for generation in [18, 19, 20] {
            let mut historical = record.clone();
            historical.set_encoding_for_edit(generation);
            let (digest, bytes) = encode_owner(&historical).unwrap();
            assert_eq!(&bytes[..8], format!("LKJOWN{generation}").as_bytes());
            assert_eq!(
                decode_owner(&bytes, historical.owner(), historical.kind(), digest).unwrap(),
                historical
            );
        }
        let OwnerRecord::Declaration(mut forged) = record.clone() else {
            unreachable!()
        };
        forged.header.owner = OwnerKey::Module(forged.module);
        assert_eq!(
            encode_owner(&OwnerRecord::Declaration(forged))
                .unwrap_err()
                .code,
            "kernel_owner_identity_domain"
        );
    }
    assert_eq!(seen, 2);
}

#[test]
fn ordinary_only_methods_still_require_an_exact_owned_self_owner() {
    // No owned primitive or Self type object: contract semantics cannot depend on type inventory.
    let literal = r#"declarations.begin
(units (module create contract
  (owned-contract create Marker (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_20000000000000000000000000000001 inspect (parameters (I64 unrestricted)) (returns I64)))))
declarations.end
"#;
    let mut source = author(literal).unwrap();
    assert!(source.types.values().all(|t| !matches!(
        t.form,
        TypeForm::ByteBuffer | TypeForm::OwnedI64Cell | TypeForm::TypeParameter { .. }
    )));
    let mut root = None;
    for (key, owner) in &mut source.owners {
        match owner {
            OwnerRecord::TypeParameter(p) => p.constraints = TypeParameterConstraints::None,
            OwnerRecord::Declaration(d)
                if matches!(d.payload, DeclarationPayload::OwnedContract(_)) =>
            {
                root = Some(*key)
            }
            _ => {}
        }
    }
    let mut diagnostics = vec![];
    validate_affine_roots_with_limits(
        &source,
        [root.unwrap()],
        &mut diagnostics,
        &mut 0,
        ExpressionValidationLimits {
            maximum_steps: 100,
            maximum_diagnostics: 10,
        },
    )
    .unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == "kernel_owned_contract"),
        "{diagnostics:?}"
    );
}

#[test]
fn owned_nominal_parameters_and_hidden_callable_method_types_reject() {
    let failure = author(
        r#"declarations.begin
(units (module create nominal
  (record create Phantom (visibility public)
    (type-parameter create T (constraint owned)) (field create marker (type I64)))))
declarations.end
"#,
    )
    .unwrap_err();
    assert!(
        failure.contains("kernel_owned_parameter_owner"),
        "{failure}"
    );
    for parameter in [
        "(function (I64) I64)",
        "Holder",
        "(Phantom (function (I64) I64))",
    ] {
        let source = format!(
            r#"declarations.begin
(units (module create hidden
  (record create Holder (visibility private) (field create callback (type (function (I64) I64))))
  (record create Phantom (visibility private) (type-parameter create T) (field create marker (type I64)))
  (owned-contract create Bad (visibility private)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_20000000000000000000000000000001 inspect (parameters ({parameter} unrestricted)) (returns I64)))))
declarations.end
"#
        );
        let failure = author(&source).unwrap_err();
        assert!(
            failure.contains("kernel_owned_contract"),
            "{parameter}: {failure}"
        );
    }
}

#[test]
fn owned_witness_inference_and_affine_metadata_stop_before_unadmitted_reads() {
    use std::cell::Cell;
    struct Read<'a> {
        source: &'a KernelSnapshot,
        reads: Cell<usize>,
        implementation_reads: Cell<usize>,
    }
    impl ExpressionRead for Read<'_> {
        fn package_id(&self) -> PackageId {
            self.source.root.package_id
        }
        fn owner(
            &self,
            key: OwnerKey,
        ) -> Result<Option<OwnerRecord>, crate::platform::diagnostic::Diagnostic> {
            self.reads.set(self.reads.get() + 1);
            let value = self.source.owners.get(&key).cloned();
            if value
                .as_ref()
                .is_some_and(|v| v.kind() == OwnerKind::OwnedImplementation)
            {
                self.implementation_reads
                    .set(self.implementation_reads.get() + 1);
            }
            Ok(value)
        }
        fn type_object(
            &self,
            ty: TypeObjectDigest,
        ) -> Result<Option<TypeObject>, crate::platform::diagnostic::Diagnostic> {
            self.reads.set(self.reads.get() + 1);
            Ok(self.source.types.get(&ty).cloned())
        }
        fn package_interface_owner(
            &self,
            _: PackageId,
            _: OwnerKey,
        ) -> Result<Option<PackageInterfaceRecord>, crate::platform::diagnostic::Diagnostic>
        {
            unreachable!()
        }
        fn has_dependency(
            &self,
            _: PackageId,
        ) -> Result<bool, crate::platform::diagnostic::Diagnostic> {
            unreachable!()
        }
    }
    let literal = format!(
        "{ORDINARY_CONTRACT}\ndeclarations.begin\n(units (module create probe (function create invoke (visibility private) (returns I64) (effect pure) (body (method-call concrete@owned::MarkerCell owned::Marker method_20000000000000000000000000000001 (i64 7))))))\ndeclarations.end\n"
    );
    let mut source = author(&literal).unwrap();
    let root = *source
        .owners
        .iter()
        .find(|(_, v)| matches!(v, OwnerRecord::Declaration(d) if d.name.as_str() == "invoke"))
        .unwrap()
        .0;
    // An unrelated ordinary inventory cannot cause a scan before zero-budget admission.
    let mut item = source.types.keys().next().copied().unwrap();
    for _ in 0..1024 {
        let object = TypeObject::new(TypeForm::List { item }).unwrap();
        item = encode_type_object(&object).unwrap().0;
        source.types.insert(item, object);
    }
    // Sixteen valid mappings, fifteen unused by the only call. Enlarging a
    // canonical inventory must increase proof work even when dispatch selects
    // its first method; no host-side resolver serves as the expected result.
    let ids: Vec<_> = (1..16)
        .map(|i| crate::platform::semantic_id::MethodId::migrate(b"unused-owned-method", i))
        .collect();
    for owner in source.owners.values_mut() {
        let OwnerRecord::Declaration(d) = owner else {
            continue;
        };
        match &mut d.payload {
            DeclarationPayload::OwnedContract(c) => {
                let prototype = c.methods[0].clone();
                for (index, id) in ids.iter().enumerate() {
                    let mut method = prototype.clone();
                    method.id = *id;
                    method.name = Name::new(format!("unused_{index}")).unwrap();
                    c.methods.push(method);
                }
            }
            DeclarationPayload::OwnedImplementation(i) => {
                let function = i.methods[0].function;
                i.methods
                    .extend(ids.iter().map(|id| OwnedMethodImplementation {
                        method: *id,
                        function,
                        type_arguments: vec![],
                    }));
                i.methods.sort_by_key(|m| m.method);
            }
            _ => {}
        }
    }
    let read = Read {
        source: &source,
        reads: Cell::new(0),
        implementation_reads: Cell::new(0),
    };
    let mut diagnostics = vec![];
    assert_eq!(
        validate_affine_roots_with_limits(
            &read,
            [root],
            &mut diagnostics,
            &mut 0,
            ExpressionValidationLimits {
                maximum_steps: 0,
                maximum_diagnostics: 1
            }
        ),
        Err(ExpressionValidationExhaustion::Steps)
    );
    assert_eq!(read.reads.get(), 0);
    let mut work = 99;
    assert_eq!(
        validate_expression_roots_with_limits(
            &read,
            [root],
            &mut diagnostics,
            &mut work,
            ExpressionValidationLimits {
                maximum_steps: 100,
                maximum_diagnostics: 1
            }
        ),
        Err(ExpressionValidationExhaustion::Steps)
    );
    assert_eq!(work, 100);
    assert_eq!(read.implementation_reads.get(), 0);
    assert!(diagnostics.is_empty());
    let mut total = 0;
    validate_affine_roots_with_limits(
        &source,
        [root],
        &mut diagnostics,
        &mut total,
        ExpressionValidationLimits {
            maximum_steps: 10000,
            maximum_diagnostics: 1,
        },
    )
    .unwrap();
    assert!(diagnostics.is_empty());
    assert!(total > 16 * 16 / 2, "every unused map must be metered");
    for limit in 0..total {
        read.reads.set(0);
        let mut work = 0;
        assert_eq!(
            validate_affine_roots_with_limits(
                &read,
                [root],
                &mut diagnostics,
                &mut work,
                ExpressionValidationLimits {
                    maximum_steps: limit,
                    maximum_diagnostics: 1
                }
            ),
            Err(ExpressionValidationExhaustion::Steps),
            "{limit}"
        );
        assert_eq!(work, limit);
        assert!(
            read.reads.get() <= limit,
            "metadata reads must be admitted before delegation"
        );
        assert!(diagnostics.is_empty());
    }
    let checkpoints = Cell::new(0);
    let cancellation = || {
        checkpoints.set(checkpoints.get() + 1);
        if checkpoints.get() == 50 {
            Err(crate::platform::diagnostic::Diagnostic::new(
                crate::platform::diagnostic::DiagnosticClass::Resource,
                "owned_fixture_cancelled",
                "cancel unused method inventory validation",
            ))
        } else {
            Ok(())
        }
    };
    let checked = super::infer::CheckedExpressionRead {
        read: &read,
        checkpoint: &cancellation,
    };
    let mut work = 0;
    validate_expression_roots_with_limits(
        &checked,
        [root],
        &mut diagnostics,
        &mut work,
        ExpressionValidationLimits {
            maximum_steps: 10000,
            maximum_diagnostics: 1,
        },
    )
    .unwrap();
    assert_eq!(checkpoints.get(), 50);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "owned_fixture_cancelled");
}
