//! Genuine unselected library preparation, plus private contextual fault inputs.
//! Faulted maps below are never loaded artifacts or admission tokens.
use super::*;
use crate::platform::compiler::{
    LoadedArtifact, OptimizationPolicy, build_clean, link_artifact, load_artifact,
};
use crate::platform::execution::ExecutionControl;
use crate::platform::execution::normalized::prepare::{
    NormalizedCode, NormalizedFunctionBody, NormalizedInstruction, NormalizedPreparationWork,
    NormalizedProgram, RuntimeIndexes, load_runtime_owners, load_type_objects, load_units,
    normalized_parameters, validate_normalized_resource_signature,
};
use crate::platform::execution::normalized::prepared_types::Budget;
use crate::platform::execution::normalized::value::ValueOrigin;
use crate::platform::kernel::{ParameterUse, TypeObject};
use crate::platform::publication::GraphRepository;
use std::sync::Arc;

const SOURCE: &str = r#"declarations.begin
(units (module create unselected-associated-library
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (associated-type assoc_type_c8000000000000000000000000000001 Block (bound owned))
    (method method_c8000000000000000000000000000001 inspect
      (parameters ((associated-contract-member Reader assoc_type_c8000000000000000000000000000001) borrow))
      (returns Unit) (effect pure)))
  (function create borrow-block (visibility public) (effect pure)
    (type-parameter create W (constraint owned))
    (implementation-parameter implparam_c8000000000000000000000000000001 reader Reader W)
    (parameter create block
      (type (associated-witness-projection borrow-block implparam_c8000000000000000000000000000001
        Reader assoc_type_c8000000000000000000000000000001)) (use borrow))
    (returns Unit) (body (unit)))))
declarations.end
"#;

struct Fixture {
    loaded: LoadedArtifact,
    declaration: DeclarationReference,
    parameters: Vec<NormalizedParameter>,
    result: TypeObjectDigest,
    units: BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    owners: RuntimeOwnerMap,
    types: BTreeMap<TypeObjectDigest, TypeObject>,
}

impl Fixture {
    fn context(&self) -> Context<'_> {
        Context {
            declaration: self.declaration,
            parameters: &self.parameters,
            result: self.result,
            units: &self.units,
            owners: &self.owners,
            types: &self.types,
        }
    }
    fn owned(&self, work: &mut Budget<'_>) -> Result<bool, Diagnostic> {
        self.context().owned(
            self.parameters[0].ty,
            Some(self.parameters[0].parameter),
            work,
        )
    }
}

fn fixture() -> Fixture {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .expect("fresh public native prepare and publication");
    crate::platform::kernel::validate_full(&source).unwrap();
    assert!(!source.owners.values().any(|owner| matches!(owner,
        OwnerRecord::Declaration(record) if matches!(record.payload, DeclarationPayload::OwnedImplementation(_)))));
    let declaration = source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.name.as_str() == "borrow-block" =>
            {
                Some(DeclarationReference {
                    package: source.root.package_id,
                    declaration: *id,
                })
            }
            _ => None,
        })
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&directory.path().join("library"), &source, None)
        .unwrap()
        .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let linked = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    let loaded = load_artifact(&linked.artifact.bytes).unwrap();
    loaded.require_current_admission(|| Ok(())).unwrap();
    let mut work = NormalizedPreparationWork::default();
    let inputs = load_units(&loaded, &mut work).unwrap();
    let units = inputs.units;
    let owners = load_runtime_owners(&loaded, &mut work).unwrap();
    let types = load_type_objects(&loaded, &mut work).unwrap();
    let indexes = RuntimeIndexes::build(&units, ValueOrigin::fresh().unwrap()).unwrap();
    let unit = &units[&(
        declaration.package,
        OwnerKey::Declaration(declaration.declaration),
    )];
    let CompilationPayload::Function { signature, .. } = &unit.payload else {
        unreachable!()
    };
    let parameters = normalized_parameters(
        &owners,
        declaration.package,
        unit,
        &signature.parameters,
        &indexes,
    )
    .unwrap();
    let result = unit.tables.types[signature.result as usize];
    Fixture {
        loaded,
        declaration,
        parameters,
        result,
        units,
        owners,
        types,
    }
}

#[test]
fn associated_signature_unselected_library_strictly_loads_and_prepares_without_runtime_properties()
{
    let f = fixture();
    let projected = f.parameters[0].ty;
    let declaration = f.declaration;
    let program = NormalizedProgram::prepare(f.loaded).unwrap();
    let function = program
        .functions
        .iter()
        .find(|f| f.declaration == declaration)
        .unwrap();
    assert!(function.implementation_arguments.is_empty());
    assert_eq!(function.implementation_parameters.len(), 1);
    assert_eq!(function.parameters[0].use_mode, ParameterUse::Borrow);
    assert_eq!(function.parameters[0].ty, projected);
    assert!(!program.is_owned_type(projected));
    for property in [
        &program.capture_safe_types,
        &program.ordinary_types,
        &program.buffer_free_types,
        &program.comparable_types,
        &program.application_free_types,
    ] {
        assert!(!property.contains(&projected));
    }
}

