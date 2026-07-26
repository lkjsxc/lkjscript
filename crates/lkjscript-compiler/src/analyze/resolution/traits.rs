use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn solve_trait_bound(
        &self,
        function: &str,
        trait_id: TraitId,
        ty: &Type,
    ) -> Result<TraitWitness> {
        let definition = self
            .analyzer
            .traits
            .get(trait_id.index().unwrap_or(usize::MAX))
            .filter(|definition| definition.id == trait_id)
            .ok_or_else(|| {
                self.error(format!(
                    "{function}: bound references unknown TraitId {}",
                    trait_id.raw()
                ))
            })?;
        let kind = if let Some(core_trait) = definition.core.filter(|role| role.is_auto()) {
            let mut work = 0;
            let mut active = HashSet::new();
            let mut memo = HashMap::new();
            match self.auto_trait_holds(core_trait, ty, 0, &mut work, &mut active, &mut memo)? {
                true => TraitWitnessKind::AutoTrait,
                false => {
                    return Err(self.error(format!(
                        "{function}: type {ty:?} does not satisfy trait {}",
                        definition.name
                    )))
                }
            }
        } else {
            let Type::Product(name) = ty else {
                return Err(self.error(format!(
                    "{function}: type {ty:?} has no exact implementation of trait {}",
                    definition.name
                )));
            };
            let product = self
                .analyzer
                .product_names
                .get(name)
                .copied()
                .ok_or_else(|| self.error(format!("{function}: unknown product type {name}")))?;
            let implementation = self
                .analyzer
                .implementation_index
                .get(&(trait_id, product))
                .copied()
                .ok_or_else(|| {
                    self.error(format!(
                        "{function}: product {name} does not implement trait {}",
                        definition.name
                    ))
                })?;
            TraitWitnessKind::Explicit(implementation)
        };
        Ok(TraitWitness {
            trait_id,
            ty: ty.clone(),
            kind,
        })
    }

    pub(in crate::analyze) fn auto_trait_holds(
        &self,
        core_trait: CoreTrait,
        ty: &Type,
        depth: usize,
        work: &mut usize,
        active: &mut HashSet<ProductId>,
        memo: &mut HashMap<Type, bool>,
    ) -> Result<bool> {
        if let Some(result) = memo.get(ty) {
            return Ok(*result);
        }
        if depth > TRAIT_SOLVER_MAX_DEPTH {
            return Err(self.error(format!(
                "trait solver depth exceeded {TRAIT_SOLVER_MAX_DEPTH}"
            )));
        }
        *work = work
            .checked_add(1)
            .ok_or_else(|| self.error("trait solver work overflow"))?;
        if *work > TRAIT_SOLVER_MAX_WORK {
            return Err(self.error(format!(
                "trait solver work exceeded {TRAIT_SOLVER_MAX_WORK}"
            )));
        }
        let result = match core_trait {
            CoreTrait::Copy => match ty {
                Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Symbol => true,
                Type::Ref(inner) if inner.as_ref() == &Type::Buf => true,
                Type::Never
                | Type::Buf
                | Type::Owned(_)
                | Type::Ref(_)
                | Type::RefMut(_)
                | Type::Handle
                | Type::Enum { .. }
                | Type::Fn { .. }
                | Type::Forall { .. }
                | Type::Param(_) => false,
                Type::List(inner) | Type::Option(inner) => {
                    self.auto_trait_holds(core_trait, inner, depth + 1, work, active, memo)?
                }
                Type::Result(ok, error) => {
                    self.auto_trait_holds(core_trait, ok, depth + 1, work, active, memo)?
                        && self.auto_trait_holds(
                            core_trait,
                            error,
                            depth + 1,
                            work,
                            active,
                            memo,
                        )?
                }
                Type::Product(name) => {
                    let product = self.analyzer.product_by_name(name)?;
                    if !active.insert(product.id) {
                        return Err(self.error(format!(
                            "trait solver encountered recursive product cycle at {name}"
                        )));
                    }
                    let mut result = true;
                    for field in &product.fields {
                        if !self.auto_trait_holds(
                            core_trait,
                            &field.ty,
                            depth + 1,
                            work,
                            active,
                            memo,
                        )? {
                            result = false;
                            break;
                        }
                    }
                    active.remove(&product.id);
                    result
                }
            },
            CoreTrait::Send | CoreTrait::Sync => {
                matches!(ty, Type::Unit | Type::Bool | Type::I64 | Type::F64)
            }
            CoreTrait::Clone | CoreTrait::Drop => false,
        };
        memo.insert(ty.clone(), result);
        Ok(result)
    }
}
