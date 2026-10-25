//! Canonical expression execution for lexical regions. No bytecode instruction
//! or prepared classification supplies the reference evaluator's type decisions.
use super::super::region_value::{self, Layout};
use super::*;
use super::super::value::RawValue;
use crate::platform::kernel::{BindingKind, RegionExpression as E, RegionMode};

impl ReferenceState<'_> {
    pub(super) fn evaluate_region(
        &mut self,
        operation: E<
            ExpressionId,
            crate::platform::semantic_id::BindingId,
            LocalValueReference,
            TypeObjectDigest,
        >,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        let arena = self.regions;
        let control = self.control;
        let domain = self.memory_domain;
        match operation {
            operation @ (E::MmapSave { .. } | E::MmapLoad { .. }) => {
                self.region_persistence(operation, locals)
            }
            E::With {
                binding,
                capacity,
                body,
            } => {
                let capacity = RawValue::new(self.evaluate(capacity, locals)?.release());
                let NormalizedValue::I64(capacity) = capacity.as_raw() else {
                    return Err(region_value::error());
                };
                let capacity = u64::try_from(*capacity).map_err(|_| region_value::error())?;
                let Some(OwnerRecord::Binding(record)) = self.owner(OwnerKey::Binding(binding))?
                else {
                    return Err(region_value::error());
                };
                if record.kind != BindingKind::Region || record.value.is_some() {
                    return Err(region_value::error());
                }
                let ty = record.declared_type.ok_or_else(region_value::error)?;
                let Some(TypeForm::RegionCap {
                    region,
                    mode: RegionMode::Unique,
                }) = self.schema.types.get(&ty).map(|ty| &ty.form)
                else {
                    return Err(region_value::error());
                };
                if !matches!(self.schema.types.get(region).map(|ty| &ty.form), Some(TypeForm::RegionBrand { binding: actual, callable }) if *actual == binding && callable.package == self.active_package)
                {
                    return Err(region_value::error());
                }
                let local = LocalValueReference::LexicalBinding(binding);
                if locals.contains_key(&local) {
                    return Err(region_value::error());
                }
                self.charge_allocation(
                    std::mem::size_of::<(LocalValueReference, CheckedValue)>() as u64
                )?;
                let token = arena.open(domain, capacity, control, |bytes| {
                    self.charge_allocation(bytes)
                })?;
                let identity = token.identity();
                let value = CheckedValue::region(
                    &self.schema,
                    arena,
                    domain,
                    ty,
                    NormalizedValue::RegionCap { ty, token },
                    control,
                )?;
                locals.insert(local, value);
                let result = self.evaluate(body, locals);
                locals.remove(&local);
                let cleanup = arena.close(identity);
                match result {
                    Err(error) => Err(error),
                    Ok(value) => {
                        cleanup?;
                        Ok(value)
                    }
                }
            }
            E::Offset { pointer } => {
                let value = RawValue::new(self.evaluate(pointer, locals)?.release());
                let NormalizedValue::RegionPointer { pointer, .. } = value.as_raw() else {
                    return Err(region_value::error());
                };
                CheckedValue::primitive(
                    &self.schema,
                    NormalizedValue::I64(i64::try_from(pointer.offset()).map_err(|_| region_value::error())?),
                )
            }
            E::Freeze {
                frozen_type,
                capability,
            } => {
                let frozen_type = self.resolve_type_arguments(&[frozen_type])?[0];
                let Some(TypeForm::RegionCap {
                    region,
                    mode: RegionMode::Frozen,
                }) = self.schema.types.get(&frozen_type).map(|ty| &ty.form)
                else {
                    return Err(region_value::error());
                };
                let value = locals.get(&capability).ok_or_else(region_value::error)?;
                let NormalizedValue::RegionCap { ty, token } = value.raw() else {
                    return Err(region_value::error());
                };
                if !matches!(self.schema.types.get(ty).map(|ty| &ty.form), Some(TypeForm::RegionCap { region: actual, mode: RegionMode::Unique }) if actual == region)
                {
                    return Err(region_value::error());
                }
                arena.validate(token, domain, true)?;
                let value = locals.remove(&capability).ok_or_else(region_value::error)?;
                let NormalizedValue::RegionCap { token, .. } = value.release() else {
                    return Err(region_value::error());
                };
                let token = arena.freeze(token, domain, control)?;
                CheckedValue::region(
                    &self.schema,
                    arena,
                    domain,
                    frozen_type,
                    NormalizedValue::RegionCap {
                        ty: frozen_type,
                        token,
                    },
                    control,
                )
            }
            operation => {
                let pointer_type = self.resolve_type_arguments(&[*operation
                    .ty()
                    .ok_or_else(region_value::error)?])?[0];
                let Some(TypeForm::RelPtr { region, item }) =
                    self.schema.types.get(&pointer_type).map(|ty| &ty.form)
                else {
                    return Err(region_value::error());
                };
                let (region, item) = (*region, *item);
                let schema = Arc::clone(&self.schema);
                let layout = Layout::derive(&schema.types, item, region, control, &mut |bytes| {
                    self.charge_allocation(bytes)
                })?;
                let mut inputs = operation.expressions().into_iter().flatten();
                let first = inputs
                    .next()
                    .map(|id| self.evaluate(*id, locals).map(|value| RawValue::new(value.release())))
                    .transpose()?;
                let second = inputs
                    .next()
                    .map(|id| self.evaluate(*id, locals).map(|value| RawValue::new(value.release())))
                    .transpose()?;
                let capability = operation.capability().ok_or_else(region_value::error)?;
                let source = locals.get(capability).ok_or_else(region_value::error)?;
                let NormalizedValue::RegionCap { ty, token } = source.raw() else {
                    return Err(region_value::error());
                };
                if !matches!(schema.types.get(ty).map(|ty| &ty.form), Some(TypeForm::RegionCap { region: actual, .. }) if *actual == region)
                {
                    return Err(region_value::error());
                }
                arena.validate(token, domain, false)?;
                let identity = token.identity();
                let mut reserve = |bytes| self.charge_allocation(bytes);
                let (raw, result_type) = match operation {
                    E::Alloc { .. } => {
                        let bytes = layout.encode(
                            first.as_ref().ok_or_else(region_value::error)?.as_raw(),
                            identity,
                            control,
                            &mut reserve,
                        )?;
                        let pointer =
                            arena.allocate(token, domain, layout.storage, &bytes, control)?;
                        (
                            NormalizedValue::RegionPointer {
                                ty: pointer_type,
                                pointer,
                            },
                            pointer_type,
                        )
                    }
                    E::Read { .. } | E::Write { .. } => {
                        let NormalizedValue::RegionPointer { ty, pointer } =
                            first.as_ref().ok_or_else(region_value::error)?.as_raw()
                        else {
                            return Err(region_value::error());
                        };
                        if *ty != pointer_type {
                            return Err(region_value::error());
                        }
                        if matches!(operation, E::Write { .. }) {
                            let bytes = layout.encode(
                                second.as_ref().ok_or_else(region_value::error)?.as_raw(),
                                identity,
                                control,
                                &mut reserve,
                            )?;
                            arena.write(
                                token,
                                domain,
                                layout.storage,
                                *pointer,
                                &bytes,
                                control,
                            )?;
                            return CheckedValue::primitive(&schema, NormalizedValue::Unit);
                        }
                        (
                            arena.read(
                                token,
                                domain,
                                layout.storage,
                                *pointer,
                                control,
                                |bytes| layout.decode(bytes, identity, control, &mut reserve),
                            )?,
                            item,
                        )
                    }
                    E::Resolve { .. } => {
                        let NormalizedValue::I64(offset) =
                            first.as_ref().ok_or_else(region_value::error)?.as_raw()
                        else {
                            return Err(region_value::error());
                        };
                        let pointer = arena.resolve(
                            token,
                            domain,
                            layout.storage,
                            u64::try_from(*offset).map_err(|_| region_value::error())?,
                            control,
                        )?;
                        (
                            NormalizedValue::RegionPointer {
                                ty: pointer_type,
                                pointer,
                            },
                            pointer_type,
                        )
                    }
                    _ => return Err(region_value::error()),
                };
                CheckedValue::region(&schema, arena, domain, result_type, raw, control)
            }
        }
    }

    fn region_persistence(
        &mut self,
        operation: E<ExpressionId, BindingId, LocalValueReference, TypeObjectDigest>,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        let (requirement, target) = *operation.store().ok_or_else(region_value::error)?;
        let requirement = self.resolve_requirement(requirement)?;
        self.charge_allocation((3 * std::mem::size_of::<NormalizedValue>()) as u64)?;
        let mut arguments = Vec::new();
        arguments
            .try_reserve_exact(3)
            .map_err(|_| region_value::error())?;
        for child in operation.expressions().into_iter().flatten() {
            let value = self.evaluate(*child, locals)?;
            if value.ownership(&self.schema, &mut self.observation.value_work)?
                != Ownership::Ordinary
            {
                return Err(region_value::error());
            }
            arguments.push(value.release());
        }
        self.admit_operation_allowance(requirement)?;
        let capabilities = self
            .capabilities
            .ok_or_else(reference_capabilities_unbound)?;
        let canonical = capabilities.canonical_requirement_exact(self.program, requirement)?;
        let call_policy = capabilities.call_policy_exact(self.program, requirement, target)?;
        let capability = *operation.capability().ok_or_else(region_value::error)?;
        let load = matches!(operation, E::MmapLoad { .. });
        let frozen_type = if let E::MmapLoad { frozen_type, .. } = operation {
            Some(self.resolve_type_arguments(&[frozen_type])?[0])
        } else {
            None
        };
        let source = locals.get(&capability).ok_or_else(region_value::error)?;
        let NormalizedValue::RegionCap { ty, token } = source.raw() else {
            return Err(region_value::error());
        };
        let Some(TypeForm::RegionCap { region, mode }) = self.schema.types.get(ty).map(|t| &t.form)
        else {
            return Err(region_value::error());
        };
        if *mode
            != if load {
                RegionMode::Unique
            } else {
                RegionMode::Frozen
            }
        {
            return Err(region_value::error());
        }
        if let Some(ty) = frozen_type
            && !matches!(self.schema.types.get(&ty).map(|t| &t.form),
            Some(TypeForm::RegionCap { region: actual, mode: RegionMode::Frozen }) if actual == region)
        {
            return Err(region_value::error());
        }
        self.regions.validate(token, self.memory_domain, load)?;
        if !self.transactions.contains_key(&canonical) {
            return Err(reference_error(
                "normalized_data_transaction_required",
                "region persistence requires an active exact data transaction",
            ));
        }
        self.charge_capability_call(requirement)?;
        if !load {
            self.charge_allocation(32 + 2 * std::mem::size_of::<usize>() as u64)?;
        }
        let transaction = self
            .transactions
            .get_mut(&canonical)
            .ok_or_else(region_value::error)?;
        let mut reserve = |bytes| {
            charge_reference_allocation(
                self.policy,
                &mut self.observation,
                self.shared_budget.as_deref(),
                bytes,
            )
        };
        if let Some(ty) = frozen_type {
            let image = transaction.transaction.region_load(
                &call_policy,
                &arguments,
                self.control,
                &mut reserve,
            )?;
            let source = locals.remove(&capability).ok_or_else(region_value::error)?;
            let NormalizedValue::RegionCap { token, .. } = source.release() else {
                return Err(region_value::error());
            };
            let token =
                self.regions
                    .activate_image(token, self.memory_domain, image, self.control)?;
            CheckedValue::region(
                &self.schema,
                self.regions,
                self.memory_domain,
                ty,
                NormalizedValue::RegionCap { ty, token },
                self.control,
            )
        } else {
            let digest =
                self.regions
                    .with_read(token, self.memory_domain, self.control, |read| {
                        transaction.transaction.region_save(
                            &call_policy,
                            &arguments,
                            read,
                            self.control,
                            &mut reserve,
                        )
                    })?;
            CheckedValue::primitive(&self.schema, NormalizedValue::bytes(digest.bytes()))
        }
    }
}
