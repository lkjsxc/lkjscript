//! Substitution contexts are disposable and must never capture another call.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::TypeParameterId;
use std::cell::Cell;
use std::collections::BTreeMap;

const SEED: &[u8] = b"applied-type-environments";
static EMPTY_READ_SCOPES: BTreeMap<ReadScopeReference, ReadScopeArgument> = BTreeMap::new();

#[derive(Default)]
struct Read {
    types: BTreeMap<TypeObjectDigest, TypeObject>,
    cancelled: Cell<bool>,
}

impl Read {
    fn ty(&mut self, form: TypeForm) -> TypeObjectDigest {
        let object = TypeObject::new(form).unwrap();
        let digest = encode_type_object(&object).unwrap().0;
        self.types.insert(digest, object);
        digest
    }
}

impl ExpressionRead for Read {
    fn package_id(&self) -> PackageId {
        PackageId::migrate(SEED, 0)
    }
    fn owner(&self, _owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        Ok(None)
    }
    fn type_object(&self, ty: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        Ok(self.types.get(&ty).cloned())
    }
    fn package_interface_owner(
        &self,
        _package: PackageId,
        _owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        Ok(None)
    }
    fn has_dependency(&self, _package: PackageId) -> Result<bool, Diagnostic> {
        Ok(false)
    }
    fn validation_work(&self) -> Result<(), Diagnostic> {
        if self.cancelled.get() {
            Err(Diagnostic::new(
                DiagnosticClass::Cancelled,
                "test_cancelled",
                "substitution was cancelled",
            ))
        } else {
            Ok(())
        }
    }
}

#[test]
fn repeated_substitution_keeps_exact_bindings_and_retains_previous_results() {
    let mut read = Read::default();
    let parameter = TypeParameterId::migrate(SEED, 0);
    let generic = read.ty(TypeForm::TypeParameter { parameter });
    let template = read.ty(TypeForm::Option { item: generic });
    let first = read.ty(TypeForm::I64);
    let second = read.ty(TypeForm::Bool);
    let expected_first = read.ty(TypeForm::Option { item: first });
    let expected_second = read.ty(TypeForm::Option { item: second });
    // Results must be available from the overlay, not just from fixture input.
    read.types.remove(&expected_first);
    read.types.remove(&expected_second);
    let first_bindings = BTreeMap::from([(parameter, first)]);
    let second_bindings = BTreeMap::from([(parameter, second)]);
    let mut applied = parallel_types::AppliedTypes::new(&read);
    assert_eq!(
        applied.substitute(template, &first_bindings, 0).unwrap(),
        expected_first
    );
    let used = applied.metadata_bytes_for_test();
    assert_eq!(
        applied.substitute(template, &first_bindings, 0).unwrap(),
        expected_first
    );
    assert_eq!(applied.metadata_bytes_for_test(), used);
    assert_eq!(
        applied.substitute(template, &second_bindings, 0).unwrap(),
        expected_second
    );
    assert_eq!(
        applied.type_object(expected_first).unwrap().unwrap().form,
        TypeForm::Option { item: first }
    );
    assert_eq!(
        applied.type_object(expected_second).unwrap().unwrap().form,
        TypeForm::Option { item: second }
    );
    assert_eq!(
        applied.substitute(template, &BTreeMap::new(), 0).unwrap(),
        template
    );
    assert_eq!(
        applied.substitute(template, &first_bindings, 0).unwrap(),
        expected_first
    );
}

#[test]
fn simultaneous_actuals_are_not_captured_by_another_parameter_binding() {
    let mut read = Read::default();
    let parameter = TypeParameterId::migrate(SEED, 0);
    let caller_parameter = TypeParameterId::migrate(SEED, 1);
    let generic = read.ty(TypeForm::TypeParameter { parameter });
    let caller_actual = read.ty(TypeForm::TypeParameter {
        parameter: caller_parameter,
    });
    let concrete = read.ty(TypeForm::I64);
    let template = read.ty(TypeForm::Option { item: generic });
    let expected = read.ty(TypeForm::Option {
        item: caller_actual,
    });
    let bindings = BTreeMap::from([(parameter, caller_actual), (caller_parameter, concrete)]);
    let mut applied = parallel_types::AppliedTypes::new(&read);
    assert_eq!(
        applied.substitute(template, &bindings, 0).unwrap(),
        expected
    );
}

#[test]
fn binding_snapshot_is_admitted_before_copy_and_refusal_preserves_the_previous_context() {
    let mut read = Read::default();
    let parameter = TypeParameterId::migrate(SEED, 0);
    let generic = read.ty(TypeForm::TypeParameter { parameter });
    let first = read.ty(TypeForm::I64);
    let second = read.ty(TypeForm::Bool);
    let bindings = BTreeMap::from([(parameter, first)]);
    let other_bindings = BTreeMap::from([(parameter, second)]);
    let mut observed = parallel_types::AppliedTypes::new(&read);
    assert_eq!(observed.substitute(generic, &bindings, 0).unwrap(), first);
    let required = observed.metadata_bytes_for_test();
    assert!(required > 0);
    let mut refused = parallel_types::AppliedTypes::with_metadata_limit(&read, required - 1);
    assert_eq!(
        refused.substitute(generic, &bindings, 0).unwrap_err().code,
        "kernel_parallel_type_storage"
    );
    assert_eq!(refused.metadata_bytes_for_test(), 0);
    let mut exact = parallel_types::AppliedTypes::with_metadata_limit(&read, required);
    assert_eq!(exact.substitute(generic, &bindings, 0).unwrap(), first);
    assert_eq!(
        exact
            .substitute(generic, &other_bindings, 0)
            .unwrap_err()
            .code,
        "kernel_parallel_type_storage"
    );
    assert_eq!(exact.substitute(generic, &bindings, 0).unwrap(), first);
}

