use super::*;

unit_enum!(MemoryUseKind {
    Load = 0,
    Move = 1,
    BorrowSource = 2,
    DirectCallTarget = 3,
    IndirectCallTarget = 4,
});
unit_enum!(MemoryWitnessOperation { Transport = 0 });
canonical_struct!(MemoryWitnessParameter {
    parameter,
    operations
});
canonical_struct!(MemoryWitnessArgument { parameter, witness });
canonical_struct!(MemoryOrigin { source, expression });
canonical_struct!(MemoryPlanEntry {
    id,
    subject,
    ty,
    effects,
    mode,
    type_fact,
    root_projection,
    destination,
    copy_share,
    borrow_scope,
    drop_path,
    execution,
    execution_cutover,
    origin,
    drop_glue,
});
canonical_struct!(FunctionMemorySignature {
    function,
    witness_parameters,
    parameters,
    result
});
canonical_struct!(FunctionMemoryPlan {
    id,
    name,
    binding,
    source,
    signature,
    parameter_entries,
    result_entry,
    body,
});
canonical_struct!(MemoryUse {
    id,
    function,
    expression,
    binding,
    kind
});
canonical_struct!(MemoryConstantPlan {
    id,
    function,
    expression,
    value
});
canonical_struct!(MemoryCallPlan {
    id,
    function,
    expression,
    target,
    witness_arguments,
    parameters,
    result,
    borrow_scopes,
});

impl Canonical for MemorySubject {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        match self {
            Self::Expression {
                expression,
                parent,
                child_index,
                kind,
            } => {
                output.tag(0)?;
                output.value(expression)?;
                output.value(parent)?;
                output.value(child_index)?;
                output.value(kind)
            }
            Self::Parameter {
                function,
                index,
                binding,
                place,
            } => {
                output.tag(1)?;
                output.value(function)?;
                output.value(index)?;
                output.value(binding)?;
                output.value(place)
            }
            Self::Result { function } => tagged(output, 2, function),
            Self::Place {
                function,
                place,
                binding,
            } => {
                output.tag(3)?;
                output.value(function)?;
                output.value(place)?;
                output.value(binding)
            }
            Self::Loan {
                function,
                place,
                loan,
                expression,
            } => {
                output.tag(4)?;
                output.value(function)?;
                output.value(place)?;
                output.value(loan)?;
                output.value(expression)
            }
            Self::Constant {
                constant,
                expression,
            } => {
                output.tag(5)?;
                output.value(constant)?;
                output.value(expression)
            }
            Self::Call { call, expression } => {
                output.tag(6)?;
                output.value(call)?;
                output.value(expression)
            }
        }
    }
}

impl Canonical for MemoryConstantValue {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        match self {
            Self::I64(value) => tagged(output, 0, value),
            Self::F64(value) => tagged(output, 1, value),
            Self::Bool(value) => tagged(output, 2, value),
            Self::Unit => output.tag(3),
            Self::EmptyList => output.tag(4),
            Self::String(value) => tagged(output, 5, value),
            Self::Bytes(value) => {
                output.tag(6)?;
                output.bytes(value)
            }
            Self::Symbol(value) => tagged(output, 7, value),
        }
    }
}

impl Canonical for MemoryCallTarget {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        match self {
            Self::Direct(function) => tagged(output, 0, function),
            Self::Indirect(binding) => tagged(output, 1, binding),
            Self::Operation(operation) => tagged(output, 2, operation),
        }
    }
}

fn tagged<T: Canonical + ?Sized>(output: &mut Encoder, tag: u8, value: &T) -> Result<()> {
    output.tag(tag)?;
    output.value(value)
}
