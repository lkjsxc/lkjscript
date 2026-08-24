//! Exact deployment-grant binding for normalized Graph 5 requirements.

use super::prepare::NormalizedProgram;
use super::value::{NormalizedValue, OperationIndex, RequirementIndex};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{DeclarationReference, OperationReference, RequirementReference};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub trait NormalizedCapabilityAdapter: Send + Sync {
    fn interface(&self) -> DeclarationReference;

    fn call(
        &self,
        operation: OperationReference,
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError>;

    fn begin_transaction(
        &self,
        _control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        Err(ExecutionError::new(
            ExecutionFailureClass::Capability,
            "normalized_transaction_unsupported",
            "the exact capability adapter does not support transactions",
        ))
    }
}

pub trait NormalizedCapabilityTransaction: Send {
    fn call(
        &mut self,
        operation: OperationReference,
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError>;

    fn commit(&mut self, control: &ExecutionControl) -> Result<(), ExecutionError>;

    fn rollback(&mut self) -> Result<(), ExecutionError>;
}

#[derive(Clone)]
pub struct NormalizedCapabilityGrant {
    pub requirement: RequirementReference,
    pub operations: BTreeSet<OperationReference>,
    pub maximum_calls: u64,
    pub adapter: Arc<dyn NormalizedCapabilityAdapter>,
}

impl std::fmt::Debug for NormalizedCapabilityGrant {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NormalizedCapabilityGrant")
            .field("requirement", &self.requirement)
            .field("operations", &self.operations)
            .field("maximum_calls", &self.maximum_calls)
            .field("adapter", &"<opaque>")
            .finish()
    }
}

#[derive(Clone)]
pub struct NormalizedCapabilities {
    bindings: Arc<BTreeMap<RequirementIndex, BoundNormalizedCapability>>,
    exact_bindings: Arc<BTreeMap<RequirementReference, BoundNormalizedCapability>>,
}

impl std::fmt::Debug for NormalizedCapabilities {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NormalizedCapabilities")
            .field("requirements", &self.bindings.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
struct BoundNormalizedCapability {
    operations: BTreeSet<OperationIndex>,
    exact_operations: BTreeSet<OperationReference>,
    maximum_calls: u64,
    adapter: Arc<dyn NormalizedCapabilityAdapter>,
}

impl NormalizedCapabilities {
    pub fn bind(
        program: &NormalizedProgram,
        component: super::value::ComponentIndex,
        grants: Vec<NormalizedCapabilityGrant>,
    ) -> Result<Self, ExecutionError> {
        let component = program
            .components
            .get(component.0 as usize)
            .ok_or_else(|| {
                capability_error(
                    "normalized_component_missing",
                    "deployment names no exact prepared component",
                )
            })?;
        let required = component
            .requirements
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let mut bindings = BTreeMap::new();
        let mut exact_bindings = BTreeMap::new();
        for grant in grants {
            let requirement = program
                .requirements
                .iter()
                .position(|requirement| requirement.reference == grant.requirement)
                .map(|index| RequirementIndex(index as u32))
                .ok_or_else(|| {
                    capability_error(
                        "normalized_grant_requirement",
                        "deployment grant names no exact artifact requirement",
                    )
                })?;
            if !required.contains(&requirement) {
                return Err(capability_error(
                    "normalized_grant_component",
                    "deployment grant is outside the selected component requirement set",
                ));
            }
            let declaration = &program.requirements[requirement.0 as usize];
            if grant.maximum_calls == 0 || grant.adapter.interface() != declaration.interface {
                return Err(capability_error(
                    "normalized_grant_interface",
                    "deployment grant interface or call bound disagrees with the exact requirement",
                ));
            }
            let mut operations = BTreeSet::new();
            for operation in &grant.operations {
                let index = program
                    .operations
                    .iter()
                    .position(|candidate| candidate.reference == *operation)
                    .map(|index| OperationIndex(index as u32))
                    .ok_or_else(|| {
                        capability_error(
                            "normalized_grant_operation",
                            "deployment grant names no exact artifact operation",
                        )
                    })?;
                operations.insert(index);
            }
            if operations != declaration.operations.iter().copied().collect() {
                return Err(capability_error(
                    "normalized_grant_operation_set",
                    "deployment grant must bind the exact required operation set",
                ));
            }
            let exact_requirement = declaration.reference;
            let binding = BoundNormalizedCapability {
                operations,
                exact_operations: grant.operations,
                maximum_calls: grant.maximum_calls,
                adapter: grant.adapter,
            };
            if bindings.insert(requirement, binding.clone()).is_some()
                || exact_bindings.insert(exact_requirement, binding).is_some()
            {
                return Err(capability_error(
                    "normalized_grant_duplicate",
                    "deployment repeats one exact component requirement",
                ));
            }
        }
        if bindings.keys().copied().collect::<BTreeSet<_>>() != required {
            return Err(capability_error(
                "normalized_grant_missing",
                "deployment does not bind every exact selected component requirement",
            ));
        }
        Ok(Self {
            bindings: Arc::new(bindings),
            exact_bindings: Arc::new(exact_bindings),
        })
    }

    pub(crate) fn requires(&self, requirement: RequirementIndex) -> bool {
        self.bindings.contains_key(&requirement)
    }

    pub(crate) fn maximum_calls(
        &self,
        requirement: RequirementIndex,
    ) -> Result<u64, ExecutionError> {
        self.binding(requirement)
            .map(|binding| binding.maximum_calls)
    }

    pub(crate) fn call(
        &self,
        program: &NormalizedProgram,
        requirement: RequirementIndex,
        operation: OperationIndex,
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        let binding = self.binding(requirement)?;
        if !binding.operations.contains(&operation) {
            return Err(capability_error(
                "normalized_capability_operation",
                "execution requested an operation outside the exact deployment grant",
            ));
        }
        let operation = program
            .operations
            .get(operation.0 as usize)
            .ok_or_else(|| {
                capability_error(
                    "normalized_capability_operation_index",
                    "prepared capability operation index is outside the artifact table",
                )
            })?
            .reference;
        binding.adapter.call(operation, arguments, control)
    }

    pub(crate) fn begin_transaction(
        &self,
        requirement: RequirementIndex,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        self.binding(requirement)?
            .adapter
            .begin_transaction(control)
    }

    pub(crate) fn requires_exact(&self, requirement: RequirementReference) -> bool {
        self.exact_bindings.contains_key(&requirement)
    }

    pub(crate) fn maximum_calls_exact(
        &self,
        requirement: RequirementReference,
    ) -> Result<u64, ExecutionError> {
        self.exact_binding(requirement)
            .map(|binding| binding.maximum_calls)
    }

    pub(crate) fn call_exact(
        &self,
        requirement: RequirementReference,
        operation: OperationReference,
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        let binding = self.exact_binding(requirement)?;
        if !binding.exact_operations.contains(&operation) {
            return Err(capability_error(
                "normalized_capability_operation",
                "execution requested an operation outside the exact deployment grant",
            ));
        }
        binding.adapter.call(operation, arguments, control)
    }

    pub(crate) fn begin_transaction_exact(
        &self,
        requirement: RequirementReference,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        self.exact_binding(requirement)?
            .adapter
            .begin_transaction(control)
    }

    fn binding(
        &self,
        requirement: RequirementIndex,
    ) -> Result<&BoundNormalizedCapability, ExecutionError> {
        self.bindings.get(&requirement).ok_or_else(|| {
            capability_error(
                "normalized_capability_unbound",
                "effectful execution has no exact deployment grant for its requirement",
            )
        })
    }

    fn exact_binding(
        &self,
        requirement: RequirementReference,
    ) -> Result<&BoundNormalizedCapability, ExecutionError> {
        self.exact_bindings.get(&requirement).ok_or_else(|| {
            capability_error(
                "normalized_capability_unbound",
                "effectful execution has no exact deployment grant for its requirement",
            )
        })
    }
}

fn capability_error(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Capability, code, message)
}
