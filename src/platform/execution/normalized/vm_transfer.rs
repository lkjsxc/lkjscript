//! Sealed custody between fresh invocations of one prepared program.
//! This physical boundary is shared by evaluators; ordinary shape admission is not.
use super::super::prepare::NormalizedProgram;
use super::super::value::{
    FunctionIndex, MAXIMUM_ADMISSION_ITEMS, NormalizedValue, ValueOrigin, release_raw_values,
};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{FunctionEffect, ParameterUse, TypeForm, TypeObjectDigest};

#[path = "vm_transfer_application.rs"]
mod application;
pub(in super::super) use application::TaskApplication;

#[path = "vm_transfer_type.rs"]
mod types;
pub(in super::super) use types::resolve as resolve_type;

#[path = "scoped_read.rs"]
mod scoped_read;
pub(in super::super) use scoped_read::{ScopedArgument, ScopedReadCustody, ScopedReadLease};

/// No Clone, payload projection or unchecked constructor. Pending custody owns
/// both the exact application and every argument, including partial adoptions.
pub(in super::super) struct TransferArguments {
    program: ValueOrigin,
    source: ValueOrigin,
    destination: ValueOrigin,
    application: Option<TaskApplication>,
    values: Option<Vec<NormalizedValue>>,
    reads: Vec<ScopedReadLease>,
}
impl TransferArguments {
    #[cfg(test)]
    pub(in super::super) fn seal(
        program: &NormalizedProgram,
        source: ValueOrigin,
        destination: ValueOrigin,
        function: FunctionIndex,
        values: Vec<NormalizedValue>,
        control: &ExecutionControl,
        ordinary: &mut impl FnMut(&NormalizedValue, TypeObjectDigest) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        let application = match TaskApplication::bind(
            program,
            function,
            std::sync::Arc::from([]),
            control,
            &mut |_| Ok(()),
        ) {
            Ok(application) => application,
            Err(error) => {
                release_raw_values(values);
                return Err(error);
            }
        };
        Self::seal_applied(
            program,
            source,
            destination,
            application,
            values,
            control,
            ordinary,
        )
    }

    #[cfg(test)]
    pub(in super::super) fn seal_applied(
        program: &NormalizedProgram,
        source: ValueOrigin,
        destination: ValueOrigin,
        application: TaskApplication,
        values: Vec<NormalizedValue>,
        control: &ExecutionControl,
        ordinary: &mut impl FnMut(&NormalizedValue, TypeObjectDigest) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        let transfer = Self {
            program: program.value_origin,
            source,
            destination,
            application: Some(application),
            values: Some(values),
            reads: Vec::new(),
        };
        let application = transfer.application.as_ref().ok_or_else(reject)?;
        control.check()?;
        if source == destination || application.function().1 != program.value_origin {
            return Err(reject());
        }
        let selected = program
            .functions
            .get(application.function().0 as usize)
            .ok_or_else(reject)?;
        let arguments = transfer.values.as_ref().ok_or_else(reject)?;
        if arguments.len() != selected.parameters.len()
            || arguments.len() != selected.parameter_count as usize
        {
            return Err(reject());
        }
        let mut work = Work { control, nodes: 0 };
        validate_type(program, application.result(), 0, &mut work)?;
        let mut seen_owned = false;
        for (index, (argument, parameter)) in
            arguments.iter().zip(selected.parameters.iter()).enumerate()
        {
            let ty = application.parameter(program, index)?;
            if parameter.resource_requirement.is_some() {
                return Err(reject());
            }
            let memory = validate_type(program, ty, 0, &mut work)?;
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
            inspect_value(program, argument, ty, source, 0, &mut work, ordinary)?;
        }
        control.check()?;
        Ok(transfer)
    }

    #[cfg(test)]
    pub(in super::super) fn adopt(
        self,
        program: &NormalizedProgram,
        destination: ValueOrigin,
        control: &ExecutionControl,
    ) -> Result<(FunctionIndex, Vec<NormalizedValue>), ExecutionError> {
        self.adopt_applied(program, destination, control)
            .map(|(a, values)| (a.function(), values))
    }

