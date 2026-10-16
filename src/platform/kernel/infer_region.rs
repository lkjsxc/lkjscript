//! Exact lexical brands and fixed-layout cells for region expressions.
use super::*;
use crate::platform::kernel::{RegionExpression as E, RegionMode};

impl<R: ExpressionRead> ExpressionValidator<'_, '_, R> {
    pub(super) fn infer_region(
        &mut self,
        operation: E<ExpressionId, BindingId, LocalValueReference, TypeObjectDigest>,
        context: &ExecutionContext,
        generation: u16,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        let i64_type = self.canonical_type(TypeForm::I64)?;
        match operation {
            operation @ (E::MmapSave { .. } | E::MmapLoad { .. }) => {
                let (requirement, target) = *operation
                    .store()
                    .ok_or_else(|| region_error("missing storage authority"))?;
                if context.pure || !context.requirements.contains(&requirement) {
                    return Err(region_error(
                        "region persistence requires its exact task allowance",
                    ));
                }
                let requirement = self.requirement_constraint(requirement)?;
                let record = self.operation_record(target.package, target.operation)?;
                if requirement.interface.package != target.package
                    || requirement.interface.declaration != record.declaration
                    || !requirement.operations.contains(&target)
                {
                    return Err(region_error(
                        "region storage operation is outside its exact requirement",
                    ));
                }
                let parameters = self.parameter_types(target.package, &record.parameters)?;
                for parameter in &record.parameters {
                    let unrestricted = if target.package == self.read.package_id() {
                        matches!(self.read.owner(OwnerKey::Parameter(*parameter))?, Some(OwnerRecord::Parameter(p))
                            if p.use_mode == ParameterUse::Unrestricted && p.resource_requirement.is_none())
                    } else {
                        matches!(self.dependency_owner(target.package, OwnerKey::Parameter(*parameter), "region storage parameter")?,
                            PackageInterfaceRecord::Parameter(p) if p.use_mode == ParameterUse::Unrestricted && p.resource_requirement.is_none())
                    };
                    if !unrestricted {
                        return Err(region_error(
                            "region storage parameters must be unrestricted data",
                        ));
                    }
                }
                let load = matches!(operation, E::MmapLoad { .. });
                let bytes = self.canonical_type(TypeForm::Bytes)?;
                let text = self.canonical_type(TypeForm::StaticText)?;
                let boolean = self.canonical_type(TypeForm::Bool)?;
                if (load && (record.name.as_str() != "get" || parameters.len() != 2))
                    || (!load
                        && (record.name.as_str() != "put"
                            || parameters.len() != 4
                            || parameters[2] != bytes
                            || record.result != boolean))
                    || parameters.first() != Some(&text)
                {
                    return Err(region_error(
                        "region persistence needs the exact data get/put signature",
                    ));
                }
                for (position, child) in operation.expressions().into_iter().flatten().enumerate() {
                    let expected = if position < 2 {
                        parameters[position]
                    } else if load {
                        bytes
                    } else {
                        parameters[3]
                    };
                    let actual = self.infer(*child, context, depth)?;
                    require_same(
                        expected,
                        actual,
                        "kernel_region_storage_argument",
                        "region storage argument",
                    )?;
                }
                let capability = *operation
                    .capability()
                    .ok_or_else(|| region_error("missing persistence custody"))?;
                let ty = self.raw_local_type(capability, context, depth)?;
                let TypeForm::RegionCap { region, mode } = self.type_object(ty)?.form else {
                    return Err(region_error("persistence requires region custody"));
                };
                if load {
                    let E::MmapLoad { frozen_type, .. } = operation else {
                        return Err(region_error("invalid load"));
                    };
                    if mode != RegionMode::Unique
                        || !matches!(self.type_object(frozen_type)?.form,
                        TypeForm::RegionCap { region: actual, mode: RegionMode::Frozen } if actual == region)
                    {
                        return Err(region_error(
                            "image load must freeze fresh custody of the same brand",
                        ));
                    }
                    Ok(frozen_type)
                } else if mode == RegionMode::Frozen {
                    Ok(bytes)
                } else {
                    Err(region_error("image save requires frozen custody"))
                }
            }
            E::With {
                binding,
                capacity,
                body,
            } => {
                let actual = self.infer(capacity, context, depth)?;
                require_same(
                    i64_type,
                    actual,
                    "kernel_region_capacity",
                    "region capacity",
                )?;
                let Some(OwnerRecord::Binding(record)) =
                    self.read.owner(OwnerKey::Binding(binding))?
                else {
                    return Err(region_error("region binder is absent"));
                };
                let ty = record
                    .declared_type
                    .ok_or_else(|| region_error("region binder needs an exact capability type"))?;
                let TypeForm::RegionCap {
                    region,
                    mode: RegionMode::Unique,
                } = self.type_object(ty)?.form
                else {
                    return Err(region_error("region-with introduces unique custody"));
                };
                let TypeForm::RegionBrand {
                    callable,
                    binding: brand_binding,
                } = self.type_object(region)?.form
                else {
                    return Err(region_error("region-with requires a fresh lexical brand"));
                };
                if record.kind != BindingKind::Region
                    || record.value.is_some()
                    || brand_binding != binding
                    || context.callable != Some(callable)
                {
                    return Err(region_error(
                        "region binder does not match its lexical callable and brand",
                    ));
                }
                let mut scoped = self.scoped_context(context)?;
                if scoped
                    .bindings
                    .insert(binding, (BindingKind::Region, ty))
                    .is_some()
                {
                    return Err(region_error("region binding is already live"));
                }
                self.validate_nominal_type(
                    ty,
                    &scoped,
                    self.type_occurrence(&scoped, generation),
                    0,
                )?;
                let result = self.infer(body, &scoped, depth)?;
                self.require_region_absent(result, region, &mut BTreeSet::new(), 0)?;
                Ok(result)
            }
            E::Freeze {
                frozen_type,
                capability,
            } => {
                let TypeForm::RegionCap {
                    region,
                    mode: RegionMode::Frozen,
                } = self.type_object(frozen_type)?.form
                else {
                    return Err(region_error("freeze result must be frozen custody"));
                };
                self.require_region_capability(capability, region, true, context, depth)?;
                Ok(frozen_type)
            }
            E::Offset { pointer } => {
                let ty = self.infer(pointer, context, depth)?;
                if !matches!(self.type_object(ty)?.form, TypeForm::RelPtr { .. }) {
                    return Err(region_error("region-offset requires a relative pointer"));
                }
                Ok(i64_type)
            }
            operation => {
                let pointer_type = *operation
                    .ty()
                    .ok_or_else(|| region_error("missing region pointer type"))?;
                let TypeForm::RelPtr { region, item } = self.type_object(pointer_type)?.form else {
                    return Err(region_error(
                        "region cell operation requires an exact relative-pointer type",
                    ));
                };
                self.require_region_cell(item, region, &mut BTreeSet::new(), 0)?;
                let capability = *operation
                    .capability()
                    .ok_or_else(|| region_error("missing region capability"))?;
                let mutable = matches!(operation, E::Alloc { .. } | E::Write { .. });
                self.require_region_capability(capability, region, mutable, context, depth)?;
                match operation {
                    E::Alloc { value, .. } => {
                        let actual = self.infer(value, context, depth)?;
                        require_same(
                            item,
                            actual,
                            "kernel_region_value",
                            "allocated region value",
                        )?;
                        Ok(pointer_type)
                    }
                    E::Read { pointer, .. } => {
                        let actual = self.infer(pointer, context, depth)?;
                        require_same(
                            pointer_type,
                            actual,
                            "kernel_region_pointer",
                            "region read pointer",
                        )?;
                        Ok(item)
                    }
                    E::Write { pointer, value, .. } => {
                        let actual = self.infer(pointer, context, depth)?;
                        require_same(
                            pointer_type,
                            actual,
                            "kernel_region_pointer",
                            "region write pointer",
                        )?;
                        let actual = self.infer(value, context, depth)?;
                        require_same(
                            item,
                            actual,
                            "kernel_region_value",
                            "replacement region value",
                        )?;
                        self.canonical_type(TypeForm::Unit)
                    }
                    E::Resolve { offset, .. } => {
                        let actual = self.infer(offset, context, depth)?;
                        require_same(
                            i64_type,
                            actual,
                            "kernel_region_offset",
                            "persisted region offset",
                        )?;
                        Ok(pointer_type)
                    }
                    _ => Err(region_error("invalid region operation")),
                }
            }
        }
    }

