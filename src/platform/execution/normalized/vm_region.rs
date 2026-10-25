//! Bytecode region execution. Checked values retain exact prepared types while
//! the region owner checks live activation custody and every relative offset.
use super::super::region_value::{self, Layout};
use super::*;
use super::super::value::RawValue;
use crate::platform::compiler::unit::RegionInstruction as I;
use crate::platform::kernel::RegionMode;

impl Machine<'_> {
    pub(super) fn region_operation(
        &mut self,
        operation: super::super::prepare::NormalizedRegionInstruction,
    ) -> Result<(), ExecutionError> {
        let arena = self.regions;
        let control = self.control;
        let domain = self.memory_domain;
        match operation {
            operation @ (I::MmapSave { .. } | I::MmapLoad { .. }) => {
                self.region_persistence(operation)
            }
            I::Begin {
                local,
                capability_type,
            } => {
                let capability_type = self.resolve_frame_type(
                    capability_type,
                    "region binder type escaped its application",
                )?;
                if !matches!(
                    self.program.types.get(&capability_type).map(|ty| &ty.form),
                    Some(TypeForm::RegionCap {
                        mode: RegionMode::Unique,
                        ..
                    })
                ) {
                    return Err(region_value::error());
                }
                let capacity = RawValue::new(self.pop()?.into_raw());
                let NormalizedValue::I64(capacity) = capacity.as_raw() else {
                    return Err(region_value::error());
                };
                let capacity = u64::try_from(*capacity).map_err(|_| region_value::error())?;
                if self.region_scopes.len() == self.region_scopes.capacity() {
                    self.charge_allocation(
                        ((self.region_scopes.len() + 1)
                            * std::mem::size_of::<(
                                u64,
                                u32,
                                crate::platform::execution::region::runtime::Identity,
                            )>()) as u64,
                    )?;
                    self.region_scopes
                        .try_reserve_exact(1)
                        .map_err(|_| region_value::error())?;
                }
                let token = arena.open(domain, capacity, control, |bytes| {
                    self.charge_allocation(bytes)
                })?;
                let identity = token.identity();
                let value = CheckedValue::region(
                    self.program,
                    arena,
                    domain,
                    capability_type,
                    NormalizedValue::RegionCap {
                        ty: capability_type,
                        token,
                    },
                    control,
                )?;
                self.set_local(local, Some(value))?;
                self.region_scopes
                    .push((self.current_frame()?.id, local, identity));
                Ok(())
            }
            I::End { local } => {
                let frame = self.current_frame()?.id;
                let (owner, binding, identity) =
                    self.region_scopes.pop().ok_or_else(region_value::error)?;
                if owner != frame || binding != local {
                    return Err(region_value::error());
                }
                self.set_local(local, None)?;
                arena.close(identity)
            }
            I::Offset => {
                let value = RawValue::new(self.pop()?.into_raw());
                let NormalizedValue::RegionPointer { pointer, .. } = value.as_raw() else {
                    return Err(region_value::error());
                };
                let value = CheckedValue::scalar(
                    self.program,
                    NormalizedValue::I64(i64::try_from(pointer.offset()).map_err(|_| region_value::error())?),
                )?;
                self.push(value)
            }
            I::Freeze { local, frozen_type } => {
                let frozen_type = self.resolve_frame_type(
                    frozen_type,
                    "region freeze type escaped its application",
                )?;
                let Some(TypeForm::RegionCap {
                    region,
                    mode: RegionMode::Frozen,
                }) = self.program.types.get(&frozen_type).map(|ty| &ty.form)
                else {
                    return Err(region_value::error());
                };
                let source = self
                    .current_frame()?
                    .locals
                    .get(local as usize)
                    .and_then(Option::as_ref)
                    .ok_or_else(region_value::error)?;
                let NormalizedValue::RegionCap { ty, token } = source.raw() else {
                    return Err(region_value::error());
                };
                if !matches!(self.program.types.get(ty).map(|ty| &ty.form), Some(TypeForm::RegionCap { region: actual, mode: RegionMode::Unique }) if actual == region)
                {
                    return Err(region_value::error());
                }
                arena.validate(token, domain, true)?;
                let source = self
                    .current_frame_mut()?
                    .locals
                    .get_mut(local as usize)
                    .and_then(Option::take)
                    .ok_or_else(region_value::error)?;
                let NormalizedValue::RegionCap { token, .. } = source.into_raw() else {
                    return Err(region_value::error());
                };
                let token = arena.freeze(token, domain, control)?;
                let result = CheckedValue::region(
                    self.program,
                    arena,
                    domain,
                    frozen_type,
                    NormalizedValue::RegionCap {
                        ty: frozen_type,
                        token,
                    },
                    control,
                )?;
                self.push(result)
            }
            operation => {
                let pointer_type = self.resolve_frame_type(
                    *operation.ty().ok_or_else(region_value::error)?,
                    "region pointer type escaped its application",
                )?;
                let Some(TypeForm::RelPtr { region, item }) =
                    self.program.types.get(&pointer_type).map(|ty| &ty.form)
                else {
                    return Err(region_value::error());
                };
                let (region, item) = (*region, *item);
                let layout =
                    Layout::derive(&self.program.types, item, region, control, &mut |bytes| {
                        reserve_execution_allocation(
                            self.policy,
                            self.shared_budget.as_deref(),
                            &mut self.observation,
                            bytes,
                        )
                    })?;
                let value = if matches!(operation, I::Alloc { .. } | I::Write { .. }) {
                    Some(RawValue::new(self.pop()?.into_raw()))
                } else {
                    None
                };
                let input = if matches!(
                    operation,
                    I::Read { .. } | I::Write { .. } | I::Resolve { .. }
                ) {
                    Some(RawValue::new(self.pop()?.into_raw()))
                } else {
                    None
                };
                let local = operation.local().ok_or_else(region_value::error)?;
                let source = self
                    .frames
                    .last()
                    .and_then(|frame| frame.locals.get(local as usize))
                    .and_then(Option::as_ref)
                    .ok_or_else(region_value::error)?;
                let NormalizedValue::RegionCap { ty, token } = source.raw() else {
                    return Err(region_value::error());
                };
                if !matches!(self.program.types.get(ty).map(|ty| &ty.form), Some(TypeForm::RegionCap { region: actual, .. }) if *actual == region)
                {
                    return Err(region_value::error());
                }
                arena.validate(token, domain, false)?;
                let identity = token.identity();
                let mut reserve = |bytes| {
                    reserve_execution_allocation(
                        self.policy,
                        self.shared_budget.as_deref(),
                        &mut self.observation,
                        bytes,
                    )
                };
                let (raw, result_type) = match operation {
                    I::Alloc { .. } => {
                        let bytes = layout.encode(
                            value.as_ref().ok_or_else(region_value::error)?.as_raw(),
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
                    I::Read { .. } | I::Write { .. } => {
                        let Some(NormalizedValue::RegionPointer { ty, pointer }) = input.as_ref().map(RawValue::as_raw) else {
                            return Err(region_value::error());
                        };
                        if *ty != pointer_type {
                            return Err(region_value::error());
                        }
                        if matches!(operation, I::Write { .. }) {
                            let bytes = layout.encode(
                                value.as_ref().ok_or_else(region_value::error)?.as_raw(),
                                identity,
                                control,
                                &mut reserve,
                            )?;
                            arena.write(token, domain, layout.storage, *pointer, &bytes, control)?;
                            let value = CheckedValue::scalar(self.program, NormalizedValue::Unit)?;
                            return self.push(value);
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
                    I::Resolve { .. } => {
                        let Some(NormalizedValue::I64(offset)) = input.as_ref().map(RawValue::as_raw) else {
                            return Err(region_value::error());
                        };
                        let offset = u64::try_from(*offset).map_err(|_| region_value::error())?;
                        let pointer =
                            arena.resolve(token, domain, layout.storage, offset, control)?;
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
                let value =
                    CheckedValue::region(self.program, arena, domain, result_type, raw, control)?;
                self.push(value)
            }
        }
    }

    fn region_persistence(
        &mut self,
        operation: super::super::prepare::NormalizedRegionInstruction,
    ) -> Result<(), ExecutionError> {
        let (requirement, target) = *operation.store().ok_or_else(region_value::error)?;
        let super::super::prepare::NormalizedTransactionRequirement::Concrete(requirement) =
            requirement
        else {
            return Err(type_error("region storage requirement is not closed"));
        };
        let arguments = self.pop_many(3)?;
        for argument in &arguments {
            if argument.class(self.program, &mut self.observation.value_work)? != Class::Free {
                return Err(region_value::error());
            }
        }
        let arguments: Vec<_> = arguments.into_iter().map(CheckedValue::into_raw).collect();
        self.admit_operation_allowance(requirement)?;
        let capabilities = self.capabilities.ok_or_else(capabilities_unbound)?;
        let canonical = capabilities.canonical_requirement(requirement)?;
        let call_policy = capabilities.call_policy(self.program, requirement, target)?;
        let local = operation.local().ok_or_else(region_value::error)?;
        let load = matches!(operation, I::MmapLoad { .. });
        let frozen_type = if let I::MmapLoad { frozen_type, .. } = operation {
            Some(self.resolve_frame_type(frozen_type, "region load type escaped its application")?)
        } else {
            None
        };
        {
            let value = self
                .current_frame()?
                .locals
                .get(local as usize)
                .and_then(Option::as_ref)
                .ok_or_else(region_value::error)?;
            let NormalizedValue::RegionCap { ty, token } = value.raw() else {
                return Err(region_value::error());
            };
            let Some(TypeForm::RegionCap { region, mode }) =
                self.program.types.get(ty).map(|t| &t.form)
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
                && !matches!(self.program.types.get(&ty).map(|t| &t.form),
                Some(TypeForm::RegionCap { region: actual, mode: RegionMode::Frozen }) if actual == region)
            {
                return Err(region_value::error());
            }
            self.regions.validate(token, self.memory_domain, load)?;
        }
        if !self.transactions.contains_key(&canonical) {
            return Err(runtime_error(
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
            reserve_execution_allocation(
                self.policy,
                self.shared_budget.as_deref(),
                &mut self.observation,
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
            let source = self
                .current_frame_mut()?
                .locals
                .get_mut(local as usize)
                .and_then(Option::take)
                .ok_or_else(region_value::error)?;
            let NormalizedValue::RegionCap { token, .. } = source.into_raw() else {
                return Err(region_value::error());
            };
            let token =
                self.regions
                    .activate_image(token, self.memory_domain, image, self.control)?;
            let value = CheckedValue::region(
                self.program,
                self.regions,
                self.memory_domain,
                ty,
                NormalizedValue::RegionCap { ty, token },
                self.control,
            )?;
            self.push(value)
        } else {
            let source = self
                .frames
                .last()
                .and_then(|f| f.locals.get(local as usize))
                .and_then(Option::as_ref)
                .ok_or_else(region_value::error)?;
            let NormalizedValue::RegionCap { token, .. } = source.raw() else {
                return Err(region_value::error());
            };
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
            self.push(CheckedValue::scalar(
                self.program,
                NormalizedValue::bytes(digest.bytes()),
            )?)
        }
    }
}
