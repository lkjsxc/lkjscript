//! Sealed custody between fresh invocations of one prepared program.
//! This physical boundary is shared by evaluators; ordinary shape admission is not.

use super::super::prepare::NormalizedProgram;
use super::super::value::{
    FunctionIndex, MAXIMUM_ADMISSION_ITEMS, NormalizedValue, ValueOrigin, release_raw_values,
};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{FunctionEffect, ParameterUse, TypeForm, TypeObjectDigest};

/// No Clone, payload projection, or unchecked constructor. While this exists,
/// neither invocation possesses its argument owners.
pub(in super::super) struct TransferArguments {
    program: ValueOrigin,
    source: ValueOrigin,
    destination: ValueOrigin,
    function: FunctionIndex,
    values: Option<Vec<NormalizedValue>>,
}

impl TransferArguments {
    /// The caller already owns and charged the argument vector. This read-only
    /// traversal allocates no storage; ordinary admission reserves its own scratch.
    pub(in super::super) fn seal(
        program: &NormalizedProgram,
        source: ValueOrigin,
        destination: ValueOrigin,
        function: FunctionIndex,
        values: Vec<NormalizedValue>,
        control: &ExecutionControl,
        ordinary: &mut impl FnMut(&NormalizedValue, TypeObjectDigest) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        let transfer = Self {
            program: program.value_origin,
            source,
            destination,
            function,
            values: Some(values),
        };
        control.check()?;
        if source == destination || function.1 != program.value_origin {
            return Err(reject());
        }
        let selected = program
            .functions
            .get(function.0 as usize)
            .ok_or_else(reject)?;
        let empty_task = matches!(&selected.effect,
            FunctionEffect::Task { requirements, effect_parameters }
                if requirements.is_empty() && effect_parameters.is_empty());
        if !selected.graph_function
            || !empty_task
            || !selected.task_requirements.is_empty()
            || !selected.type_parameters.is_empty()
            || !selected.effect_parameters.is_empty()
            || !selected.requirement_parameters.is_empty()
            || !selected.implementation_parameters.is_empty()
            || !program.comparable_types.contains(&selected.result)
        {
            return Err(reject());
        }
        let arguments = transfer.values.as_ref().ok_or_else(reject)?;
        if arguments.len() != selected.parameters.len()
            || arguments.len() != selected.parameter_count as usize
        {
            return Err(reject());
        }
        let mut work = Work { control, nodes: 0 };
        let mut seen_owned = false;
        for (argument, parameter) in arguments.iter().zip(selected.parameters.iter()) {
            if parameter.resource_requirement.is_some() {
                return Err(reject());
            }
            let memory = validate_type(program, parameter.ty, 0, &mut work)?;
            if seen_owned && !memory {
                return Err(reject());
            }
            seen_owned |= memory;
            if parameter.use_mode
                != if memory {
                    ParameterUse::Consume
                } else {
                    ParameterUse::Unrestricted
                }
            {
                return Err(reject());
            }
            inspect_value(
                program,
                argument,
                parameter.ty,
                source,
                0,
                &mut work,
                ordinary,
            )?;
        }
        control.check()?;
        Ok(transfer)
    }

    /// Acceptance is already irrevocable. If cancellation interrupts adoption,
    /// this custodian destroys every argument, including partially retagged ones.
    pub(in super::super) fn adopt(
        mut self,
        program: &NormalizedProgram,
        destination: ValueOrigin,
        control: &ExecutionControl,
    ) -> Result<(FunctionIndex, Vec<NormalizedValue>), ExecutionError> {
        control.check()?;
        if self.program != program.value_origin
            || self.function.1 != program.value_origin
            || self.destination != destination
        {
            return Err(reject());
        }
        let mut work = Work { control, nodes: 0 };
        for value in self.values.as_mut().ok_or_else(reject)? {
            adopt_value(value, self.source, destination, 0, &mut work)?;
        }
        control.check()?;
        Ok((self.function, self.values.take().ok_or_else(reject)?))
    }
}

impl Drop for TransferArguments {
    fn drop(&mut self) {
        if let Some(values) = self.values.take() {
            release_raw_values(values);
        }
    }
}

