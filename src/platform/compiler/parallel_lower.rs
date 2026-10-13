//! Closed child signatures are recomputed from canonical records, not table claims.
use super::*;
use crate::platform::kernel::{ExpressionRead, PackageInterfaceRecord};

pub(super) struct Child {
    pub(super) function: u32,
    pub(super) arguments: u32,
    pub(super) result: TypeObjectDigest,
    pub(super) result_owned: bool,
    pub(super) types: Vec<u32>,
    pub(super) implementations: Vec<crate::platform::kernel::ImplementationOperand>,
    pub(super) read_scope_arguments: Vec<ReadScopeArgument>,
}

impl<B: CodeRead + ?Sized> UnitBuilder<'_, B> {
    pub(super) fn parallel_call_contract(
        &mut self,
        expression: ExpressionId,
        wrapper_generation: u16,
        live_loans: &BTreeSet<BindingId>,
    ) -> Result<crate::platform::kernel::parallel::ParallelCall, Diagnostic> {
        let read = CodeExpressionRead {
            canonical: self.canonical,
            package: self.package,
            derived: &self.derived_type_objects,
            work: RefCell::new(CanonicalReadWork::default()),
            owners: Cell::new(0),
            parallel_steps: Some(&self.derived_type_admission.parallel_steps),
        };
        let admission = &self.derived_type_admission.parallel;
        let result: Result<_, Diagnostic> =
            (|| {
                // This reads the actual child header and complete written witness tree.
                // A closed output cannot erase the original consumer's generation.
                let caller = self.scope.map(|declaration| DeclarationReference {
                    package: self.package,
                    declaration,
                });
                let call = crate::platform::kernel::parallel::admit_call_at_with_scope_authority(
                &read, expression, self.scope,
                crate::platform::kernel::owned_associated::read_view_admission::ScopeAuthority {
                    binder: caller.map(ReadScopeBinder::Declaration), lexical: caller, live_loans,
                }, admission,
            )?;
                validate_wrapper(
                    &read,
                    expression,
                    self.scope,
                    wrapper_generation,
                    &call,
                    admission,
                )?;
                Ok(call)
            })();
        self.work.canonical.add(read.work.into_inner());
        self.work.owner_records_read = self
            .work
            .owner_records_read
            .saturating_add(read.owners.get());
        let call = result?;
        self.retain_parallel_result_types(call.result, &call.types)?;
        Ok(call)
    }

    fn retain_parallel_result_types(
        &mut self,
        result: TypeObjectDigest,
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
    ) -> Result<(), Diagnostic> {
        self.derived_type_admission
            .parallel
            .reserve::<TypeObjectDigest>(1)?;
        let mut pending = vec![result];
        let mut seen = BTreeSet::new();
        while let Some(ty) = pending.pop() {
            proof_step(&self.derived_type_admission.parallel_steps)?;
            self.canonical.code_step()?;
            if seen.contains(&ty) {
                continue;
            }
            self.derived_type_admission
                .parallel
                .reserve::<TypeObjectDigest>(1)?;
            seen.insert(ty);
            if let Some(object) = types.get(&ty) {
                self.derived_type_admission
                    .parallel
                    .reserve::<TypeObjectDigest>(object.child_type_count())?;
                pending.extend(object.child_types());
                self.retain_derived_type(Some(ty), object)?;
            } else {
                let object = self.parallel_type(ty)?;
                self.derived_type_admission
                    .parallel
                    .reserve::<TypeObjectDigest>(object.child_type_count())?;
                pending.extend(object.child_types());
            }
        }
        Ok(())
    }

    pub(super) fn parallel_type(&mut self, ty: TypeObjectDigest) -> Result<TypeObject, Diagnostic> {
        let read = self.canonical.code_type(ty)?;
        self.work.canonical.add(read.work);
        if let Some(object) = read.value {
            return Ok(object);
        }
        let key = ObjectKey::from_digest(
            crate::platform::storage::object::ObjectDomain::Type,
            ty.bytes(),
        );
        let bytes = self.derived_type_objects.get(&key).ok_or_else(|| {
            compiler_corrupt(
                "compiler_parallel_result_type",
                "missing closed child result type",
            )
        })?;
        crate::platform::kernel::decode_type_object(bytes, ty)
    }
}

