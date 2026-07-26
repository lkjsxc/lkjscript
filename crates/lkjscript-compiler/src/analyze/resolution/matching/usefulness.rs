use super::usefulness_matrix::{
    constructor, constructors, default_matrix, specialize_matrix, specialize_pattern,
};
use crate::analyze::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Constructor {
    Bool(bool),
    I64(i64),
    Variant(VariantId),
    Product(ProductId),
}

#[derive(Clone, Debug)]
pub(super) enum Witness {
    Wild(Type),
    Constructor(Type, String, Vec<Witness>),
}

pub(super) struct Usefulness<'a> {
    pub(super) enums: &'a [EnumDefinition],
    pub(super) products: &'a [ProductDefinition],
    pub(super) work: u64,
    pub(super) limit: u64,
}

impl<'a> Usefulness<'a> {
    pub(super) fn new(
        enums: &'a [EnumDefinition],
        products: &'a [ProductDefinition],
        limit: u64,
    ) -> Self {
        Self {
            enums,
            products,
            work: 0,
            limit,
        }
    }

    pub(super) fn useful(
        &mut self,
        matrix: &[Vec<MatchPattern>],
        vector: &[MatchPattern],
        types: &[Type],
    ) -> Result<Option<Vec<Witness>>> {
        self.work = self
            .work
            .checked_add(1)
            .ok_or_else(|| Error::msg("match usefulness work overflow"))?;
        if self.work > self.limit {
            return Err(Error::msg(
                "match usefulness specialization reservation exceeded",
            ));
        }
        if vector.is_empty() {
            return Ok(matrix.is_empty().then(Vec::new));
        }
        let ty = types
            .first()
            .ok_or_else(|| Error::msg("match usefulness lost column type"))?;
        if let Some(constructor) = constructor(&vector[0]) {
            return self.specialized(matrix, vector, types, ty, constructor);
        }
        let present = constructors(matrix);
        if let Some(all) = self.complete_space(ty)? {
            if all.iter().all(|item| present.contains(item)) {
                for constructor in all {
                    if let Some(witness) =
                        self.specialized(matrix, vector, types, ty, constructor)?
                    {
                        return Ok(Some(witness));
                    }
                }
                return Ok(None);
            }
        }
        let defaults = default_matrix(matrix);
        let Some(mut tail) = self.useful(&defaults, &vector[1..], &types[1..])? else {
            return Ok(None);
        };
        tail.insert(0, self.missing_witness(ty, &present)?);
        Ok(Some(tail))
    }

    fn specialized(
        &mut self,
        matrix: &[Vec<MatchPattern>],
        vector: &[MatchPattern],
        types: &[Type],
        ty: &Type,
        constructor: Constructor,
    ) -> Result<Option<Vec<Witness>>> {
        let field_types = self.field_types(ty, &constructor)?;
        let specialized = specialize_matrix(matrix, &constructor, &field_types);
        let mut candidate = specialize_pattern(&vector[0], &constructor, &field_types)
            .ok_or_else(|| Error::msg("candidate constructor specialization failed"))?;
        candidate.extend_from_slice(&vector[1..]);
        let mut next_types = field_types.clone();
        next_types.extend_from_slice(&types[1..]);
        let Some(mut witness) = self.useful(&specialized, &candidate, &next_types)? else {
            return Ok(None);
        };
        let fields: Vec<_> = witness.drain(..field_types.len()).collect();
        witness.insert(
            0,
            Witness::Constructor(ty.clone(), self.label(ty, &constructor)?, fields),
        );
        Ok(Some(witness))
    }
}
