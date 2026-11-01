//! Capacity controls exercise the independent canonical type readers directly.
//! Seeded observations test accounting, not physical allocation or workload size.

use super::*;
use crate::platform::execution::{ExecutionControl, ExecutionFailureClass};

fn scalar_schema() -> Result<(NormalizedReferenceSchema, TypeObjectDigest), ExecutionError> {
    let mut schema = NormalizedReferenceSchema::default();
    let ty = insert(&mut schema, TypeForm::I64)?;
    Ok((schema, ty))
}

fn insert(
    schema: &mut NormalizedReferenceSchema,
    form: TypeForm,
) -> Result<TypeObjectDigest, ExecutionError> {
    let object = TypeObject::new(form).map_err(|_| failure())?;
    let (ty, _) = encode_type_object(&object).map_err(|_| failure())?;
    schema.types.insert(ty, object);
    Ok(ty)
}

#[test]
fn reference_capacity_type_policy_none_zero_one_short_exact_and_max() -> Result<(), ExecutionError>
{
    let (schema, ty) = scalar_schema()?;
    let control = ExecutionControl::uncancelled();
    let mut work = Meter::new(None);
    let mut metadata = Meter::new(None);
    parallel_transfer_type(&schema, &[], ty, None, &mut work, &mut metadata, &control)?;
    let exact_work = work.observation().consumed;
    let exact_bytes = metadata.observation().consumed;
    assert!(exact_work > 0 && exact_bytes > 0);
    assert!(!work.observation().saturated && !metadata.observation().saturated);

    for (steps, bytes, accepts) in [
        (None, None, true),
        (Some(0), None, false),
        (None, Some(0), false),
        (Some(exact_work - 1), Some(exact_bytes), false),
        (Some(exact_work), Some(exact_bytes - 1), false),
        (Some(exact_work), Some(exact_bytes), true),
        (Some(u64::MAX), Some(u64::MAX), true),
    ] {
        let mut work = Meter::new(steps);
        let mut metadata = Meter::new(bytes);
        let result =
            parallel_transfer_type(&schema, &[], ty, None, &mut work, &mut metadata, &control);
        if accepts {
            result?;
            assert_eq!(work.observation(), Observation::exact(exact_work));
            assert_eq!(metadata.observation(), Observation::exact(exact_bytes));
            assert_eq!(work.remaining(), steps.map(|limit| limit - exact_work));
            assert_eq!(metadata.remaining(), bytes.map(|limit| limit - exact_bytes));
        } else {
            assert!(
                result
                    .as_ref()
                    .is_err_and(|error| error.class == ExecutionFailureClass::Resource),
                "{result:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn reference_capacity_type_saturation_preserves_complete_child_rejection()
-> Result<(), ExecutionError> {
    let (mut schema, ty) = scalar_schema()?;
    let buffer = insert(&mut schema, TypeForm::ByteBuffer)?;
    let bad_list = insert(&mut schema, TypeForm::List { item: buffer })?;
    let control = ExecutionControl::uncancelled();
    let mut work = Meter::new(None);
    let mut metadata = Meter::new(None);
    assert!(work.charge(Observation::exact(u64::MAX)).is_ok());
    assert!(metadata.charge(Observation::exact(u64::MAX)).is_ok());
    parallel_transfer_type(&schema, &[], ty, None, &mut work, &mut metadata, &control)?;
    assert!(work.observation().saturated && metadata.observation().saturated);
    let rejection = parallel_transfer_type(
        &schema,
        &[],
        bad_list,
        None,
        &mut work,
        &mut metadata,
        &control,
    );
    assert!(
        rejection
            .as_ref()
            .is_err_and(|error| error.class == ExecutionFailureClass::Infrastructure
                && error.code == "reference_instantiation_scope"),
        "a list must still reject an affine element: {rejection:?}"
    );

    let safe = property_types(
        &schema,
        &mut work,
        &mut metadata,
        &control,
        Retention::Transfer,
    )?;
    assert!(safe.contains(&ty));
    assert!(!safe.contains(&bad_list));
    assert!(work.observation().saturated && metadata.observation().saturated);
    Ok(())
}

#[test]
fn reference_capacity_modeled_metadata_and_actual_layout_are_separate() -> Result<(), ExecutionError>
{
    const FORMER_METADATA_BYTES: usize = 256 * 1024 * 1024;
    let mut metadata = Meter::new(None);
    allocate::<u8>(&mut metadata, FORMER_METADATA_BYTES + 1)?;
    assert_eq!(
        metadata.observation(),
        Observation::from_usize(FORMER_METADATA_BYTES + 1)
    );
    let mut finite = Meter::new(Some(FORMER_METADATA_BYTES as u64));
    let before = finite;
    assert!(allocate::<u8>(&mut finite, FORMER_METADATA_BYTES + 1).is_err());
    assert_eq!(finite, before);

    let mut maximum = Meter::new(Some(u64::MAX));
    assert!(maximum.charge(Observation::exact(u64::MAX)).is_ok());
    assert_eq!(maximum.remaining(), Some(0));
    assert!(allocate::<u8>(&mut maximum, 1).is_err());
    assert!(!maximum.observation().saturated);
    assert!(metadata.charge(Observation::exact(u64::MAX)).is_ok());
    allocate::<u8>(&mut metadata, 1)?;
    assert!(metadata.observation().saturated);

    // This is a deterministic representational refusal, not an attempted huge
    // allocation. Vec cannot represent a contiguous u8 allocation of usize::MAX.
    let refusal = reserved_vec::<u8>(usize::MAX);
    assert!(
        refusal
            .as_ref()
            .is_err_and(|error| error.class == ExecutionFailureClass::Resource),
        "{refusal:?}"
    );
    let small = reserved_vec::<u8>(7)?;
    assert!(small.is_empty() && small.capacity() >= 7);
    Ok(())
}

#[test]
fn reference_capacity_type_cancellation_stays_distinct_from_policy_refusal()
-> Result<(), ExecutionError> {
    let (schema, ty) = scalar_schema()?;
    let mut completed = false;
    for cutoff in 0..32 {
        let result = parallel_transfer_type(
            &schema,
            &[],
            ty,
            None,
            &mut Meter::new(None),
            &mut Meter::new(None),
            &ExecutionControl::cancel_after_checks(cutoff),
        );
        match result {
            Ok(()) => {
                completed = true;
                break;
            }
            Err(error) => assert_eq!(error.class, ExecutionFailureClass::Cancelled, "{error:?}"),
        }
    }
    assert!(completed, "the finite scalar proof must terminate");
    Ok(())
}
