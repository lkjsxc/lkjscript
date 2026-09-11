//! Exact finite effect rows. These describe requirements and never contain execution grants.
use super::{EffectParameterReference, RequirementReference};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(
    Clone, Debug, Default, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(deny_unknown_fields)]
pub struct EffectRow {
    pub requirements: Vec<RequirementReference>,
    pub parameters: Vec<EffectParameterReference>,
}

impl EffectRow {
    /// Authored unions normalize; strict owner/type decoding calls validate without normalization.
    pub fn normalize(&mut self) -> Result<(), Diagnostic> {
        self.requirements.sort_unstable();
        self.requirements.dedup();
        self.parameters.sort_unstable();
        self.parameters.dedup();
        self.check_count()
    }

    pub fn validate(&self) -> Result<(), Diagnostic> {
        self.check_count()?;
        if self.requirements.windows(2).any(|p| p[0] >= p[1])
            || self.parameters.windows(2).any(|p| p[0] >= p[1])
        {
            return Err(error(
                "kernel_effect_row_order",
                "effect row atoms must be sorted and unique by exact identity",
            ));
        }
        Ok(())
    }

    fn check_count(&self) -> Result<(), Diagnostic> {
        if self
            .requirements
            .len()
            .checked_add(self.parameters.len())
            .is_none_or(|n| n > super::contract::MAXIMUM_CHILDREN)
        {
            return Err(error(
                "kernel_effect_row_count",
                "effect row exceeds the canonical atom bound",
            ));
        }
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        self.parameters.is_empty()
    }

    /// Bounded exact identity paths for diagnostics. Names and deployment data are absent.
    pub(crate) fn diagnostic(&self) -> String {
        let count = self
            .requirements
            .len()
            .saturating_add(self.parameters.len());
        let atoms = self
            .requirements
            .iter()
            .map(|r| format!("requirement {}/{}", r.package, r.requirement))
            .chain(
                self.parameters
                    .iter()
                    .map(|p| format!("parameter {}/{}", p.package, p.parameter)),
            )
            .take(4)
            .collect::<Vec<_>>()
            .join(", ");
        if count > 4 {
            format!("{{{atoms}, … {} further atoms}}", count - 4)
        } else {
            format!("{{{atoms}}}")
        }
    }

    /// Simultaneous rank-one substitution: a supplied caller row is not resubstituted as a callee row.
    /// Admission runs before any result growth, and receives the number of atom occurrences.
    pub fn substitute(
        &self,
        bindings: &BTreeMap<EffectParameterReference, EffectRow>,
        mut admit: impl FnMut(usize) -> Result<(), Diagnostic>,
    ) -> Result<Self, Diagnostic> {
        self.validate()?;
        let mut count = self.requirements.len();
        for parameter in &self.parameters {
            let row = bindings.get(parameter).ok_or_else(|| {
                error(
                    "kernel_effect_parameter_scope",
                    format!(
                        "effect parameter {}/{} has no exact substitution",
                        parameter.package, parameter.parameter
                    ),
                )
            })?;
            row.validate()?;
            count = count
                .checked_add(row.requirements.len())
                .and_then(|n| n.checked_add(row.parameters.len()))
                .ok_or_else(|| {
                    error(
                        "kernel_effect_row_count",
                        "effect substitution size overflow",
                    )
                })?;
        }
        // Intermediate occurrence work and storage are bounded, even when the union collapses.
        admit(count)?;
        let mut result = Self {
            requirements: self.requirements.clone(),
            parameters: Vec::new(),
        };
        for parameter in &self.parameters {
            let row = &bindings[parameter];
            result.requirements.extend_from_slice(&row.requirements);
            result.parameters.extend_from_slice(&row.parameters);
        }
        result.normalize()?;
        Ok(result)
    }