    fn require_region_capability(
        &mut self,
        capability: LocalValueReference,
        expected: TypeObjectDigest,
        mutable: bool,
        context: &ExecutionContext,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        let ty = self.raw_local_type(capability, context, depth)?;
        match self.type_object(ty)?.form {
            TypeForm::RegionCap { region, mode }
                if region == expected && (!mutable || mode == RegionMode::Unique) =>
            {
                Ok(())
            }
            _ => Err(region_error(
                "region operation requires live custody of the same brand and mode",
            )),
        }
    }

    fn require_region_absent(
        &mut self,
        ty: TypeObjectDigest,
        region: TypeObjectDigest,
        seen: &mut BTreeSet<TypeObjectDigest>,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        self.consume_work()?;
        if depth > MAXIMUM_TYPE_DEPTH || ty == region {
            return Err(region_error(
                "result retains its lexical region or exceeds type admission",
            ));
        }
        if !seen.insert(ty) {
            return Ok(());
        }
        let object = self.type_object(ty)?;
        for child in object.child_types() {
            self.require_region_absent(child, region, seen, depth + 1)?;
        }
        if let Some((declaration, arguments)) = nominal_parts(&object.form) {
            for child in self.nominal_children(declaration, arguments)? {
                self.require_region_absent(child, region, seen, depth + 1)?;
            }
        }
        Ok(())
    }

    fn require_region_cell(
        &mut self,
        ty: TypeObjectDigest,
        region: TypeObjectDigest,
        active: &mut BTreeSet<TypeObjectDigest>,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        self.consume_work()?;
        if depth > MAXIMUM_TYPE_DEPTH || !active.insert(ty) {
            return Err(region_error("region cell has an unbounded inline layout"));
        }
        match self.type_object(ty)?.form {
            TypeForm::Unit | TypeForm::Bool | TypeForm::I64 => {}
            TypeForm::RelPtr { region: actual, .. } if actual == region => {}
            TypeForm::StructuralRecord { fields } => {
                for field in fields {
                    self.require_region_cell(field.ty, region, active, depth + 1)?;
                }
            }
            _ => {
                return Err(region_error(
                    "region cells require fixed ordinary data or same-region relative handles",
                ));
            }
        }
        active.remove(&ty);
        Ok(())
    }
}

fn region_error(message: &str) -> Diagnostic {
    type_error("kernel_region_expression", message)
}
