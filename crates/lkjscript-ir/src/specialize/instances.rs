use std::collections::{BTreeMap, BTreeSet};

use crate::{FunctionId, GenericInstantiation, IrError};

pub const MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_DECLARATION: usize = 32;
pub const MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_PACKAGE: usize = 1_024;
pub const MAX_NATIVE_TRANSPORT_SPECIALIZATIONS: usize =
    MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_PACKAGE;

pub(super) type NativeInstances = BTreeMap<FunctionId, BTreeSet<GenericInstantiation>>;

pub(super) fn record_instance(
    instances: &mut NativeInstances,
    target: FunctionId,
    instantiation: GenericInstantiation,
) -> crate::Result<()> {
    let target_instances = instances.entry(target).or_default();
    target_instances.insert(instantiation);
    if target_instances.len() > MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_DECLARATION {
        return Err(IrError::new(
            "native transport specialization declaration budget exceeded",
        ));
    }
    Ok(())
}

pub(super) fn checked_instance_count(instances: &NativeInstances) -> crate::Result<usize> {
    let count = instances
        .values()
        .try_fold(0usize, |count, target_instances| {
            count
                .checked_add(target_instances.len())
                .ok_or_else(|| IrError::new("native specialization instance count overflow"))
        })?;
    if count > MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_PACKAGE {
        return Err(IrError::new(
            "native transport specialization package budget exceeded",
        ));
    }
    Ok(count)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::{MemoryWitnessBinding, MemoryWitnessId, SsaType, TypeSubstitution};

    fn instance(index: usize) -> GenericInstantiation {
        let mut identity = [0u8; 32];
        identity[24..].copy_from_slice(&(index as u64 + 1).to_be_bytes());
        GenericInstantiation {
            substitutions: vec![TypeSubstitution {
                parameter: "t".into(),
                ty: SsaType::I64,
            }],
            witnesses: Vec::new(),
            memory_witnesses: vec![MemoryWitnessBinding {
                parameter: "t".into(),
                witness: MemoryWitnessId::new(identity),
            }],
        }
    }

    #[test]
    fn duplicate_instances_are_canonical_and_declaration_overflow_rejects() {
        let mut instances = BTreeMap::new();
        record_instance(&mut instances, FunctionId::new(0), instance(0)).expect("first instance");
        record_instance(&mut instances, FunctionId::new(0), instance(0))
            .expect("duplicate exact instance");
        assert_eq!(checked_instance_count(&instances).expect("count"), 1);
        for index in 1..MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_DECLARATION {
            record_instance(&mut instances, FunctionId::new(0), instance(index))
                .expect("instance within declaration budget");
        }
        let error = record_instance(
            &mut instances,
            FunctionId::new(0),
            instance(MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_DECLARATION),
        )
        .expect_err("33rd declaration instance must reject");
        assert_eq!(
            error.to_string(),
            "native transport specialization declaration budget exceeded"
        );
    }

    #[test]
    fn package_closure_overflow_rejects_without_weakening_declaration_bound() {
        let mut instances = BTreeMap::new();
        let declarations = MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_PACKAGE
            / MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_DECLARATION;
        for target in 0..declarations {
            for index in 0..MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_DECLARATION {
                record_instance(
                    &mut instances,
                    FunctionId::new(target as u64),
                    instance(index),
                )
                .expect("instance within declaration budget");
            }
        }
        assert_eq!(
            checked_instance_count(&instances).expect("package budget"),
            MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_PACKAGE
        );
        record_instance(
            &mut instances,
            FunctionId::new(declarations as u64),
            instance(0),
        )
        .expect("first instance of another declaration");
        let error = checked_instance_count(&instances).expect_err("package overflow must reject");
        assert_eq!(
            error.to_string(),
            "native transport specialization package budget exceeded"
        );
        assert_eq!(
            MAX_NATIVE_TRANSPORT_SPECIALIZATIONS,
            MAX_NATIVE_TRANSPORT_SPECIALIZATIONS_PER_PACKAGE
        );
    }
}