#[test]
fn associated_signature_context_refuses_wrong_scope_witness_member_and_consuming_generations() {
    // Each mutation changes only private classifier inputs, not an admitted
    // artifact. This tests the contextual checks, not the complete source grammar.
    for case in 0..9 {
        let mut f = fixture();
        let ty = f.parameters[0].ty;
        let parameter = f.parameters[0].parameter;
        let TypeForm::AssociatedWitnessProjection { contract, .. } = f.types[&ty].form else {
            unreachable!()
        };
        match case {
            0 => {
                let TypeForm::AssociatedWitnessProjection { scope, .. } =
                    &mut f.types.get_mut(&ty).unwrap().form
                else {
                    unreachable!()
                };
                *scope = contract;
            }
            1 => {
                let TypeForm::AssociatedWitnessProjection { parameter, .. } =
                    &mut f.types.get_mut(&ty).unwrap().form
                else {
                    unreachable!()
                };
                *parameter = ImplementationParameterId::migrate(b"missing-signature-witness", 0);
            }
            2 => {
                let TypeForm::AssociatedWitnessProjection { member, .. } =
                    &mut f.types.get_mut(&ty).unwrap().form
                else {
                    unreachable!()
                };
                *member = AssociatedTypeMemberId::migrate(b"missing-signature-member", 0);
            }
            3 => {
                let Some(OwnerRecord::Declaration(record)) = f.owners.get_mut(&(
                    f.declaration.package,
                    OwnerKey::Declaration(f.declaration.declaration),
                )) else {
                    unreachable!()
                };
                record.header.contract_version -= 1;
            }
            4 => {
                let Some(OwnerRecord::Parameter(record)) = f
                    .owners
                    .get_mut(&(f.declaration.package, OwnerKey::Parameter(parameter)))
                else {
                    unreachable!()
                };
                record.header.contract_version -= 1;
            }
            5 => {
                let Some(OwnerRecord::Parameter(record)) = f
                    .owners
                    .get_mut(&(f.declaration.package, OwnerKey::Parameter(parameter)))
                else {
                    unreachable!()
                };
                record.parent = ParameterParent::Function(contract.declaration);
            }
            6 => {
                let Some(OwnerRecord::Declaration(record)) = f.owners.get_mut(&(
                    contract.package,
                    OwnerKey::Declaration(contract.declaration),
                )) else {
                    unreachable!()
                };
                record.header.contract_version -= 1;
            }
            7 => {
                let Some(OwnerRecord::Declaration(record)) = f.owners.get_mut(&(
                    f.declaration.package,
                    OwnerKey::Declaration(f.declaration.declaration),
                )) else {
                    unreachable!()
                };
                let DeclarationPayload::Function(function) = &mut record.payload else {
                    unreachable!()
                };
                function
                    .implementation_parameters
                    .push(function.implementation_parameters[0].clone());
            }
            8 => {
                let TypeForm::AssociatedWitnessProjection { member, .. } = f.types[&ty].form else {
                    unreachable!()
                };
                f.types.get_mut(&ty).unwrap().form =
                    TypeForm::AssociatedContractMember { contract, member };
            }
            _ => unreachable!(),
        }
        let control = ExecutionControl::uncancelled();
        let error = f.owned(&mut Budget::new(&control)).unwrap_err();
        assert_eq!(
            error.class,
            crate::platform::diagnostic::DiagnosticClass::Corrupt,
            "case{case}: {error:?}"
        );
        assert_eq!(error.code, "normalized_associated_signature");
    }
}

