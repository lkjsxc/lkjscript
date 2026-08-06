impl TypePlanner<'_> {
    fn producer_semantic_type(
        &self,
        ty: &Type,
    ) -> Result<lkjscript_contracts::SemanticType> {
        crate::stack::grow(|| self.producer_semantic_type_inner(ty))
    }

    fn producer_semantic_type_inner(
        &self,
        ty: &Type,
    ) -> Result<lkjscript_contracts::SemanticType> {
        use lkjscript_contracts::{SemanticPrimitiveKind as P, SemanticType as S};
        Ok(match ty {
            Type::Never => S::Primitive(P::Never),
            Type::Unit => S::Primitive(P::Unit),
            Type::Bool => S::Primitive(P::Bool),
            Type::I64 => S::Primitive(P::I64),
            Type::F64 => S::Primitive(P::F64),
            Type::Str => S::Primitive(P::String),
            Type::Bytes => S::Primitive(P::Bytes),
            Type::Path => S::Primitive(P::Path),
            Type::ByteVector => S::Primitive(P::ByteVector),
            Type::ByteSlice => S::Primitive(P::ByteSlice),
            Type::ByteSliceMut => S::Primitive(P::ByteSliceMut),
            Type::Symbol => S::Primitive(P::Symbol),
            Type::Capability(kind) => S::Capability(*kind),
            Type::Resource(kind) => S::Resource(*kind),
            Type::Product(name) => S::Product(self.product(name)?.identity),
            Type::Enum { id, arguments, .. } => S::Enum {
                identity: id.bytes(),
                arguments: arguments
                    .iter()
                    .map(|item| self.producer_semantic_type(item))
                    .collect::<Result<_>>()?,
            },
            Type::Param(name) => S::Parameter(name.clone()),
            Type::List(item) => S::List(Box::new(self.producer_semantic_type(item)?)),
            Type::Fn { params, ret } => S::Function {
                parameters: params
                    .iter()
                    .map(|item| self.producer_semantic_type(item))
                    .collect::<Result<_>>()?,
                result: Box::new(self.producer_semantic_type(ret)?),
            },
            Type::Forall { vars, body } => S::ForAll {
                parameters: vars.clone(),
                body: Box::new(self.producer_semantic_type(body)?),
            },
        })
    }
}
