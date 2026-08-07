use super::*;

mod aggregate;
mod catalog;
mod identity;
mod witness_slots;

pub(in crate::lower) use catalog::StructuralCatalog;
pub(in crate::lower) use identity::*;
use witness_slots::native_witness_slots;

impl LayoutInterner {
    pub(super) const FIRST_NESTED_IDENTITY: u64 = 1;

    pub(super) fn build(
        program: &lkjscript_ir::Program,
        functions: &[FunctionId],
    ) -> Result<Self, LoweringError> {
        let mut interner = Self {
            identities: HashMap::new(),
            semantics: HashMap::new(),
            region_products: program
                .region_products
                .iter()
                .map(|metadata| (metadata.product, metadata.identity.bytes()))
                .collect(),
            structural: StructuralCatalog::build(program)?,
            witness_slots: native_witness_slots(program)?,
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
                interner.intern_semantics(program, ty)?;
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
            SsaType::Str | SsaType::Path | SsaType::Product(_) => {}
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

    fn intern_semantics(
        &mut self,
        program: &lkjscript_ir::Program,
        ty: &SsaType,
    ) -> Result<(), LoweringError> {
        match ty {
            SsaType::List(inner) => self.intern_semantics(program, inner)?,
            SsaType::Enum { arguments, .. } => {
                for argument in arguments {
                    self.intern_semantics(program, argument)?;
                }
            }
            SsaType::Str | SsaType::Path | SsaType::Product(_) => {}
            _ => return Ok(()),
        }
        if !self.semantics.contains_key(ty) {
            let semantic = lkjscript_ir::runtime_structural_semantic_type(Some(program), ty)
                .map_err(|error| invalid_structural(&error.to_string()))?
                .get();
            self.semantics.insert(ty.clone(), semantic);
        }
        Ok(())
    }

    pub(super) fn region_product_identity(
        &self,
        product: lkjscript_ir::ProductId,
    ) -> Option<[u8; 32]> {
        self.region_products.get(&product).copied()
    }

    pub(super) const fn structural(&self) -> &StructuralCatalog {
        &self.structural
    }

    pub(super) fn witness_slot(
        &self,
        witness: lkjscript_ir::MemoryWitnessId,
        storage: lkjscript_native::StructuralStorageRoute,
    ) -> Option<u64> {
        self.witness_slots.get(&(witness, storage)).copied()
    }

    pub(super) fn semantic(&self, ty: &SsaType) -> Option<u64> {
        self.structural
            .value_type(ty)
            .map(lkjscript_native::StructuralTypeIdentity::semantic_type)
            .or_else(|| self.semantics.get(ty).copied())
    }

    pub(super) fn identity(&self, ty: &SsaType) -> Option<LayoutIdentity> {
        if let Some(value_type) = self.structural.owner_type(ty) {
            return Some(value_type.layout_identity());
        }
        match ty {
            SsaType::Unit => Some(ValueType::Unit.layout_identity()),
            SsaType::Bool => Some(ValueType::Bool.layout_identity()),
            SsaType::I64 => Some(ValueType::I64.layout_identity()),
            SsaType::F64 => Some(ValueType::F64.layout_identity()),
            SsaType::Product(product) => Some(LayoutIdentity::product(product.raw())),
            SsaType::Str | SsaType::Path | SsaType::List(_) | SsaType::Enum { .. } => {
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