    #[cfg(test)]
    pub(in super::super) fn adopt_applied(
        mut self,
        program: &NormalizedProgram,
        destination: ValueOrigin,
        control: &ExecutionControl,
    ) -> Result<(TaskApplication, Vec<NormalizedValue>), ExecutionError> {
        if !self.reads.is_empty() {
            return Err(reject());
        }
        control.check()?;
        if self.program != program.value_origin
            || self.application.as_ref().ok_or_else(reject)?.function().1 != program.value_origin
            || self.destination != destination
        {
            return Err(reject());
        }
        let mut work = Work { control, nodes: 0 };
        for value in self.values.as_mut().ok_or_else(reject)? {
            adopt_value(value, self.source, destination, 0, &mut work)?;
        }
        control.check()?;
        Ok((
            self.application.take().ok_or_else(reject)?,
            self.values.take().ok_or_else(reject)?,
        ))
    }

    /// Mode-aware child custody. A borrowed root is certified without walking
    /// its payload; consuming carriers retain the complete recursive boundary.
    pub(in super::super) fn seal_scoped_applied(
        program: &NormalizedProgram,
        source: ValueOrigin,
        destination: ValueOrigin,
        application: TaskApplication,
        values: Vec<ScopedArgument>,
        control: &ExecutionControl,
        ordinary: &mut impl FnMut(&NormalizedValue, TypeObjectDigest) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        // The envelope takes cleanup ownership before any admission can fail.
        let mut transfer = Self {
            program: program.value_origin,
            source,
            destination,
            application: Some(application),
            values: Some(Vec::with_capacity(values.len())),
            reads: Vec::with_capacity(values.len()),
        };
        control.check()?;
        let application = transfer.application.as_ref().ok_or_else(reject)?;
        if source == destination || application.function().1 != program.value_origin {
            return Err(reject());
        }
        let selected = program
            .functions
            .get(application.function().0 as usize)
            .ok_or_else(reject)?;
        if values.len() != selected.parameters.len()
            || values.len() != selected.parameter_count as usize
        {
            return Err(reject());
        }
        let mut work = Work { control, nodes: 0 };
        validate_type(program, application.result(), 0, &mut work)?;
        let mut seen_owned = false;
        for (index, (argument, parameter)) in values
            .into_iter()
            .zip(selected.parameters.iter())
            .enumerate()
        {
            let ty = application.parameter(program, index)?;
            if parameter.resource_requirement.is_some() {
                return Err(reject());
            }
            let (value, lease, memory) = match (parameter.use_mode, argument) {
                (ParameterUse::Borrow, ScopedArgument::Read { value, custody }) => {
                    if !validate_shareable_type(program, ty, 0, &mut work)? {
                        return Err(reject());
                    }
                    let lease = ScopedReadLease::seal(
                        program,
                        source,
                        destination,
                        application,
                        index,
                        &value,
                        custody,
                        control,
                    )?;
                    (value, Some(lease), true)
                }
                (ParameterUse::Consume, ScopedArgument::Value(value)) => {
                    if !validate_type(program, ty, 0, &mut work)? {
                        return Err(reject());
                    }
                    inspect_value(program, &value, ty, source, 0, &mut work, ordinary)?;
                    (value, None, true)
                }
                (ParameterUse::Unrestricted, ScopedArgument::Value(value)) => {
                    if validate_type(program, ty, 0, &mut work)? {
                        return Err(reject());
                    }
                    inspect_value(program, &value, ty, source, 0, &mut work, ordinary)?;
                    (value, None, false)
                }
                _ => return Err(reject()),
            };
            if seen_owned && !memory {
                return Err(reject());
            }
            seen_owned |= memory;
            transfer.values.as_mut().ok_or_else(reject)?.push(value);
            if let Some(lease) = lease {
                transfer.reads.push(lease);
            }
        }
        control.check()?;
        Ok(transfer)
    }