#[test]
fn cached_substitution_remains_cancellable_and_recovers_without_changing_context() {
    let mut read = Read::default();
    let parameter = TypeParameterId::migrate(SEED, 0);
    let generic = read.ty(TypeForm::TypeParameter { parameter });
    let template = read.ty(TypeForm::Option { item: generic });
    let actual = read.ty(TypeForm::I64);
    let bindings = BTreeMap::from([(parameter, actual)]);
    let mut applied = parallel_types::AppliedTypes::new(&read);
    let expected = applied.substitute(template, &bindings, 0).unwrap();
    let used = applied.metadata_bytes_for_test();
    read.cancelled.set(true);
    assert_eq!(
        applied
            .substitute(template, &bindings, 0)
            .unwrap_err()
            .class,
        DiagnosticClass::Cancelled
    );
    assert_eq!(applied.metadata_bytes_for_test(), used);
    read.cancelled.set(false);
    assert_eq!(
        applied.substitute(template, &bindings, 0).unwrap(),
        expected
    );
    assert_eq!(applied.metadata_bytes_for_test(), used);
}

#[test]
fn source_effects_are_rewritten_before_opaque_caller_type_actuals() {
    use crate::platform::semantic_id::{EffectParameterId, RequirementId};
    use owned_associated::AssociatedTypeContext;
    let mut read = Read::default();
    let parameter = TypeParameterId::migrate(SEED, 10);
    let effect_parameter = EffectParameterReference {
        package: read.package_id(),
        parameter: EffectParameterId::migrate(SEED, 10),
    };
    let requirement: RequirementOperand = RequirementReference {
        package: read.package_id(),
        requirement: RequirementId::migrate(SEED, 10),
    }
    .into();
    let unit = read.ty(TypeForm::Unit);
    let generic = read.ty(TypeForm::TypeParameter { parameter });
    let symbolic = EffectRow {
        requirements: vec![],
        parameters: vec![effect_parameter],
    };
    let concrete = EffectRow {
        requirements: vec![requirement],
        parameters: vec![],
    };
    let caller_actual = read.ty(TypeForm::TaskFunction {
        parameters: vec![],
        result: unit,
        effect: symbolic.clone(),
    });
    let template = read.ty(TypeForm::TaskFunction {
        parameters: vec![generic],
        result: unit,
        effect: symbolic,
    });
    let expected = read.ty(TypeForm::TaskFunction {
        parameters: vec![caller_actual],
        result: unit,
        effect: concrete.clone(),
    });
    read.types.remove(&expected);
    let effects = BTreeMap::from([(effect_parameter, concrete)]);
    let requirements = RequirementSubstitution::new();
    let ordinary = BTreeMap::from([(parameter, caller_actual)]);
    let implementations = BTreeMap::new();
    let mut effect_types = parallel_types::EffectAppliedTypes::new(&read, &effects, &requirements);
    let prepared = effect_types.substitute(template, 0).unwrap();
    let mut applied = parallel_types::WitnessAppliedTypes::new(
        &effect_types,
        parallel_types::AssociatedTypeEnvironment {
            ordinary: &ordinary,
            source: AssociatedTypeContext::Forbidden,
            caller: AssociatedTypeContext::Forbidden,
            consumer_generation: contract::GRAPH_CONTRACT_VERSION,
            contract_selection: None,
            implementations: &implementations,

            scopes: &EMPTY_READ_SCOPES,
        },
    );
    assert_eq!(applied.substitute(prepared, 0).unwrap(), expected);
    let derived = applied.into_types();
    assert!(
        derived.contains_key(&expected),
        "derived signature must remain available"
    );
    let overlay = owned_contract::AppliedTypeRead {
        read: &read,
        types: &derived,
    };
    let mut wrong_order =
        parallel_types::EffectAppliedTypes::new(&overlay, &effects, &requirements);
    assert_ne!(
        wrong_order.substitute(expected, 0).unwrap(),
        expected,
        "rewriting after type insertion would capture the caller's effect parameter"
    );
}

#[test]
fn effect_substitution_cache_retains_exact_environment_and_cancellation() {
    use crate::platform::semantic_id::{EffectParameterId, RequirementId};
    let mut read = Read::default();
    let parameter = EffectParameterReference {
        package: read.package_id(),
        parameter: EffectParameterId::migrate(SEED, 11),
    };
    let unit = read.ty(TypeForm::Unit);
    let template = read.ty(TypeForm::TaskFunction {
        parameters: vec![],
        result: unit,
        effect: EffectRow {
            requirements: vec![],
            parameters: vec![parameter],
        },
    });
    let requirements = RequirementSubstitution::new();
    let empty = BTreeMap::from([(
        parameter,
        EffectRow {
            requirements: vec![],
            parameters: vec![],
        },
    )]);
    let nonempty = BTreeMap::from([(
        parameter,
        EffectRow {
            requirements: vec![
                RequirementReference {
                    package: read.package_id(),
                    requirement: RequirementId::migrate(SEED, 11),
                }
                .into(),
            ],
            parameters: vec![],
        },
    )]);
    let mut first = parallel_types::EffectAppliedTypes::new(&read, &empty, &requirements);
    let expected = first.substitute(template, 0).unwrap();
    let mut second = parallel_types::EffectAppliedTypes::new(&read, &nonempty, &requirements);
    assert_ne!(second.substitute(template, 0).unwrap(), expected);
    read.cancelled.set(true);
    assert_eq!(
        first.substitute(template, 0).unwrap_err().class,
        DiagnosticClass::Cancelled
    );
    read.cancelled.set(false);
    assert_eq!(first.substitute(template, 0).unwrap(), expected);
    assert!(first.into_types().contains_key(&expected));
}