struct Work<'a> {
    control: &'a ExecutionControl,
    nodes: u64,
}
impl Work<'_> {
    fn visit(&mut self, depth: usize) -> Result<(), ExecutionError> {
        self.control.check()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(reject());
        }
        self.nodes = self
            .nodes
            .checked_add(1)
            .filter(|n| *n <= MAXIMUM_ADMISSION_ITEMS)
            .ok_or_else(|| {
                ExecutionError::resource(
                    "normalized_parallel_transfer_items",
                    "structured transfer exceeds finite admission storage",
                )
            })?;
        Ok(())
    }
}

fn validate_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    depth: usize,
    work: &mut Work<'_>,
) -> Result<bool, ExecutionError> {
    work.visit(depth)?;
    match &program.types.get(&ty).ok_or_else(reject)?.form {
        TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => Ok(true),
        TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
            // Include unselected cases: an empty branch grants no hidden authority.
            for field in fields {
                validate_type(program, field.ty, depth + 1, work)?;
            }
            Ok(true)
        }
        // This prepared fixed-point includes all nominal arguments and cases and
        // excludes secret/function/resource/owned types. ordinary_types does not.
        _ if program.comparable_types.contains(&ty) => Ok(false),
        _ => Err(reject()),
    }
}

fn inspect_value(
    program: &NormalizedProgram,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
    source: ValueOrigin,
    depth: usize,
    work: &mut Work<'_>,
    ordinary: &mut impl FnMut(&NormalizedValue, TypeObjectDigest) -> Result<(), ExecutionError>,
) -> Result<(), ExecutionError> {
    work.visit(depth)?;
    match (value, &program.types.get(&ty).ok_or_else(reject)?.form) {
        (NormalizedValue::ByteBuffer(token), TypeForm::ByteBuffer) => token.validate(source, true),
        (NormalizedValue::OwnedI64Cell(token), TypeForm::OwnedI64Cell) => {
            token.validate(source, true)
        }
        (NormalizedValue::OwnedProduct(token), TypeForm::OwnedProduct { fields })
            if token.ty() == ty =>
        {
            token.inspect_transfer(source, |values| {
                if values.len() != fields.len() {
                    return Err(reject());
                }
                for (value, field) in values.iter().zip(fields) {
                    inspect_value(program, value, field.ty, source, depth + 1, work, ordinary)?;
                }
                Ok(())
            })
        }
        (NormalizedValue::OwnedChoice(token), TypeForm::OwnedChoice { cases })
            if token.ty() == ty =>
        {
            let selected = cases.get(token.case() as usize).ok_or_else(reject)?;
            token.inspect_transfer(source, |payload| {
                inspect_value(
                    program,
                    payload,
                    selected.ty,
                    source,
                    depth + 1,
                    work,
                    ordinary,
                )
            })
        }
        _ if value.memory_form().is_none() && program.comparable_types.contains(&ty) => {
            ordinary(value, ty)
        }
        _ => Err(reject()),
    }
}

fn adopt_value(
    value: &mut NormalizedValue,
    source: ValueOrigin,
    destination: ValueOrigin,
    depth: usize,
    work: &mut Work<'_>,
) -> Result<(), ExecutionError> {
    work.visit(depth)?;
    match value {
        NormalizedValue::ByteBuffer(token) => token.adopt_transfer(source, destination),
        NormalizedValue::OwnedI64Cell(token) => token.adopt_transfer(source, destination),
        NormalizedValue::OwnedProduct(token) => {
            token.adopt_transfer(source, destination, |fields| {
                for field in fields {
                    adopt_value(field, source, destination, depth + 1, work)?;
                }
                Ok(())
            })
        }
        NormalizedValue::OwnedChoice(token) => {
            token.adopt_transfer(source, destination, |payload| {
                adopt_value(payload, source, destination, depth + 1, work)
            })
        }
        _ => Ok(()), // Ordinary immutable metadata keeps its exact prepared-program identities.
    }
}

fn reject() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Trap,
        "normalized_parallel_transfer",
        "structured task transfer has a foreign type, target, domain, loan, or payload",
    )
}

#[cfg(test)]
#[path = "vm_transfer_tests.rs"]
mod tests;
