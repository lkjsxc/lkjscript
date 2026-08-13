use super::*;

impl Canonical for lkjscript_core::CapabilityKind {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        output.tag(*self as u8)
    }
}

impl Canonical for lkjscript_core::ResourceKind {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        output.tag(*self as u8)
    }
}

unit_enum!(MemoryParameterMode {
    Copy = 0,
    BorrowShared = 1,
    BorrowExclusive = 2,
    Consume = 3,
});
unit_enum!(MemoryResultMode { Trivial = 0, Owned = 1, SealedShared = 2, External = 3 });
unit_enum!(MemoryBorrowKind { Shared = 0, Exclusive = 1 });
unit_enum!(MemoryBindingStorage { Local = 0, Function = 1 });

impl Canonical for MemoryType {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        let mut pending = vec![self];
        while let Some(ty) = pending.pop() {
            match ty {
                Self::Never => output.tag(0)?,
                Self::Unit => output.tag(1)?,
                Self::Bool => output.tag(2)?,
                Self::I64 => output.tag(3)?,
                Self::F64 => output.tag(4)?,
                Self::String => output.tag(5)?,
                Self::Bytes => output.tag(6)?,
                Self::Path => output.tag(7)?,
                Self::Capability(kind) => tagged(output, 8, kind)?,
                Self::ByteVector => output.tag(9)?,
                Self::ByteSlice => output.tag(10)?,
                Self::ByteSliceMut => output.tag(11)?,
                Self::Symbol => output.tag(12)?,
                Self::Resource(kind) => tagged(output, 13, kind)?,
                Self::Product(id) => tagged(output, 14, &id.raw())?,
                Self::Enum { id, arguments } => {
                    output.tag(15)?;
                    output.value(id)?;
                    output.value(&u64::try_from(arguments.len()).map_err(|_| {
                        Error::msg("canonical memory type argument count exceeds u64")
                    })?)?;
                    pending.extend(arguments.iter().rev());
                }
                Self::TypeParameter(name) => tagged(output, 16, name)?,
                Self::List(element) => {
                    output.tag(17)?;
                    pending.push(element);
                }
                Self::Function { parameters, result } => {
                    output.tag(18)?;
                    output.value(&u64::try_from(parameters.len()).map_err(|_| {
                        Error::msg("canonical memory type parameter count exceeds u64")
                    })?)?;
                    pending.push(result);
                    pending.extend(parameters.iter().rev());
                }
                Self::ForAll { variables, body } => {
                    output.tag(19)?;
                    output.value(variables)?;
                    pending.push(body);
                }
            }
        }
        Ok(())
    }
}

impl Canonical for MemoryExpressionKind {
    fn encode(&self, output: &mut Encoder) -> Result<()> {
        match self {
            Self::I64Literal(value) => tagged(output, 0, value),
            Self::F64Literal(value) => tagged(output, 1, value),
            Self::BoolLiteral(value) => tagged(output, 2, value),
            Self::UnitLiteral => output.tag(3),
            Self::EmptyList => output.tag(4),
            Self::StringLiteral => output.tag(5),
            Self::BytesLiteral => output.tag(6),
            Self::Load { binding, storage } => {
                output.tag(7)?;
                output.value(binding)?;
                output.value(storage)
            }
            Self::Move { place, binding } => {
                output.tag(8)?;
                output.value(place)?;
                output.value(binding)
            }
            Self::Borrow {
                place,
                loan,
                kind,
                binding,
            } => {
                output.tag(9)?;
                output.value(place)?;
                output.value(loan)?;
                output.value(kind)?;
                output.value(binding)
            }
            Self::DirectCall => output.tag(10),
            Self::IndirectCall => output.tag(11),
            Self::Operation(value) => tagged(output, 12, value),
            Self::F64FromI64Exact => output.tag(13),
            Self::F64FromI64Rounded => output.tag(14),
            Self::I64FromF64Exact => output.tag(15),
            Self::I64FromF64Trunc => output.tag(16),
            Self::Sequence => output.tag(17),
            Self::If => output.tag(18),
            Self::While => output.tag(19),
            Self::Loop => output.tag(20),
            Self::Return => output.tag(21),
            Self::Break => output.tag(22),
            Self::Continue => output.tag(23),
            Self::Trap => output.tag(24),
            Self::Exit => output.tag(25),
            Self::Let => output.tag(26),
            Self::MutableLocal => output.tag(27),
            Self::SetLocal => output.tag(28),
            Self::ProductValue => output.tag(29),
            Self::ProductField => output.tag(30),
            Self::WithProductField => output.tag(31),
            Self::EnumValue => output.tag(32),
            Self::EnumIsVariant => output.tag(33),
            Self::EnumField => output.tag(34),
            Self::EnumUnwrap => output.tag(35),
            Self::MatchUnreachable => output.tag(36),
            Self::SymbolLiteral => output.tag(37),
        }
    }
}

fn tagged<T: Canonical + ?Sized>(output: &mut Encoder, tag: u8, value: &T) -> Result<()> {
    output.tag(tag)?;
    output.value(value)
}
