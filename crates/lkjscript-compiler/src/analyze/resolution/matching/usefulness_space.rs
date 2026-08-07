use crate::analyze::*;

use super::usefulness::{reserve, Constructor, Usefulness, WitnessId, WitnessNode};

impl Usefulness<'_> {
    pub(super) fn complete_space(&self, ty: &Type) -> Result<Option<Vec<Constructor>>> {
        let mut constructors = Vec::new();
        match ty {
            Type::Bool => {
                reserve(&mut constructors, 2, "Bool match constructor space")?;
                constructors.extend([Constructor::Bool(false), Constructor::Bool(true)]);
            }
            Type::Enum { id, .. } => {
                let definition = self.enum_def(*id)?;
                reserve(
                    &mut constructors,
                    definition.variants.len(),
                    "enum match constructor space",
                )?;
                constructors.extend(
                    definition
                        .variants
                        .iter()
                        .map(|variant| Constructor::Variant(variant.id)),
                );
            }
            Type::Product(name) => {
                reserve(&mut constructors, 1, "product match constructor space")?;
                constructors.push(Constructor::Product(self.product(name)?.id));
            }
            Type::I64 => return Ok(None),
            _ => {
                return Err(Error::msg(format!(
                    "type {ty} has no closed match constructor space",
                )))
            }
        }
        Ok(Some(constructors))
    }

    pub(super) fn missing_witness(
        &mut self,
        ty: &Type,
        present: &[Constructor],
    ) -> Result<WitnessId> {
        let constructor = if let Some(all) = self.complete_space(ty)? {
            all.into_iter()
                .find(|item| !present.contains(item))
                .ok_or_else(|| Error::msg("complete constructor space has no missing witness"))?
        } else {
            Constructor::I64(missing_i64(present)?)
        };
        let field_types = self.field_types(ty, &constructor)?;
        let mut fields = Vec::new();
        reserve(
            &mut fields,
            field_types.len(),
            "missing match witness fields",
        )?;
        for field_type in field_types {
            fields.push(self.wild_witness(field_type)?);
        }
        self.push_witness(WitnessNode::Constructor {
            ty: ty.clone(),
            constructor,
            fields,
        })
    }

    pub(super) fn field_types(&self, ty: &Type, constructor: &Constructor) -> Result<Vec<Type>> {
        let mut fields = Vec::new();
        match (ty, constructor) {
            (Type::Bool, Constructor::Bool(_)) | (Type::I64, Constructor::I64(_)) => {}
            (Type::Product(name), Constructor::Product(id)) => {
                let product = self.product(name)?;
                if product.id != *id {
                    return Err(Error::msg("product pattern constructor identity mismatch"));
                }
                reserve(
                    &mut fields,
                    product.fields.len(),
                    "product match constructor fields",
                )?;
                fields.extend(product.fields.iter().map(|field| field.ty.clone()));
            }
            (Type::Enum { id, arguments, .. }, Constructor::Variant(variant)) => {
                let definition = self.enum_def(*id)?;
                let selected = definition
                    .variants
                    .iter()
                    .find(|item| item.id == *variant)
                    .ok_or_else(|| Error::msg("match constructor references unknown variant"))?;
                let mut substitutions = HashMap::new();
                substitutions
                    .try_reserve(definition.type_parameters.len())
                    .map_err(|_| Error::host("match constructor substitution allocation failed"))?;
                substitutions.extend(
                    definition
                        .type_parameters
                        .iter()
                        .cloned()
                        .zip(arguments.iter().cloned()),
                );
                reserve(
                    &mut fields,
                    selected.fields.len(),
                    "enum match constructor fields",
                )?;
                fields.extend(
                    selected
                        .fields
                        .iter()
                        .map(|field| field.ty.subst(&substitutions)),
                );
            }
            _ => return Err(Error::msg("match constructor/type mismatch")),
        }
        Ok(fields)
    }

    pub(super) fn enum_def(&self, id: EnumId) -> Result<&EnumDefinition> {
        self.enums
            .iter()
            .find(|item| item.id == id)
            .ok_or_else(|| Error::msg("match type references unknown enum"))
    }

    pub(super) fn product(&self, name: &str) -> Result<&ProductDefinition> {
        self.products
            .iter()
            .find(|item| item.name == name)
            .ok_or_else(|| Error::msg("match type references unknown product"))
    }

    fn wild_witness(&mut self, ty: Type) -> Result<WitnessId> {
        if let Some(id) = self.wild_witnesses.get(&ty) {
            return Ok(*id);
        }
        let key = ty.clone();
        let id = self.push_witness(WitnessNode::Wild(ty))?;
        self.wild_witnesses
            .try_reserve(1)
            .map_err(|_| Error::host("match wildcard witness cache allocation failed"))?;
        self.wild_witnesses.insert(key, id);
        Ok(id)
    }
}

fn missing_i64(present: &[Constructor]) -> Result<i64> {
    let mut value = 0_i64;
    while present.contains(&Constructor::I64(value)) {
        value = if value > 0 {
            -value
        } else {
            value
                .checked_neg()
                .and_then(|item| item.checked_add(1))
                .ok_or_else(|| Error::host("I64 witness search representation overflow"))?
        };
    }
    Ok(value)
}
