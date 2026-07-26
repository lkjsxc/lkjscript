use super::*;

impl LayoutInterner {
    pub(super) const FIRST_NESTED_IDENTITY: u32 = 32 + u16::MAX as u32 + 1;

    pub(super) fn build(
        program: &lkjscript_ir::Program,
        functions: &[FunctionId],
    ) -> Result<Self, LoweringError> {
        let mut interner = Self {
            identities: HashMap::new(),
            enum_layouts: HashMap::new(),
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
        let needs_system_options = functions.iter().try_fold(false, |needed, function| {
            let item = source_function(program, *function)?;
            Ok::<_, LoweringError>(
                needed
                    || item
                        .blocks
                        .iter()
                        .flat_map(|block| &block.instructions)
                        .any(|instruction| {
                            matches!(
                                instruction.kind,
                                InstructionKind::Runtime {
                                    operation: RuntimeOp::BufSlice,
                                    ..
                                }
                            )
                        }),
            )
        })?;
        if needs_system_options {
            interner.intern(&lkjscript_ir::prelude_contract::option(SsaType::I64))?;
            interner.intern(&lkjscript_ir::prelude_contract::option(SsaType::Str))?;
        }
        let enum_types: Vec<_> = interner.identities.keys().cloned().collect();
        for ty in enum_types {
            if let SsaType::Enum { id, .. } = &ty {
                let layout = program
                    .enums
                    .iter()
                    .find(|item| item.id == *id)
                    .map(|item| item.layout.identity.bytes())
                    .ok_or_else(|| {
                        LoweringError::new(
                            LoweringFailureCode::UnsupportedType,
                            None,
                            "enum type has no stable runtime layout identity",
                        )
                    })?;
                interner.enum_layouts.insert(ty, layout);
            }
        }
        Ok(interner)
    }

    pub(super) fn intern(&mut self, ty: &SsaType) -> Result<(), LoweringError> {
        match ty {
            SsaType::List(inner) => self.intern(inner)?,
            SsaType::Enum { arguments, .. } => {
                for argument in arguments {
                    self.intern(argument)?;
                }
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

    pub(super) fn enum_layout(&self, ty: &SsaType) -> Option<[u8; 32]> {
        self.enum_layouts.get(ty).copied()
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
            SsaType::List(_) | SsaType::Enum { .. } => self.identities.get(ty).copied(),
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