    /// Returned leases outlive the complete child Machine, including tail calls.
    pub(in super::super) fn adopt_scoped_applied(
        mut self,
        program: &NormalizedProgram,
        destination: ValueOrigin,
        control: &ExecutionControl,
    ) -> Result<(TaskApplication, Vec<NormalizedValue>, Vec<ScopedReadLease>), ExecutionError> {
        control.check()?;
        if self.program != program.value_origin
            || self.application.as_ref().ok_or_else(reject)?.function().1 != program.value_origin
            || self.destination != destination
        {
            return Err(reject());
        }
        let values = self.values.as_mut().ok_or_else(reject)?;
        let mut work = Work { control, nodes: 0 };
        let mut leases = self.reads.iter().peekable();
        for (index, value) in values.iter_mut().enumerate() {
            match leases.peek().filter(|lease| lease.position() == index) {
                Some(lease) => lease.adopt(
                    program,
                    destination,
                    self.application.as_ref().ok_or_else(reject)?,
                    value,
                    control,
                )?,
                None => adopt_value(value, self.source, destination, 0, &mut work)?,
            }
            if leases.peek().is_some_and(|lease| lease.position() == index) {
                leases.next();
            }
        }
        if leases.next().is_some() {
            return Err(reject());
        }
        control.check()?;
        Ok((
            self.application.take().ok_or_else(reject)?,
            self.values.take().ok_or_else(reject)?,
            std::mem::take(&mut self.reads),
        ))
    }
}
impl Drop for TransferArguments {
    fn drop(&mut self) {
        if let Some(values) = self.values.take() {
            release_raw_values(values);
        }
        // Release every leaf token before the retained ancestor custody.
        self.reads.clear();
    }
}

/// Return custody retains the exact instantiated task, not just its declaration.
/// No child-local cleanup can dispose of the value while this custodian holds it.
pub(in super::super) struct TransferResult {
    program: ValueOrigin,
    source: ValueOrigin,
    destination: ValueOrigin,
    application: TaskApplication,
    ty: TypeObjectDigest,
    value: Option<NormalizedValue>,
}
impl TransferResult {
    #[cfg(test)]
    #[allow(
        clippy::too_many_arguments,
        reason = "negative controls retain explicit monomorphic identity"
    )]
    pub(in super::super) fn seal(
        program: &NormalizedProgram,
        source: ValueOrigin,
        destination: ValueOrigin,
        function: FunctionIndex,
        ty: TypeObjectDigest,
        value: NormalizedValue,
        control: &ExecutionControl,
        ordinary: &mut impl FnMut(&NormalizedValue, TypeObjectDigest) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        let application = match TaskApplication::bind(
            program,
            function,
            std::sync::Arc::from([]),
            control,
            &mut |_| Ok(()),
        ) {
            Ok(application) if application.result() == ty => application,
            Ok(_) => {
                super::super::value::release_raw_value(value);
                return Err(reject());
            }
            Err(error) => {
                super::super::value::release_raw_value(value);
                return Err(error);
            }
        };
        Self::seal_applied(
            program,
            source,
            destination,
            application,
            value,
            control,
            ordinary,
        )
    }

    pub(in super::super) fn seal_applied(
        program: &NormalizedProgram,
        source: ValueOrigin,
        destination: ValueOrigin,
        application: TaskApplication,
        value: NormalizedValue,
        control: &ExecutionControl,
        ordinary: &mut impl FnMut(&NormalizedValue, TypeObjectDigest) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        let transfer = Self {
            program: program.value_origin,
            source,
            destination,
            ty: application.result(),
            application,
            value: Some(value),
        };
        control.check()?;
        if source == destination || transfer.application.function().1 != program.value_origin {
            return Err(reject());
        }
        let mut work = Work { control, nodes: 0 };
        validate_type(program, transfer.ty, 0, &mut work)?;
        inspect_value(
            program,
            transfer.value.as_ref().ok_or_else(reject)?,
            transfer.ty,
            source,
            0,
            &mut work,
            ordinary,
        )?;
        control.check()?;
        Ok(transfer)
    }

    #[cfg(test)]
    pub(in super::super) fn adopt(
        self,
        program: &NormalizedProgram,
        destination: ValueOrigin,
        function: FunctionIndex,
        ty: TypeObjectDigest,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.adopt_applied(program, destination, function, &[], ty, control)
    }

    pub(in super::super) fn adopt_applied(
        mut self,
        program: &NormalizedProgram,
        destination: ValueOrigin,
        function: FunctionIndex,
        types: &[TypeObjectDigest],
        ty: TypeObjectDigest,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        if self.program != program.value_origin
            || self.application.function() != function
            || function.1 != program.value_origin
            || self.destination != destination
            || self.ty != ty
            || self.application.types().as_ref() != types
            || self.application.result() != ty
            || !self.application.matches(program)
        {
            return Err(reject());
        }
        let mut work = Work { control, nodes: 0 };
        adopt_value(
            self.value.as_mut().ok_or_else(reject)?,
            self.source,
            destination,
            0,
            &mut work,
        )?;
        control.check()?;
        self.value.take().ok_or_else(reject)
    }
}
impl Drop for TransferResult {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            super::super::value::release_raw_value(value);
        }
    }
}