pub(super) fn proof_step(steps: &Cell<usize>) -> Result<(), Diagnostic> {
    let next = steps
        .get()
        .checked_add(1)
        .filter(|next| *next <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
        .ok_or_else(|| {
            compiler_error(
                DiagnosticClass::Resource,
                "compiler_parallel_proof_work",
                "parallel signature proof exceeds its cumulative work allowance",
            )
        })?;
    steps.set(next);
    Ok(())
}

/// Point reads, canonical re-lowering and table copies consume one link-wide
/// allowance. This adapter never enters compiler-unit or artifact caches.
pub(super) struct ReconstructionRead<'a, B: ?Sized> {
    pub(super) source: &'a B,
    pub(super) remaining: &'a Cell<usize>,
}

impl<B: CanonicalBaseRead + ?Sized> ReconstructionRead<'_, B> {
    pub(super) fn reserve(&self, count: usize) -> Result<(), Diagnostic> {
        self.source.validation_checkpoint()?;
        let next = self.remaining.get().checked_sub(count).ok_or_else(|| {
            compiler_error(
                DiagnosticClass::Resource,
                "compiler_parallel_result_type_limit",
                "canonical type reconstruction exceeds the existing validation work bound",
            )
        })?;
        self.remaining.set(next);
        Ok(())
    }

    fn observed<T>(
        &self,
        prepaid: usize,
        operation: impl FnOnce() -> Result<CanonicalRead<T>, Diagnostic>,
    ) -> Result<CanonicalRead<T>, Diagnostic> {
        self.reserve(prepaid)?;
        let read = operation()?;
        let cost = read
            .work
            .point_reads
            .checked_add(read.work.map_entries_visited)
            .and_then(|cost| cost.checked_add(read.work.canonical_records_decoded))
            .and_then(|cost| usize::try_from(cost).ok())
            .ok_or_else(|| {
                compiler_error(
                    DiagnosticClass::Resource,
                    "compiler_parallel_result_type_limit",
                    "canonical read work overflowed",
                )
            })?;
        self.reserve(cost.saturating_sub(prepaid))?;
        Ok(read)
    }

    fn representation_prepaid(&self) -> Result<usize, Diagnostic> {
        usize::try_from(self.source.dependency_count())
            .ok()
            .and_then(|count| count.checked_add(1))
            .ok_or_else(|| {
                compiler_error(
                    DiagnosticClass::Resource,
                    "compiler_parallel_result_type_limit",
                    "canonical dependency work overflowed",
                )
            })
    }
}

impl<B: CanonicalBaseRead + ?Sized> CodeRead for ReconstructionRead<'_, B> {
    fn code_step(&self) -> Result<(), Diagnostic> {
        self.reserve(1)
    }
    fn code_dependency(&self, package: PackageId) -> Result<CanonicalRead<bool>, Diagnostic> {
        self.observed(1, || self.source.code_dependency(package))
    }
    fn code_owner(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
        self.observed(1, || self.source.code_owner(owner))
    }
    fn code_type(
        &self,
        ty: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        self.observed(1, || self.source.code_type(ty))
    }
    fn code_interface(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        self.observed(2, || self.source.code_interface(package, owner))
    }
    fn code_representation_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<bool>, Diagnostic> {
        self.observed(self.representation_prepaid()?, || {
            self.source.code_representation_dependency(package)
        })
    }
    fn code_representation_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        self.observed(self.representation_prepaid()?, || {
            self.source.code_representation_owner(package, owner)
        })
    }
}