    /// Symbolic inclusion is exact; the caller owns the separately validated concrete coverage relation.
    pub fn is_contained_by(
        &self,
        available: &Self,
        mut covers: impl FnMut(RequirementReference, RequirementReference) -> Result<bool, Diagnostic>,
    ) -> Result<bool, Diagnostic> {
        self.validate()?;
        available.validate()?;
        if self
            .parameters
            .iter()
            .any(|p| available.parameters.binary_search(p).is_err())
        {
            return Ok(false);
        }
        for required in &self.requirements {
            if available.requirements.binary_search(required).is_ok() {
                continue;
            }
            let mut covered = false;
            for allowance in &available.requirements {
                if covers(*required, *allowance)? {
                    covered = true;
                    break;
                }
            }
            if !covered {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::kernel::{
        PackageId, TypeForm, TypeObject, decode_type_object, encode_type_object,
    };
    use crate::platform::semantic_id::{EffectParameterId, RequirementId};

    fn row(mask: u8) -> EffectRow {
        let package = PackageId::from_bytes([1; 16]).unwrap();
        EffectRow {
            requirements: (0..3)
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| RequirementReference {
                    package,
                    requirement: RequirementId::from_bytes([i + 1; 16]).unwrap(),
                })
                .collect(),
            parameters: (0..2)
                .filter(|i| mask & (8 << i) != 0)
                .map(|i| EffectParameterReference {
                    package,
                    parameter: EffectParameterId::from_bytes([i + 1; 16]).unwrap(),
                })
                .collect(),
        }
    }

    #[test]
    fn finite_bitset_oracle_checks_union_substitution_and_exact_symbolic_containment() {
        for required in 0..32 {
            for available in 0..32 {
                assert_eq!(
                    row(required)
                        .is_contained_by(&row(available), |_, _| Ok(false))
                        .unwrap(),
                    required & !available == 0
                );
                let mut duplicate = row(required);
                duplicate
                    .requirements
                    .extend(row(available).requirements.into_iter().rev());
                duplicate
                    .parameters
                    .extend(row(available).parameters.into_iter().rev());
                duplicate.normalize().unwrap();
                assert_eq!(duplicate, row(required | available));
            }
        }
        for first in 0..32 {
            for second in 0..32 {
                let scope = row(24).parameters;
                let bindings = BTreeMap::from([(scope[0], row(first)), (scope[1], row(second))]);
                let mut admitted = 0;
                let result = row(25)
                    .substitute(&bindings, |n| {
                        admitted += n;
                        Ok(())
                    })
                    .unwrap();
                assert_eq!(result, row(1 | first | second));
                assert_eq!(
                    admitted,
                    1 + first.count_ones() as usize + second.count_ones() as usize
                );
            }
        }
        assert!(row(8).substitute(&BTreeMap::new(), |_| Ok(())).is_err());
        assert!(
            row(1)
                .substitute(&BTreeMap::new(), |_| Err(error("probe", "refused")))
                .is_err()
        );
        let mut foreign = row(8);
        foreign.parameters[0].package = PackageId::from_bytes([2; 16]).unwrap();
        assert!(!foreign.is_contained_by(&row(8), |_, _| Ok(true)).unwrap());
    }

    #[test]
    fn repeated_finite_unions_charge_occurrences_without_expanding_the_canonical_universe() {
        let mut duplicate = row(1);
        duplicate.requirements =
            vec![duplicate.requirements[0]; super::super::contract::MAXIMUM_CHILDREN + 1];
        duplicate.normalize().unwrap();
        assert_eq!(duplicate, row(1));
        let scope = row(24).parameters;
        let mut closed = row(1);
        closed.requirements = (1..=super::super::contract::MAXIMUM_CHILDREN)
            .map(|n| RequirementReference {
                package: closed.requirements[0].package,
                requirement: RequirementId::migrate(b"finite-effect-universe", n as u64),
            })
            .collect();
        closed.normalize().unwrap();
        let bindings = BTreeMap::from([(scope[0], closed.clone()), (scope[1], closed.clone())]);
        let mut occurrences = 0;
        let result = row(24)
            .substitute(&bindings, |n| {
                occurrences = n;
                Ok(())
            })
            .unwrap();
        assert_eq!(result, closed);
        assert_eq!(occurrences, 2 * super::super::contract::MAXIMUM_CHILDREN);
    }

    #[test]
    fn task_encoding_is_disjoint_and_rehashed_noncanonical_rows_reject() {
        let integer = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
            .unwrap()
            .0;
        let pure = TypeObject::new(TypeForm::Function {
            parameters: vec![integer],
            result: integer,
        })
        .unwrap();
        let (pure_digest, pure_bytes) = encode_type_object(&pure).unwrap();
        assert_eq!(&pure_bytes[..8], b"LKJTYP10");
        for mask in 0..32 {
            let task = TypeObject::new(TypeForm::TaskFunction {
                parameters: vec![integer],
                result: integer,
                effect: row(mask),
            })
            .unwrap();
            let (digest, bytes) = encode_type_object(&task).unwrap();
            assert_eq!(&bytes[..8], b"LKJTFN01");
            assert_ne!(digest, pure_digest);
            assert_eq!(decode_type_object(&bytes, digest).unwrap(), task);
            let alternate = crate::platform::packed::encode(
                super::super::contract::TYPE_OBJECT_MAGIC,
                super::super::contract::TYPE_OBJECT_ENVELOPE_DOMAIN,
                &task,
                super::super::contract::MAXIMUM_TYPE_OBJECT_BYTES,
            )
            .unwrap();
            assert!(
                decode_type_object(
                    &alternate,
                    crate::platform::kernel::TypeObjectDigest::of(&alternate)
                )
                .is_err()
            );
        }
        let mut duplicate = row(1);
        duplicate.requirements.push(duplicate.requirements[0]);
        assert!(duplicate.validate().is_err());
        assert!(
            TypeObject::new(TypeForm::TaskFunction {
                parameters: vec![],
                result: integer,
                effect: duplicate
            })
            .is_err()
        );
        assert_eq!(
            encode_type_object(&pure).unwrap(),
            (pure_digest, pure_bytes)
        );
    }
}
