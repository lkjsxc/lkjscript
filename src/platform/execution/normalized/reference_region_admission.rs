//! Independent all-path region type, custody and escape admission.
use super::*;
use kernel::{RegionExpression as E, RegionMode};

impl Resolver<'_, '_> {
    pub(super) fn region_expression(
        &mut self,
        operation: E<ExpressionId, BindingId, LocalValueReference, TypeObjectDigest>,
        context: &Context,
        locals: &mut Locals,
        syntax: &mut Syntax,
        demand: Use,
        depth: usize,
    ) -> Result<Value> {
        // This independent traversal admits the complete enclosing body, including
        // region custody which the older memory-only loan traversal cannot model.
        self.named_view_function(context)?;
        self.budget.reserve(1)?;
        syntax.view_functions.insert(
            context
                .owner
                .ok_or_else(|| invalid("region callable absent"))?,
        );
        let i64_type = self.intern(TypeForm::I64)?;
        match operation {
            operation @ (E::MmapSave { .. } | E::MmapLoad { .. }) => {
                let (requirement, target) = *operation
                    .store()
                    .ok_or_else(|| invalid("missing region storage authority"))?;
                let constraint = self.direct_requirement(requirement, context)?;
                let OwnerRecord::Operation(record) =
                    self.owner(target.package, OwnerKey::Operation(target.operation))?
                else {
                    return Err(invalid("region storage operation absent"));
                };
                let load = matches!(operation, E::MmapLoad { .. });
                if !constraint.operations.contains(&target)
                    || target.package != constraint.interface.package
                    || record.declaration != constraint.interface.declaration
                    || record.parameters.len() != if load { 2 } else { 4 }
                    || record.name.as_str() != if load { "get" } else { "put" }
                {
                    return Err(invalid(
                        "region storage changes its exact get/put authority",
                    ));
                }
                let bytes = self.intern(TypeForm::Bytes)?;
                if !load && self.object(record.result)?.form != TypeForm::Bool {
                    return Err(invalid("region put result is not Bool"));
                }
                let mut parameters = Vec::new();
                self.budget.reserve(record.parameters.len())?;
                for parameter in record.parameters {
                    let OwnerRecord::Parameter(parameter) =
                        self.owner(target.package, OwnerKey::Parameter(parameter))?
                    else {
                        return Err(invalid("region storage parameter absent"));
                    };
                    if parameter.parent != ParameterParent::Operation(target.operation)
                        || parameter.use_mode != ParameterUse::Unrestricted
                    {
                        return Err(invalid(
                            "region storage parameter changes its exact owner or use",
                        ));
                    }
                    parameters.push(parameter.ty);
                }
                if self.object(parameters[0])?.form != TypeForm::StaticText
                    || (!load && parameters[2] != bytes)
                {
                    return Err(invalid("region storage parameter shape differs"));
                }
                for (position, expression) in
                    operation.expressions().into_iter().flatten().enumerate()
                {
                    let expected = if position < 2 {
                        parameters[position]
                    } else if load {
                        bytes
                    } else {
                        parameters[3]
                    };
                    if self
                        .scan(*expression, context, locals, syntax, Use::Value, depth)?
                        .ty
                        != Some(expected)
                    {
                        return Err(invalid("region storage argument type differs"));
                    }
                }
                let capability = *operation
                    .capability()
                    .ok_or_else(|| invalid("missing persistence capability"))?;
                let local = self.local(capability, context, locals)?;
                let TypeForm::RegionCap { region, mode } = self.object(local.ty)?.form else {
                    return Err(invalid("region persistence operand is not custody"));
                };
                let ty = if let E::MmapLoad { frozen_type, .. } = operation {
                    if mode != RegionMode::Unique
                        || local.read
                        || !matches!(self.object(frozen_type)?.form,
                        TypeForm::RegionCap { region: actual, mode: RegionMode::Frozen } if actual == region)
                    {
                        return Err(invalid("region load changes its brand or unique custody"));
                    }
                    self.local_value(capability, context, locals, Use::Consume)?;
                    frozen_type
                } else {
                    if mode != RegionMode::Frozen {
                        return Err(invalid("region save requires frozen custody"));
                    }
                    bytes
                };
                Ok(Value {
                    ty: Some(ty),
                    ..Value::default()
                })
            }
            E::With {
                binding,
                capacity,
                body,
            } => {
                let capacity = self.scan(capacity, context, locals, syntax, Use::Value, depth)?;
                if capacity.ty != Some(i64_type) {
                    return Err(invalid("region capacity is not I64"));
                }
                let owner = context
                    .owner
                    .ok_or_else(|| invalid("region has no defining callable"))?;
                let OwnerRecord::Binding(record) =
                    self.owner(owner.package, OwnerKey::Binding(binding))?
                else {
                    return Err(invalid("region binder is absent"));
                };
                let ty = record
                    .declared_type
                    .ok_or_else(|| invalid("region capability annotation is absent"))?;
                let TypeForm::RegionCap {
                    region,
                    mode: RegionMode::Unique,
                } = self.object(ty)?.form
                else {
                    return Err(invalid("region-with needs unique custody"));
                };
                if record.value.is_some()
                    || !matches!(self.object(region)?.form, TypeForm::RegionBrand { callable, binding: actual } if callable == owner && actual == binding)
                {
                    return Err(invalid("region binder and exact brand disagree"));
                }
                self.budget.reserve(context.lexical.len() + 1)?;
                let mut scoped = context.clone();
                if !scoped.lexical.insert(binding) {
                    return Err(invalid("region binder is already live"));
                }
                self.binding(
                    binding,
                    &Value {
                        ty: Some(ty),
                        ..Value::default()
                    },
                    &scoped,
                    locals,
                    syntax,
                    Some(BindingKind::Region),
                )?;
                let result = self.scan(body, &scoped, locals, syntax, demand, depth)?;
                self.region_absent(
                    result
                        .ty
                        .ok_or_else(|| invalid("region result type is unknown"))?,
                    region,
                )?;
                locals.remove(&LocalValueReference::LexicalBinding(binding));
                Ok(result)
            }
            E::Offset { pointer } => {
                let pointer = self.scan(pointer, context, locals, syntax, Use::Value, depth)?;
                if !matches!(
                    pointer
                        .ty
                        .map(|ty| self.object(ty))
                        .transpose()?
                        .map(|ty| ty.form),
                    Some(TypeForm::RelPtr { .. })
                ) {
                    return Err(invalid("region-offset requires a relative pointer"));
                }
                self.scalar(TypeForm::I64)
            }
            E::Freeze {
                frozen_type,
                capability,
            } => {
                let TypeForm::RegionCap {
                    region,
                    mode: RegionMode::Frozen,
                } = self.object(frozen_type)?.form
                else {
                    return Err(invalid("region-freeze result is not frozen"));
                };
                let local = self.local(capability, context, locals)?;
                if !matches!(self.object(local.ty)?.form, TypeForm::RegionCap { region: actual, mode: RegionMode::Unique } if actual == region)
                {
                    return Err(invalid("region-freeze changes the region brand"));
                }
                self.local_value(capability, context, locals, Use::Consume)?;
                Ok(Value {
                    ty: Some(frozen_type),
                    ..Value::default()
                })
            }
            operation => {
                let pointer_type = *operation
                    .ty()
                    .ok_or_else(|| invalid("region pointer type absent"))?;
                let TypeForm::RelPtr { region, item } = self.object(pointer_type)?.form else {
                    return Err(invalid("region operation has no relative-pointer type"));
                };
                self.region_cell(item, region)?;
                let mut inputs = operation.expressions().into_iter().flatten();
                let first = inputs
                    .next()
                    .map(|id| self.scan(*id, context, locals, syntax, Use::Value, depth))
                    .transpose()?;
                let second = inputs
                    .next()
                    .map(|id| self.scan(*id, context, locals, syntax, Use::Value, depth))
                    .transpose()?;
                let capability = *operation
                    .capability()
                    .ok_or_else(|| invalid("region capability absent"))?;
                let local = self.local(capability, context, locals)?;
                let TypeForm::RegionCap {
                    region: actual,
                    mode,
                } = self.object(local.ty)?.form
                else {
                    return Err(invalid("region operand is not custody"));
                };
                if actual != region {
                    return Err(invalid("region pointer and capability brands differ"));
                }
                if matches!(operation, E::Alloc { .. } | E::Write { .. }) {
                    if mode != RegionMode::Unique || local.read {
                        return Err(invalid("region mutation requires unique owning custody"));
                    }
                    self.consume(capability, &local, context, locals)?;
                    locals
                        .get_mut(&capability)
                        .ok_or_else(|| invalid("region owner disappeared"))?
                        .live = true;
                }
                let first = first.and_then(|value| value.ty);
                let result = match operation {
                    E::Alloc { .. } if first == Some(item) => pointer_type,
                    E::Read { .. } if first == Some(pointer_type) => item,
                    E::Write { .. }
                        if first == Some(pointer_type)
                            && second.and_then(|value| value.ty) == Some(item) =>
                    {
                        self.intern(TypeForm::Unit)?
                    }
                    E::Resolve { .. } if first == Some(i64_type) => pointer_type,
                    _ => {
                        return Err(invalid(
                            "region operands disagree with their exact cell types",
                        ));
                    }
                };
                Ok(Value {
                    ty: Some(result),
                    ..Value::default()
                })
            }
        }
    }

