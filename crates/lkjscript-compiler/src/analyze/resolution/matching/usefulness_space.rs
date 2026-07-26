use super::usefulness::{Constructor, Usefulness, Witness};
use crate::analyze::*;

impl Usefulness<'_> {
    pub(super) fn complete_space(&self, ty: &Type) -> Result<Option<Vec<Constructor>>> {
        Ok(Some(match ty {
            Type::Bool => vec![Constructor::Bool(false), Constructor::Bool(true)],
            Type::Enum { id, .. } => self
                .enum_def(*id)?
                .variants
                .iter()
                .map(|variant| Constructor::Variant(variant.id))
                .collect(),
            Type::Product(name) => vec![Constructor::Product(self.product(name)?.id)],
            Type::I64 => return Ok(None),
            _ => {
                return Err(Error::msg(format!(
                    "type {ty:?} has no closed match constructor space",
                )))
            }
        }))
    }

    pub(super) fn missing_witness(&self, ty: &Type, present: &[Constructor]) -> Result<Witness> {
        let constructor = if let Some(all) = self.complete_space(ty)? {
            all.into_iter()
                .find(|item| !present.contains(item))
                .ok_or_else(|| Error::msg("complete constructor space has no missing witness"))?
        } else {
            Constructor::I64(missing_i64(present)?)
        };
        let fields = self
            .field_types(ty, &constructor)?
            .into_iter()
            .map(Witness::Wild)
            .collect();
        Ok(Witness::Constructor(
            ty.clone(),
            self.label(ty, &constructor)?,
            fields,
        ))
    }

    pub(super) fn field_types(&self, ty: &Type, constructor: &Constructor) -> Result<Vec<Type>> {
        match (ty, constructor) {
            (Type::Bool, Constructor::Bool(_)) | (Type::I64, Constructor::I64(_)) => Ok(Vec::new()),
            (Type::Product(name), Constructor::Product(id)) => {
                let product = self.product(name)?;
                if product.id != *id {
                    return Err(Error::msg("product pattern constructor identity mismatch"));
                }
                Ok(product
                    .fields
                    .iter()
                    .map(|field| field.ty.clone())
                    .collect())
            }
            (Type::Enum { id, arguments, .. }, Constructor::Variant(variant)) => {
                let definition = self.enum_def(*id)?;
                let selected = definition
                    .variants
                    .iter()
                    .find(|item| item.id == *variant)
                    .ok_or_else(|| Error::msg("match constructor references unknown variant"))?;
                let substitutions: HashMap<_, _> = definition
                    .type_parameters
                    .iter()
                    .cloned()
                    .zip(arguments.iter().cloned())
                    .collect();
                Ok(selected
                    .fields
                    .iter()
                    .map(|field| field.ty.subst(&substitutions))
                    .collect())
            }
            _ => Err(Error::msg("match constructor/type mismatch")),
        }
    }

    pub(super) fn label(&self, ty: &Type, constructor: &Constructor) -> Result<String> {
        Ok(match constructor {
            Constructor::Bool(value) => value.to_string(),
            Constructor::I64(value) => value.to_string(),
            Constructor::Product(id) => format!("product#{}", id.raw()),
            Constructor::Variant(id) => {
                let enum_id = match ty {
                    Type::Enum { id, .. } => *id,
                    _ => return Err(Error::msg("variant witness type mismatch")),
                };
                let index = self
                    .enum_def(enum_id)?
                    .variants
                    .iter()
                    .position(|item| item.id == *id)
                    .ok_or_else(|| Error::msg("witness variant identity is stale"))?;
                format!("variant#{index}")
            }
        })
    }

    fn enum_def(&self, id: EnumId) -> Result<&EnumDefinition> {
        self.enums
            .iter()
            .find(|item| item.id == id)
            .ok_or_else(|| Error::msg("match type references unknown enum"))
    }

    fn product(&self, name: &str) -> Result<&ProductDefinition> {
        self.products
            .iter()
            .find(|item| item.name == name)
            .ok_or_else(|| Error::msg("match type references unknown product"))
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
                .ok_or_else(|| Error::msg("I64 witness search overflow"))?
        };
    }
    Ok(value)
}
