use super::*;

impl LayoutInterner {
    pub(super) const FIRST_NESTED_IDENTITY: u32 = 32 + u16::MAX as u32 + 1;

    pub(super) fn build(
        program: &lkjscript_ir::Program,
        functions: &[FunctionId],
    ) -> Result<Self, LoweringError> {
        let mut interner = Self {
            identities: HashMap::new(),
            next: Self::FIRST_NESTED_IDENTITY,
        };
        for function in functions {
            let item = source_function(program, *function)?;
            for ty in item
                .signature
                .parameters
                .iter()
                .chain(std::iter::once(item.signature.result.as_ref()))
                .chain(item.blocks.iter().flat_map(|block| {
                    block
                        .parameters
                        .iter()
                        .map(|parameter| &parameter.ty)
                        .chain(block.instructions.iter().map(|instruction| &instruction.ty))
                }))
            {
                interner.intern(ty)?;
            }
        }
        Ok(interner)
    }

    pub(super) fn intern(&mut self, ty: &SsaType) -> Result<(), LoweringError> {
        match ty {
            SsaType::List(inner) | SsaType::Option(inner) => self.intern(inner)?,
            SsaType::Result(ok, error) => {
                self.intern(ok)?;
                self.intern(error)?;
            }
            _ => return Ok(()),
        }
        if !self.identities.contains_key(ty) {
            let identity = LayoutIdentity::new(self.next);
            self.next = self.next.checked_add(1).ok_or_else(|| {
                LoweringError::new(
                    LoweringFailureCode::UnsupportedType,
                    None,
                    "native structural layout identity space exhausted",
                )
            })?;
            self.identities.insert(ty.clone(), identity);
        }
        Ok(())
    }

    pub(super) fn identity(&self, ty: &SsaType) -> Option<LayoutIdentity> {
        match ty {
            SsaType::Unit => Some(ValueType::Unit.layout_identity()),
            SsaType::Bool => Some(ValueType::Bool.layout_identity()),
            SsaType::I64 => Some(ValueType::I64.layout_identity()),
            SsaType::F64 => Some(ValueType::F64.layout_identity()),
            SsaType::Str => Some(ValueType::Reference(ReferenceType::Str).layout_identity()),
            SsaType::Buf => Some(ValueType::Reference(ReferenceType::Buf).layout_identity()),
            SsaType::Product(product) => Some(LayoutIdentity::product(u32::from(product.raw()))),
            SsaType::List(_) | SsaType::Option(_) | SsaType::Result(_, _) => {
                self.identities.get(ty).copied()
            }
            _ => None,
        }
    }
}

pub(super) fn source_function(
    program: &lkjscript_ir::Program,
    function: FunctionId,
) -> Result<&Function, LoweringError> {
    function
        .index()
        .and_then(|index| program.functions.get(index))
        .filter(|item| item.id == function)
        .ok_or_else(|| {
            LoweringError::new(
                LoweringFailureCode::InvalidFunction,
                Some(function),
                "function is absent from the verified program",
            )
        })
}