    fn region_absent(&mut self, ty: TypeObjectDigest, region: TypeObjectDigest) -> Result<()> {
        let mut pending = Vec::new();
        region_pending(&mut pending, 1)?;
        pending.push(ty);
        let mut seen = BTreeSet::new();
        while let Some(ty) = pending.pop() {
            self.budget.step_iterative()?;
            if ty == region {
                return Err(invalid("result escapes its lexical region"));
            }
            self.budget.reserve(1)?;
            if !seen.insert(ty) {
                continue;
            }
            let children = self.object(ty)?.child_types();
            region_pending(&mut pending, children.len())?;
            pending.extend(children.into_iter().rev());
        }
        Ok(())
    }

    fn region_cell(&mut self, ty: TypeObjectDigest, region: TypeObjectDigest) -> Result<()> {
        enum Task {
            Enter(TypeObjectDigest),
            Exit(TypeObjectDigest),
        }
        let mut pending = Vec::new();
        region_pending(&mut pending, 1)?;
        pending.push(Task::Enter(ty));
        let mut active = BTreeSet::new();
        let mut complete = BTreeSet::new();
        while let Some(task) = pending.pop() {
            let ty = match task {
                Task::Exit(ty) => {
                    active.remove(&ty);
                    complete.insert(ty);
                    continue;
                }
                Task::Enter(ty) => ty,
            };
            self.budget.step_iterative()?;
            self.budget.reserve(1)?;
            if complete.contains(&ty) {
                continue;
            }
            if !active.insert(ty) {
                return Err(invalid("region cell has a cyclic inline layout"));
            }
            region_pending(&mut pending, 1)?;
            pending.push(Task::Exit(ty));
            match self.object(ty)?.form {
                TypeForm::Unit | TypeForm::Bool | TypeForm::I64 => {}
                TypeForm::RelPtr { region: actual, .. } if actual == region => {}
                TypeForm::StructuralRecord { fields } => {
                    region_pending(&mut pending, fields.len())?;
                    pending.extend(fields.into_iter().rev().map(|field| Task::Enter(field.ty)));
                }
                _ => {
                    return Err(invalid(
                        "region cell contains non-flat or foreign-region data",
                    ));
                }
            }
        }
        Ok(())
    }
}

fn region_pending<T>(pending: &mut Vec<T>, additional: usize) -> Result<()> {
    pending.try_reserve(additional).map_err(|_| {
        ExecutionError::resource(
            "reference_region_storage",
            "cannot reserve canonical region traversal storage",
        )
    })
}
