use std::collections::HashSet;

use crate::verify::*;
use crate::{IrError, ProductId, Program, SsaType, TraitRole, TraitWitnessKind};

pub(crate) fn verify_witness(
    program: &Program,
    witness: &crate::TraitWitness,
) -> crate::Result<()> {
    let trait_metadata = trait_by_id(program, witness.trait_id)?;
    match witness.kind {
        TraitWitnessKind::AutoTrait => {
            if !trait_metadata.role.is_auto() {
                return fail("SSA auto-trait witness references a non-auto trait");
            }
            let mut work = 0;
            let mut active = HashSet::new();
            if !auto_trait_holds(
                program,
                trait_metadata.role,
                &witness.ty,
                0,
                &mut work,
                &mut active,
            )? {
                return fail("SSA auto-trait witness asserts an unsupported type fact");
            }
        }
        TraitWitnessKind::Explicit(implementation_id) => {
            let implementation = impl_by_id(program, implementation_id)?;
            let SsaType::Product(product) = witness.ty else {
                return fail("SSA explicit marker witness does not target a product");
            };
            if implementation.trait_id != witness.trait_id || implementation.product != product {
                return fail(
                    "SSA explicit marker witness identity does not match trait and product",
                );
            }
        }
    }
    Ok(())
}

pub(crate) fn auto_trait_holds(
    program: &Program,
    role: TraitRole,
    ty: &SsaType,
    depth: usize,
    work: &mut usize,
    active: &mut HashSet<ProductId>,
) -> crate::Result<bool> {
    if depth > TRAIT_VERIFY_MAX_DEPTH {
        return fail(format!(
            "SSA auto-trait verification depth exceeded {TRAIT_VERIFY_MAX_DEPTH}"
        ));
    }
    *work = work
        .checked_add(1)
        .ok_or_else(|| IrError::new("SSA auto-trait work overflow"))?;
    if *work > TRAIT_VERIFY_MAX_WORK {
        return fail(format!(
            "SSA auto-trait verification work exceeded {TRAIT_VERIFY_MAX_WORK}"
        ));
    }
    match role {
        TraitRole::Copy => match ty {
            SsaType::Unit
            | SsaType::Bool
            | SsaType::I64
            | SsaType::F64
            | SsaType::Capability(_)
            | SsaType::Str
            | SsaType::Symbol => Ok(true),
            SsaType::Ref(inner) if inner.as_ref() == &SsaType::Buf => Ok(true),
            SsaType::Buf
            | SsaType::Owned(_)
            | SsaType::Ref(_)
            | SsaType::RefMut(_)
            | SsaType::Handle
            | SsaType::Function(_)
            | SsaType::TypeParameter(_) => Ok(false),
            SsaType::List(inner) => auto_trait_holds(program, role, inner, depth + 1, work, active),
            SsaType::Enum { id, arguments }
                if matches!(
                    id.bytes(),
                    crate::prelude_contract::OPTION_ID | crate::prelude_contract::RESULT_ID
                ) =>
            {
                for argument in arguments {
                    if !auto_trait_holds(program, role, argument, depth + 1, work, active)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            SsaType::Enum { .. } => Ok(false),
            SsaType::Product(product) => {
                if !active.insert(*product) {
                    return fail(format!(
                        "SSA auto-trait verification encountered product cycle at {}",
                        product.raw()
                    ));
                }
                let metadata = product_by_id(program, *product)?;
                let mut result = true;
                for field in &metadata.fields {
                    if !auto_trait_holds(program, role, &field.ty, depth + 1, work, active)? {
                        result = false;
                        break;
                    }
                }
                active.remove(product);
                Ok(result)
            }
        },
        TraitRole::Send | TraitRole::Sync => Ok(matches!(
            ty,
            SsaType::Unit | SsaType::Bool | SsaType::I64 | SsaType::F64
        )),
        TraitRole::Clone | TraitRole::Drop | TraitRole::User => Ok(false),
    }
}
