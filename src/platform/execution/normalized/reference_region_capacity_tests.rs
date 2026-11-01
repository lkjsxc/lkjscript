//! Private region walkers consume hashed canonical types after enclosing admission.
//! These fixtures isolate that traversal, not complete enclosing-source admission.
use super::*;

type TestResult<T = ()> = std::result::Result<T, String>;

fn checked<T, E: std::fmt::Debug>(result: std::result::Result<T, E>) -> TestResult<T> {
    result.map_err(|error| format!("{error:?}"))
}

fn failed<T>(result: Result<T>) -> TestResult<ExecutionError> {
    match result {
        Err(error) => Ok(error),
        Ok(_) => Err("region operation unexpectedly succeeded".to_owned()),
    }
}

struct Fixture {
    types: BTreeMap<TypeObjectDigest, TypeObject>,
    region: TypeObjectDigest,
    scalar: TypeObjectDigest,
    flat: TypeObjectDigest,
    foreign: TypeObjectDigest,
}

fn insert(
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    form: TypeForm,
) -> TestResult<TypeObjectDigest> {
    let object = checked(TypeObject::new(form))?;
    let (digest, _) = checked(encode_type_object(&object))?;
    types.insert(digest, object);
    Ok(digest)
}

fn fixture() -> TestResult<Fixture> {
    let mut types = BTreeMap::new();
    let scalar = insert(&mut types, TypeForm::I64)?;
    let callable = DeclarationReference {
        package: PackageId::migrate(b"region admission capacity", 0),
        declaration: crate::platform::semantic_id::DeclarationId::migrate(
            b"region admission capacity",
            0,
        ),
    };
    let region = insert(
        &mut types,
        TypeForm::RegionBrand {
            callable,
            binding: BindingId::migrate(b"region admission capacity", 0),
        },
    )?;
    let other_region = insert(
        &mut types,
        TypeForm::RegionBrand {
            callable,
            binding: BindingId::migrate(b"region admission capacity", 1),
        },
    )?;
    let pointer = insert(
        &mut types,
        TypeForm::RelPtr {
            region,
            item: scalar,
        },
    )?;
    let foreign = insert(
        &mut types,
        TypeForm::RelPtr {
            region: other_region,
            item: scalar,
        },
    )?;
    let flat = insert(
        &mut types,
        TypeForm::StructuralRecord {
            fields: vec![
                StructuralTypeField {
                    name: checked(Name::new("count"))?,
                    ty: scalar,
                },
                StructuralTypeField {
                    name: checked(Name::new("link"))?,
                    ty: pointer,
                },
            ],
        },
    )?;
    Ok(Fixture {
        types,
        region,
        scalar,
        flat,
        foreign,
    })
}

#[test]
fn reference_region_flat_cell_proof_keeps_exact_optional_work_and_storage() -> TestResult {
    let fixture = fixture()?;
    let control = ExecutionControl::uncancelled();
    let proof = checked(Proof::reconstruct(&[], &control))?;
    let mut resolver = proof.resolver_with_budget(&fixture.types, Budget::new(&control));
    checked(resolver.region_cell(fixture.flat, fixture.region))?;
    let work = resolver.budget.work.observation();
    let entries = resolver.budget.entries.observation();
    assert!(!work.saturated && !entries.saturated);
    assert!(work.consumed > 0 && entries.consumed > 0);
    for (work_limit, entry_limit, expected) in [
        (None, None, true),
        (Some(work.consumed), Some(entries.consumed), true),
        (Some(u64::MAX), Some(u64::MAX), true),
        (Some(0), None, false),
        (None, Some(0), false),
        (Some(work.consumed - 1), None, false),
        (None, Some(entries.consumed - 1), false),
    ] {
        let mut resolver = proof.resolver_with_budget(
            &fixture.types,
            Budget::with_limits(&control, work_limit, entry_limit),
        );
        let result = resolver.region_cell(fixture.flat, fixture.region);
        if expected {
            checked(result)?;
            assert_eq!(
                resolver.budget.work.remaining(),
                work_limit.map(|n| n - work.consumed)
            );
            assert_eq!(
                resolver.budget.entries.remaining(),
                entry_limit.map(|n| n - entries.consumed)
            );
        } else {
            assert_eq!(failed(result)?.class, ExecutionFailureClass::Resource);
        }
    }
    Ok(())
}

#[test]
fn reference_region_unbounded_walk_still_rejects_escape_and_foreign_brands() -> TestResult {
    let fixture = fixture()?;
    let control = ExecutionControl::uncancelled();
    let proof = checked(Proof::reconstruct(&[], &control))?;
    let mut resolver = proof.resolver_with_budget(&fixture.types, Budget::new(&control));
    checked(resolver.region_absent(fixture.scalar, fixture.region))?;
    assert_eq!(
        failed(resolver.region_absent(fixture.flat, fixture.region))?.class,
        ExecutionFailureClass::Infrastructure
    );
    assert_eq!(
        failed(resolver.region_cell(fixture.foreign, fixture.region))?.class,
        ExecutionFailureClass::Infrastructure
    );
    checked(resolver.region_cell(fixture.flat, fixture.region))?;
    let cancelled = ExecutionControl::uncancelled();
    cancelled.cancel();
    let mut resolver = proof.resolver_with_budget(&fixture.types, Budget::new(&cancelled));
    assert_eq!(
        failed(resolver.region_cell(fixture.flat, fixture.region))?.class,
        ExecutionFailureClass::Cancelled
    );
    assert_eq!(resolver.budget.work.observation(), Observation::exact(0));
    assert_eq!(resolver.budget.entries.observation(), Observation::exact(0));
    Ok(())
}

#[test]
fn reference_region_real_worklist_capacity_failure_preserves_existing_storage() -> TestResult {
    let mut pending = vec![7_u8];
    let pointer = pending.as_ptr();
    let error = failed(region_pending(&mut pending, usize::MAX))?;
    assert_eq!(error.class, ExecutionFailureClass::Resource);
    assert_eq!(error.code, "reference_region_storage");
    assert_eq!(pending, vec![7_u8]);
    assert_eq!(pending.as_ptr(), pointer);
    Ok(())
}