fn contains_associated(
    read: &(impl ExpressionRead + ?Sized),
    root: TypeObjectDigest,
    admission: &crate::platform::kernel::owned_associated::OwnedAssociatedAdmission,
) -> Result<bool, Diagnostic> {
    admission.reserve::<(TypeObjectDigest, usize)>(1)?;
    let mut pending = vec![(root, 0_usize)];
    let mut seen = BTreeSet::new();
    while let Some((ty, depth)) = pending.pop() {
        read.validation_work()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(compiler_error(
                DiagnosticClass::Resource,
                "compiler_parallel_type_depth",
                "parallel provenance exceeds the type-depth bound",
            ));
        }
        if seen.contains(&ty) {
            continue;
        }
        admission.reserve::<TypeObjectDigest>(1)?;
        seen.insert(ty);
        let object = read.type_object(ty)?.ok_or_else(|| {
            compiler_corrupt(
                "compiler_parallel_type",
                "parallel provenance omits a canonical type",
            )
        })?;
        if object.form.is_associated() {
            return Ok(true);
        }
        admission.reserve::<(TypeObjectDigest, usize)>(object.child_type_count())?;
        pending.extend(
            object
                .child_types()
                .into_iter()
                .map(|child| (child, depth + 1)),
        );
    }
    Ok(false)
}

fn validate_wrapper(
    read: &(impl ExpressionRead + ?Sized),
    expression: ExpressionId,
    scope: Option<DeclarationId>,
    generation: u16,
    call: &crate::platform::kernel::parallel::ParallelCall,
    admission: &crate::platform::kernel::owned_associated::OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    let Some(OwnerRecord::Expression(record)) = read.owner(OwnerKey::Expression(expression))?
    else {
        return Err(compiler_corrupt(
            "compiler_parallel_call",
            "parallel child lost its canonical source",
        ));
    };
    let occurrence = crate::platform::kernel::owned_associated::OwnedTypeOccurrence::authored(
        scope.map(|declaration| DeclarationReference {
            package: read.package_id(),
            declaration,
        }),
        generation,
    );
    let mut associated = false;
    let mut actuals = |types: &[TypeObjectDigest]| -> Result<(), Diagnostic> {
        admission.validate_occurrence_at(read, types, occurrence)?;
        for ty in types {
            associated |= contains_associated(read, *ty, admission)?;
        }
        Ok(())
    };
    match &record.operation {
        ExpressionOperation::Call { type_arguments, .. } => actuals(type_arguments)?,
        ExpressionOperation::ImplementationCall {
            type_arguments,
            implementations,
            ..
        } => {
            actuals(type_arguments)?;
            for operand in implementations {
                admission
                    .reserve::<usize>(crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH + 1)?;
                for node in operand.walk() {
                    read.validation_work()?;
                    actuals(node.type_arguments())?;
                }
            }
        }
        _ => {
            return Err(compiler_corrupt(
                "compiler_parallel_call",
                "parallel child is no longer a named call",
            ));
        }
    }
    let signature = crate::platform::kernel::function_contract(read, call.function)?;
    associated |= contains_associated(read, signature.result.ty, admission)?;
    for id in &signature.parameters {
        read.validation_work()?;
        let key = OwnerKey::Parameter(*id);
        let ty = if call.function.package == read.package_id() {
            match read.owner(key)? {
                Some(OwnerRecord::Parameter(parameter)) => parameter.ty,
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_parallel_parameter",
                        "parallel source parameter is absent",
                    ));
                }
            }
        } else {
            match read.package_interface_owner(call.function.package, key)? {
                Some(PackageInterfaceRecord::Parameter(parameter)) => parameter.ty,
                _ => {
                    return Err(compiler_corrupt(
                        "compiler_parallel_parameter",
                        "parallel imported parameter is absent",
                    ));
                }
            }
        };
        associated |= contains_associated(read, ty, admission)?;
    }
    for formal in &signature.implementation_parameters {
        read.validation_work()?;
        let contract = crate::platform::kernel::contract_record(read, formal.contract)?;
        associated |= !contract.associated_types.is_empty();
    }
    let overlay = crate::platform::kernel::AppliedTypeRead {
        read,
        types: &call.types,
    };
    admission.validate_occurrence_at(&overlay, &[call.result], occurrence)?;
    if associated {
        let parent = scope
            .and_then(|scope| read.owner(OwnerKey::Declaration(scope)).transpose())
            .transpose()?;
        if generation < 33 || parent.is_none_or(|parent| parent.header().contract_version < 33) {
            return Err(compiler_error(
                DiagnosticClass::Semantic,
                "kernel_owned_associated_generation",
                "associated parallel applications require Graph 33 at the exact parent and wrapper",
            ));
        }
    }
    Ok(())
}