#[test]
fn associated_signature_context_shares_work_storage_cancellation_and_healthy_retry() {
    let f = fixture();
    let control = ExecutionControl::uncancelled();
    let mut work = Budget::new(&control);
    assert!(f.owned(&mut work).unwrap());
    let (steps, bytes) = work.work_for_test();
    assert!(steps > 1 && bytes > 0);
    assert!(f.owned(&mut work).unwrap());
    assert_eq!(work.work_for_test(), (steps * 2, bytes * 2));
    let maximum = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
    for (remaining, success) in [(steps * 2, true), (steps * 2 - 1, false)] {
        let mut work = Budget::with_work_for_test(&control, maximum - remaining, 0);
        assert!(f.owned(&mut work).unwrap());
        match f.owned(&mut work) {
            Ok(owned) => assert!(success && owned),
            Err(error) => {
                assert!(!success);
                assert_eq!(
                    error.class,
                    crate::platform::diagnostic::DiagnosticClass::Resource,
                    "{error:?}"
                );
            }
        }
    }
    let maximum_bytes = 256 * 1024 * 1024;
    for (remaining, success) in [(bytes, true), (bytes - 1, false)] {
        let mut work = Budget::with_work_for_test(&control, 0, maximum_bytes - remaining);
        match f.owned(&mut work) {
            Ok(owned) => assert!(success && owned),
            Err(error) => {
                assert!(!success);
                assert_eq!(
                    error.class,
                    crate::platform::diagnostic::DiagnosticClass::Resource,
                    "{error:?}"
                );
            }
        }
    }
    for cutoff in [0, 1, steps / 2, steps - 1] {
        let control = ExecutionControl::cancel_after_checks(cutoff as u64);
        let error = f.owned(&mut Budget::new(&control)).unwrap_err();
        assert_eq!(
            error.class,
            crate::platform::diagnostic::DiagnosticClass::Cancelled,
            "cutoff{cutoff}: {error:?}"
        );
    }
    assert!(f.owned(&mut Budget::new(&control)).unwrap());

    // Exercise the actual signature prefix and real closure API on the SAME
    // ledger. Re-preparing this genuine loaded program is private test setup;
    // no source map, bytecode, admission seal or function context is fabricated.
    let mut program = NormalizedProgram::prepare(f.loaded.clone()).unwrap();
    let signature = program
        .functions
        .iter()
        .find(|function| function.declaration == f.declaration)
        .unwrap()
        .clone();
    let validate = |work: &mut Budget<'_>| {
        validate_normalized_resource_signature(
            f.declaration,
            &signature.type_parameters,
            &f.parameters,
            f.result,
            &[],
            &signature.body,
            &f.units,
            &f.owners,
            &f.types,
            &[],
            work,
        )
    };
    let mut prefix = Budget::new(&control);
    validate(&mut prefix).unwrap();
    let (prefix_steps, _) = prefix.work_for_test();
    assert!(prefix_steps > steps);
    let mut exhausted = Budget::with_work_for_test(&control, maximum - prefix_steps, 0);
    validate(&mut exhausted).unwrap();
    assert_eq!(exhausted.work_for_test().0, maximum);
    let error = super::super::super::prepared_types::complete_with_implementations(
        &mut program,
        &f.units,
        &f.owners,
        &mut exhausted,
    )
    .unwrap_err();
    assert_eq!(
        error.class,
        crate::platform::diagnostic::DiagnosticClass::Resource,
        "{error:?}"
    );
    assert_eq!(error.code, "normalized_instantiation_work");
    let mut one_short = Budget::with_work_for_test(&control, maximum - prefix_steps + 1, 0);
    let error = validate(&mut one_short).unwrap_err();
    assert_eq!(
        error.class,
        crate::platform::diagnostic::DiagnosticClass::Resource,
        "{error:?}"
    );
    for cutoff in [0, 1, prefix_steps - 1] {
        let cancellation = ExecutionControl::cancel_after_checks(cutoff as u64);
        let error = validate(&mut Budget::new(&cancellation)).unwrap_err();
        assert_eq!(
            error.class,
            crate::platform::diagnostic::DiagnosticClass::Cancelled,
            "signature cutoff{cutoff}: {error:?}"
        );
    }
    let mut retry = Budget::new(&control);
    validate(&mut retry).unwrap();
    super::super::super::prepared_types::complete_with_implementations(
        &mut program,
        &f.units,
        &f.owners,
        &mut retry,
    )
    .unwrap();
    assert!(retry.work_for_test().0 > prefix_steps);
}

#[test]
fn associated_signature_category_keeps_affine_mode_and_ordinary_use_guards() {
    let mut f = fixture();
    let control = ExecutionControl::uncancelled();
    let body = NormalizedFunctionBody::Code(NormalizedCode {
        parameter_count: 1,
        local_count: 1,
        instructions: Arc::from([NormalizedInstruction::Unit, NormalizedInstruction::Return]),
    });
    f.parameters[0].use_mode = ParameterUse::Unrestricted;
    let error = validate_normalized_resource_signature(
        f.declaration,
        &[],
        &f.parameters,
        f.result,
        &[],
        &body,
        &f.units,
        &f.owners,
        &f.types,
        &[],
        &mut Budget::new(&control),
    )
    .unwrap_err();
    assert_eq!(error.code, "normalized_buffer_signature");
    f.parameters[0].use_mode = ParameterUse::Borrow;
    f.parameters[0].ty = f.result; // Unit is ordinary; a Borrow mode still refuses.
    let error = validate_normalized_resource_signature(
        f.declaration,
        &[],
        &f.parameters,
        f.result,
        &[],
        &body,
        &f.units,
        &f.owners,
        &f.types,
        &[],
        &mut Budget::new(&control),
    )
    .unwrap_err();
    assert_eq!(error.code, "normalized_function_parameter_use");
}
