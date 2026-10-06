//! Private read custody between joined invocations. This is never a raw value.
use super::super::super::value::MemoryForm;
use super::{
    ExecutionControl, ExecutionError, FunctionIndex, NormalizedProgram, NormalizedValue,
    TaskApplication, TypeForm, TypeObjectDigest, ValueOrigin,
};
use std::sync::Arc;

/// Retains evaluator-owned checked values and their ancestor cleanup guards.
/// Its contents are inaccessible: a clone shares cleanup custody, not ownership.
pub(in super::super::super) struct ScopedReadCustody {
    _retained: Box<dyn Send + Sync>,
}

impl ScopedReadCustody {
    pub(in super::super::super) fn new<T: Send + Sync + 'static>(retained: T) -> Arc<Self> {
        Arc::new(Self {
            _retained: Box::new(retained),
        })
    }

    /// Evaluators reserve this wrapper and their retained value before minting it.
    pub(in super::super::super) const ALLOCATION_BYTES: u64 =
        (std::mem::size_of::<Self>() + 2 * std::mem::size_of::<usize>()) as u64;
}

pub(in super::super::super) enum ScopedArgument {
    Value(NormalizedValue),
    Read {
        value: NormalizedValue,
        custody: Arc<ScopedReadCustody>,
    },
}

/// A sealed lineage edge whose lifetime extends beyond child frame replacement.
/// Drop it only after every child operand, activation and nested child has closed.
pub(in super::super::super) struct ScopedReadLease {
    position: usize,
    program: ValueOrigin,
    source: ValueOrigin,
    destination: ValueOrigin,
    ty: TypeObjectDigest,
    function: FunctionIndex,
    types: Arc<[TypeObjectDigest]>,
    // An inert token binds the allocation without acquiring any extra rights.
    backing: NormalizedValue,
    _custody: Arc<ScopedReadCustody>,
}

impl ScopedReadLease {
    #[allow(
        clippy::too_many_arguments,
        reason = "exact scoped invocation and custody boundary"
    )]
    pub(in super::super::super) fn seal(
        program: &NormalizedProgram,
        source: ValueOrigin,
        destination: ValueOrigin,
        application: &TaskApplication,
        position: usize,
        value: &NormalizedValue,
        custody: Arc<ScopedReadCustody>,
        control: &ExecutionControl,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        if source == destination {
            return Err(super::reject());
        }
        let parameter = program
            .functions
            .get(application.function().0 as usize)
            .filter(|_| application.function().1 == program.value_origin)
            .and_then(|function| function.parameters.get(position))
            .ok_or_else(super::reject)?;
        if parameter.use_mode != crate::platform::kernel::ParameterUse::Borrow
            || parameter.resource_requirement.is_some()
        {
            return Err(super::reject());
        }
        let ty = application.parameter(program, position)?;
        inspect(program, source, ty, value)?;
        Ok(Self {
            position,
            program: program.value_origin,
            source,
            destination,
            ty,
            function: application.function(),
            types: Arc::clone(application.types()),
            backing: value.clone(),
            _custody: custody,
        })
    }

    pub(in super::super::super) fn position(&self) -> usize {
        self.position
    }

    pub(in super::super::super) fn adopt(
        &self,
        program: &NormalizedProgram,
        destination: ValueOrigin,
        application: &TaskApplication,
        value: &mut NormalizedValue,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        if self.program != program.value_origin
            || self.destination != destination
            || self.function != application.function()
            || self.types.as_ref() != application.types().as_ref()
            || self.ty != application.parameter(program, self.position)?
            || self.backing != *value
        {
            return Err(super::reject());
        }
        inspect(program, self.source, self.ty, value)?;
        match value {
            NormalizedValue::ByteBuffer(token) => token.adopt_scoped_read(self.source, destination),
            NormalizedValue::OwnedI64Cell(token) => {
                token.adopt_scoped_read(self.source, destination)
            }
            NormalizedValue::OwnedProduct(token) => {
                token.adopt_scoped_read(self.source, destination)
            }
            NormalizedValue::OwnedChoice(token) => {
                token.adopt_scoped_read(self.source, destination)
            }
            NormalizedValue::OwnedSequence(token) => {
                token.adopt_scoped_read(self.source, destination)
            }
            _ => Err(super::reject()),
        }
    }
}

/// The certificate is first-party typed construction evidence in the frozen
/// backing root. Lending checks its header, never the stored descendants.
fn inspect(
    program: &NormalizedProgram,
    source: ValueOrigin,
    ty: TypeObjectDigest,
    value: &NormalizedValue,
) -> Result<(), ExecutionError> {
    let expected = match &program.types.get(&ty).ok_or_else(super::reject)?.form {
        TypeForm::ByteBuffer => MemoryForm::ByteBuffer,
        TypeForm::OwnedI64Cell => MemoryForm::OwnedI64Cell,
        TypeForm::OwnedProduct { .. } => MemoryForm::Product(ty),
        TypeForm::OwnedChoice { .. } => MemoryForm::Choice(ty),
        TypeForm::OwnedSequence { .. } => MemoryForm::Sequence(ty),
        _ => return Err(super::reject()),
    };
    if value.memory_form() != Some(expected) || !value.memory_is_borrowed() {
        return Err(super::reject());
    }
    value.memory_validate(source, false)?;
    match value {
        NormalizedValue::ByteBuffer(token) => token.validate_admission(program.value_origin),
        NormalizedValue::OwnedI64Cell(token) => token.validate_admission(program.value_origin),
        NormalizedValue::OwnedProduct(token) => token.validate_admission(program.value_origin),
        NormalizedValue::OwnedChoice(token) => token.validate_admission(program.value_origin),
        NormalizedValue::OwnedSequence(token) => token.validate_admission(program.value_origin),
        _ => Err(super::reject()),
    }
}