struct Work<'a> {
    control: &'a ExecutionControl,
    nodes: u64,
}

/// Concrete structural obligation checking for ordinary generic calls. This
/// grants no transfer custody and shares no symbolic proof with the kernel.
pub(in super::super) fn admit_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    control: &ExecutionControl,
) -> Result<bool, ExecutionError> {
    validate_type(program, ty, 0, &mut Work { control, nodes: 0 })
}

/// Read sharing is checked independently from moving, even though today's
/// concrete carriers satisfy both structural contracts.
pub(in super::super) fn admit_shareable_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    control: &ExecutionControl,
) -> Result<bool, ExecutionError> {
    validate_shareable_type(program, ty, 0, &mut Work { control, nodes: 0 })
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
        TypeForm::OwnedSequence { item } => {
            // Empty storage is still an owner and grants no exemption from the
            // complete concrete element contract, including unselected cases.
            if !validate_type(program, *item, depth + 1, work)? {
                return Err(reject());
            }
            Ok(true)
        }
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

fn validate_shareable_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    depth: usize,
    work: &mut Work<'_>,
) -> Result<bool, ExecutionError> {
    work.visit(depth)?;
    match &program.types.get(&ty).ok_or_else(reject)?.form {
        TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => Ok(true),
        TypeForm::OwnedSequence { item } => {
            if !validate_shareable_type(program, *item, depth + 1, work)? {
                return Err(reject());
            }
            Ok(true)
        }
        TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
            for field in fields {
                validate_shareable_type(program, field.ty, depth + 1, work)?;
            }
            Ok(true)
        }
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
        (NormalizedValue::OwnedSequence(token), TypeForm::OwnedSequence { item })
            if token.ty() == ty =>
        {
            token.inspect_transfer(source, |values| {
                for value in values {
                    inspect_value(program, value, *item, source, depth + 1, work, ordinary)?;
                }
                Ok(())
            })
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
    }?;
    // A complete independent shape/type/origin inspection may certify this
    // root. Read-only seal/adoption never reaches this recursive path.
    match value {
        NormalizedValue::ByteBuffer(token) => token.establish_admission(program.value_origin),
        NormalizedValue::OwnedI64Cell(token) => token.establish_admission(program.value_origin),
        NormalizedValue::OwnedProduct(token) => token.establish_admission(program.value_origin),
        NormalizedValue::OwnedChoice(token) => token.establish_admission(program.value_origin),
        NormalizedValue::OwnedSequence(token) => token.establish_admission(program.value_origin),
        _ => Ok(()),
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
        NormalizedValue::OwnedSequence(token) => {
            token.adopt_transfer(source, destination, |values| {
                for value in values {
                    adopt_value(value, source, destination, depth + 1, work)?;
                }
                Ok(())
            })
        }
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
