//! ReadShareable follows declared accessible paths and every actual. Passive
//! custody does not become a readable path or a blanket source Share obligation.
use super::*;

impl Resolver<'_> {
    pub(super) fn read_shareable_view(
        &mut self,
        ty: TypeObjectDigest,
        occurrence: Occurrence,
    ) -> Result<bool> {
        self.read_shareable_inner(ty, occurrence, &mut BTreeSet::new(), 0)
    }
    fn read_shareable_inner(
        &mut self,
        ty: TypeObjectDigest,
        occurrence: Occurrence,
        active: &mut BTreeSet<TypeObjectDigest>,
        depth: usize,
    ) -> Result<bool> {
        self.step(depth)?;
        if active.contains(&ty) {
            return Err(Failure::Category);
        }
        self.reserve_entries(1)?;
        active.insert(ty);
        let form = self.object(ty)?.form.clone();
        self.admit_read_view_form(&form, occurrence)?;
        let TypeForm::ReadViewApplied {
            declaration,
            arguments,
            read_scope,
        } = form
        else {
            // An abstract family bound is only category evidence. Unknown
            // fields and phantom actuals supply no independent sharing proof.
            active.remove(&ty);
            return Ok(false);
        };
        self.admit_view_layout(declaration)?;
        let view = self.read_view_declaration(declaration)?;
        for actual in &arguments {
            self.step(depth)?;
            if self.contains_read_view(*actual)? {
                return Err(Failure::Category);
            }
            if !self.shareable_referent(*actual, occurrence)? {
                active.remove(&ty);
                return Ok(false);
            }
        }
        self.reserve_entries(arguments.len() + 1)?;
        let binder = ReadScopeBinder::Declaration(declaration);
        let environment = ReadViewEnvironment {
            types: Environment {
                ordinary: view
                    .type_parameters
                    .iter()
                    .copied()
                    .zip(arguments)
                    .collect(),
                source: Context::Forbidden,
                caller: occurrence,
                contract_operand: None,
                witnesses: BTreeMap::new(),
            },
            source_binder: binder,
            caller_binder: self.view_scope_binder(occurrence, &read_scope)?,
            scopes: BTreeMap::from([(
                ReadScopeReference {
                    binder,
                    parameter: view.scope_parameter.id,
                },
                read_scope,
            )]),
        };
        for field in view.fields {
            let field_type = self.substitute_view(field.ty, &environment, depth + 1)?;
            match field.kind {
                ReadViewFieldKind::Ordinary => {
                    if self.read_view_category(field_type, occurrence)? != ReadCategory::Ordinary {
                        return Err(Failure::Category);
                    }
                }
                ReadViewFieldKind::ReadReference { .. } => {
                    if self.object(field_type)?.form.is_read_view() {
                        if !self.read_shareable_inner(field_type, occurrence, active, depth + 1)? {
                            active.remove(&ty);
                            return Ok(false);
                        }
                    } else if !self.shareable_referent(field_type, occurrence)? {
                        active.remove(&ty);
                        return Ok(false);
                    }
                }
            }
        }
        active.remove(&ty);
        Ok(true)
    }
    fn shareable_referent(&mut self, ty: TypeObjectDigest, occurrence: Occurrence) -> Result<bool> {
        self.step(0)?;
        let overlay = self.overlay();
        let oracle = Oracle(&overlay, occurrence.parameter_scope);
        match self.read_view_category(ty, occurrence)? {
            ReadCategory::Ordinary => Ok(oracle.ordinary(ty)),
            ReadCategory::Owned => {
                Ok(oracle.read_shape(ty, &BTreeMap::new(), 0, &mut self.remaining) == Some(true))
            }
            _ => Ok(false),
        }
    }
}
