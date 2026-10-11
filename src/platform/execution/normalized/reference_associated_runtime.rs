//! Canonical application contexts for the existing reference evaluator.
//! Runtime identities and producer catalogues never select expected members.
pub(super) use super::super::reference_types::associated_preparation::{
    Application as CanonicalApplication, Selection as CanonicalSelection,
};
use super::*;

impl ReferenceState<'_> {
    pub(super) fn bound_mapping_operands(
        &self,
        operands: &[crate::platform::kernel::ImplementationOperand],
        depth: usize,
    ) -> Result<(), ExecutionError> {
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
            || operands.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
        {
            return Err(reference_type_error(
                "canonical method operand inventory exceeds finite shape",
            ));
        }
        self.associated_step(operands.len().saturating_add(1))?;
        for operand in operands {
            self.control.check()?;
            if let crate::platform::kernel::ImplementationOperand::Concrete {
                type_arguments,
                implementations,
                ..
            } = operand
            {
                if type_arguments.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN {
                    return Err(reference_type_error(
                        "canonical method actual inventory exceeds finite shape",
                    ));
                }
                self.associated_step(type_arguments.len().saturating_add(1))?;
                self.bound_mapping_operands(implementations, depth + 1)?;
            }
        }
        Ok(())
    }
    pub(super) fn associated_step(&self, count: usize) -> Result<(), ExecutionError> {
        self.control.check()?;
        self.associated_work
            .fetch_update(
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
                |used| {
                    used.checked_add(count as u64).filter(|used| {
                        *used <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK as u64
                    })
                },
            )
            .map_err(|_| {
                reference_resource(
                    "reference_associated_runtime_work",
                    "canonical application reads exhausted their shared finite allowance",
                )
            })?;
        Ok(())
    }
    pub(super) fn resolve_occurrence_types(
        &mut self,
        occurrence: (PackageId, OwnerKey),
        types: &[TypeObjectDigest],
    ) -> Result<Vec<TypeObjectDigest>, ExecutionError> {
        let previous = self.type_occurrence.replace(occurrence);
        let result = self.resolve_type_arguments(types);
        self.type_occurrence = previous;
        result
    }
    pub(super) fn canonical_implementation_bindings(
        &mut self,
        application: &CanonicalApplication,
        parameters: &[crate::platform::kernel::ImplementationParameter],
        supplied: &[AppliedReferenceImplementation],
    ) -> Result<
        BTreeMap<
            crate::platform::semantic_id::ImplementationParameterId,
            AppliedReferenceImplementation,
        >,
        ExecutionError,
    > {
        if parameters.len() != supplied.len() {
            return Err(reference_type_error("canonical witness arity changed"));
        }
        let proof = self.schema.associated_proof().cloned().ok_or_else(|| {
            reference_type_error("canonical witness application proof is missing")
        })?;
        self.associated_step(parameters.len().saturating_add(1))?;
        let expected = &proof
            .application(application, self.control)?
            .implementation_templates;
        for parameter in parameters {
            self.associated_step(
                parameter
                    .type_arguments
                    .len()
                    .saturating_add(parameter.name.as_str().len())
                    .saturating_add(1),
            )?;
        }
        if parameters != expected.as_slice() {
            return Err(reference_type_error(
                "canonical ordered witness formal templates changed",
            ));
        }
        let owner = self.declaration(application.function)?;
        if owner.header.owner != OwnerKey::Declaration(application.function.declaration)
            || owner.header.contract_version
                != proof.context(application, self.control)?.occurrence(
                    (
                        application.function.package,
                        OwnerKey::Declaration(application.function.declaration),
                    ),
                    self.control,
                )?
        {
            return Err(reference_type_error(
                "canonical witness formal scope changed",
            ));
        }
        self.charge_items(
            parameters.len(),
            std::mem::size_of::<(
                crate::platform::semantic_id::ImplementationParameterId,
                AppliedReferenceImplementation,
            )>(),
        )?;
        let mut result = BTreeMap::new();
        for (parameter, selected) in parameters.iter().zip(supplied) {
            self.control.check()?;
            let implementation = self.checked_canonical_implementation(selected)?;
            if parameter.contract != implementation.contract
                || result.insert(parameter.id, selected.clone()).is_some()
            {
                return Err(reference_type_error(
                    "canonical function witness inventory changed",
                ));
            }
        }
        Ok(result)
    }
    pub(super) fn checked_canonical_implementation(
        &mut self,
        selected: &AppliedReferenceImplementation,
    ) -> Result<crate::platform::kernel::OwnedImplementation, ExecutionError> {
        self.control.check()?;
        let proof = self
            .schema
            .associated_proof()
            .cloned()
            .ok_or_else(|| reference_type_error("selected canonical proof is missing"))?;
        let canonical = selected
            .canonical
            .as_ref()
            .ok_or_else(|| reference_type_error("selected canonical handle is missing"))?;
        self.associated_step(
            canonical
                .actuals
                .len()
                .saturating_add(canonical.prerequisites.len())
                .saturating_add(canonical.members.len())
                .saturating_add(1),
        )?;
        let stored = proof.selection_by_id(canonical.identity, self.control)?;
        if !Arc::ptr_eq(stored, canonical)
            || canonical.implementation != selected.implementation
            || canonical.depth != selected.depth
            || canonical.actuals.as_slice() != selected.type_arguments.as_ref()
            || canonical.prerequisites.len() != selected.implementations.len()
        {
            return Err(reference_type_error(
                "witness changed its independently selected meaning",
            ));
        }
        let owner = self.declaration(selected.implementation)?;
        if owner.header.owner != OwnerKey::Declaration(selected.implementation.declaration)
            || owner.header.contract_version != canonical.implementation_generation
        {
            return Err(reference_type_error(
                "selected implementation occurrence changed",
            ));
        }
        let DeclarationPayload::OwnedImplementation(mut implementation) = owner.payload else {
            return Err(reference_type_error(
                "selected owner is not an implementation",
            ));
        };
        self.associated_step(
            implementation
                .type_arguments
                .len()
                .saturating_add(implementation.type_parameters.len())
                .saturating_add(implementation.implementation_parameters.len())
                .saturating_add(implementation.associated_types.len())
                .saturating_add(implementation.methods.len())
                .saturating_add(1),
        )?;
        for parameter in &implementation.implementation_parameters {
            self.associated_step(parameter.type_arguments.len().saturating_add(1))?;
        }
        let contract = self.declaration(canonical.contract)?;
        if contract.header.owner != OwnerKey::Declaration(canonical.contract.declaration)
            || contract.header.contract_version != canonical.contract_generation
            || !matches!(contract.payload, DeclarationPayload::OwnedContract(_))
            || implementation.contract != canonical.contract
            || implementation.self_type != canonical.self_template
            || implementation.type_arguments != canonical.contract_argument_templates
            || implementation.implementation_parameters != canonical.prerequisite_templates
            || implementation.type_parameters != canonical.parameters
            || implementation
                .implementation_parameters
                .iter()
                .map(|parameter| parameter.id)
                .ne(canonical.prerequisite_parameters.iter().copied())
            || implementation.associated_types.len() != canonical.members.len()
        {
            return Err(reference_type_error("selected canonical inventory changed"));
        }
        let DeclarationPayload::OwnedContract(contract) = &contract.payload else {
            return Err(reference_type_error(
                "selected canonical contract kind changed",
            ));
        };
        self.associated_step(
            contract
                .type_parameters
                .len()
                .saturating_add(contract.associated_types.len())
                .saturating_add(contract.methods.len())
                .saturating_add(1),
        )?;
        for method in &contract.methods {
            self.associated_step(method.parameters.len().saturating_add(1))?;
            if let FunctionEffect::Task {
                requirements,
                effect_parameters,
            } = &method.effect
            {
                self.associated_step(requirements.len().saturating_add(effect_parameters.len()))?;
            }
        }
        if contract != proof.contract_template(canonical.contract, self.control)? {
            return Err(reference_type_error(
                "selected canonical contract template changed",
            ));
        }
        if implementation.methods.len() != contract.methods.len()
            || implementation.methods.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
        {
            return Err(reference_type_error(
                "canonical selected method inventory changed",
            ));
        }
        for (index, mapping) in implementation.methods.iter().enumerate() {
            self.associated_step(mapping.type_arguments.len().saturating_add(1))?;
            if mapping.type_arguments.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
                || (index > 0 && implementation.methods[index - 1].method >= mapping.method)
            {
                return Err(reference_type_error(
                    "canonical selected method inventory changed",
                ));
            }
            self.bound_mapping_operands(&mapping.implementations, 0)?;
            if mapping != proof.mapping_template(canonical, mapping.method, self.control)? {
                return Err(reference_type_error(
                    "canonical selected method template changed",
                ));
            }
        }
        let mut bindings = BTreeMap::new();
        for ((parameter, actual), constraint) in canonical
            .parameters
            .iter()
            .zip(&canonical.actuals)
            .zip(&canonical.constraints)
        {
            self.control.check()?;
            let Some(OwnerRecord::TypeParameter(formal)) = self.owner_in_package(
                canonical.implementation.package,
                OwnerKey::TypeParameter(*parameter),
            )?
            else {
                return Err(reference_type_error("canonical selected formal is missing"));
            };
            if formal.header.owner != OwnerKey::TypeParameter(*parameter)
                || formal.declaration != canonical.implementation.declaration
                || formal.constraints != *constraint
                || !self.closed_method_argument(*actual, *constraint)?
                || bindings.insert(*parameter, *actual).is_some()
            {
                return Err(reference_type_error(
                    "canonical selected actual violates its exact formal",
                ));
            }
        }
        for (binding, member) in implementation
            .associated_types
            .iter()
            .zip(&canonical.members)
        {
            self.control.check()?;
            if binding.member != member.member
                || binding.ty != member.template
                || self.contains_associated(binding.ty, 0, &mut 0)?
                || self.method_type(binding.ty, &bindings)? != member.ty
                || !self.schema.is_owned_type(member.ty)
            {
                return Err(reference_type_error(
                    "stored member changed its canonical projection-free RHS",
                ));
            }
        }
        for (child, expected) in selected
            .implementations
            .iter()
            .zip(canonical.prerequisites.iter())
        {
            self.control.check()?;
            if !child
                .canonical
                .as_ref()
                .is_some_and(|actual| Arc::ptr_eq(actual, expected))
            {
                return Err(reference_type_error(
                    "selected prerequisite changed exact canonical identity",
                ));
            }
            self.checked_canonical_implementation(child)?;
        }
        implementation.self_type = canonical.self_type;
        implementation
            .type_arguments
            .clone_from(&canonical.contract_actuals);
        Ok(implementation)
    }

    pub(super) fn selected_by_canonical_id(
        &mut self,
        identity: usize,
        depth: usize,
        visits: &mut usize,
    ) -> Result<AppliedReferenceImplementation, ExecutionError> {
        self.associated_step(1)?;
        self.control.check()?;
        *visits = visits
            .checked_add(1)
            .filter(|count| *count <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
            .ok_or_else(|| {
                reference_resource(
                    "reference_associated_witness_work",
                    "canonical selected DAG traversal exhausted finite work",
                )
            })?;
        let proof = self
            .schema
            .associated_proof()
            .cloned()
            .ok_or_else(|| reference_type_error("canonical selected proof is missing"))?;
        let selected = Arc::clone(proof.selection_by_id(identity, self.control)?);
        if depth.saturating_add(selected.depth)
            > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
        {
            return Err(reference_type_error(
                "canonical selected DAG exceeds finite depth",
            ));
        }
        self.charge_items(
            selected.actuals.len(),
            std::mem::size_of::<TypeObjectDigest>(),
        )?;
        self.charge_items(
            selected.prerequisites.len(),
            std::mem::size_of::<AppliedReferenceImplementation>(),
        )?;
        let mut children = Vec::with_capacity(selected.prerequisites.len());
        for child in selected.prerequisites.iter() {
            children.push(self.selected_by_canonical_id(child.identity, depth + 1, visits)?);
        }
        self.intern_implementation(selected.implementation, selected.actuals.clone(), children)
    }

    pub(super) fn check_associated_occurrence(
        &self,
        occurrence: (PackageId, OwnerKey),
        generation: u16,
    ) -> Result<(), ExecutionError> {
        if let Some(Some(application)) = self.associated_applications.last() {
            self.associated_step(1)?;
            let proof = self.schema.associated_proof().ok_or_else(|| {
                reference_type_error("active canonical application proof is missing")
            })?;
            if proof
                .context(application, self.control)?
                .occurrence(occurrence, self.control)?
                != generation
            {
                return Err(reference_type_error(
                    "runtime occurrence changed exact canonical generation",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn associated_root(
        &self,
        application: Option<&CanonicalApplication>,
        occurrence: (PackageId, OwnerKey),
        template: TypeObjectDigest,
    ) -> Result<Option<TypeObjectDigest>, ExecutionError> {
        if self.schema.associated_proof().is_none() {
            return Ok(None);
        }
        self.associated_step(1)?;
        self.control.check()?;
        if !self.contains_associated(template, 0, &mut 0)? {
            return Ok(None);
        }
        let proof = self.schema.associated_proof().ok_or_else(|| {
            reference_type_error("associated runtime type has no canonical application proof")
        })?;
        let application = application.ok_or_else(|| {
            reference_type_error("associated runtime type has no exact active application")
        })?;
        let closed = proof.closed_root(application, occurrence, template, self.control)?;
        if !self.schema.types.contains_key(&closed) {
            return Err(reference_type_error(
                "canonical associated result escaped schema types",
            ));
        }
        Ok(Some(closed))
    }

    fn contains_associated(
        &self,
        ty: TypeObjectDigest,
        depth: usize,
        visits: &mut u64,
    ) -> Result<bool, ExecutionError> {
        self.control.check()?;
        self.associated_step(1)?;
        *visits = visits
            .checked_add(1)
            .filter(|count| *count <= super::super::value::MAXIMUM_ADMISSION_ITEMS)
            .ok_or_else(|| {
                reference_resource(
                    "reference_associated_runtime_work",
                    "associated occurrence scan exhausted finite work",
                )
            })?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(reference_type_error(
                "associated occurrence exceeds canonical type depth",
            ));
        }
        let form = &self
            .schema
            .types
            .get(&ty)
            .ok_or_else(|| reference_type_error("missing canonical occurrence type"))?
            .form;
        if form.is_associated() {
            return Ok(true);
        }
        match form {
            TypeForm::List { item }
            | TypeForm::Option { item }
            | TypeForm::Stream { item }
            | TypeForm::OwnedSequence { item } => {
                self.contains_associated(*item, depth + 1, visits)
            }
            TypeForm::Map { key, value } => {
                Ok(self.contains_associated(*key, depth + 1, visits)?
                    || self.contains_associated(*value, depth + 1, visits)?)
            }
            TypeForm::Result { ok, error } => {
                Ok(self.contains_associated(*ok, depth + 1, visits)?
                    || self.contains_associated(*error, depth + 1, visits)?)
            }
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                for field in fields {
                    if self.contains_associated(field.ty, depth + 1, visits)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            TypeForm::Applied { arguments, .. } => {
                for argument in arguments {
                    if self.contains_associated(*argument, depth + 1, visits)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            TypeForm::Function { parameters, result }
            | TypeForm::TaskFunction {
                parameters, result, ..
            } => {
                for parameter in parameters {
                    if self.contains_associated(*parameter, depth + 1, visits)? {
                        return Ok(true);
                    }
                }
                self.contains_associated(*result, depth + 1, visits)
            }
            _ => Ok(false),
        }
    }

    pub(super) fn canonical_application(
        &mut self,
        declaration: DeclarationReference,
        application: ReferenceApplication<'_>,
    ) -> Result<Option<CanonicalApplication>, ExecutionError> {
        let Some(proof) = self.schema.associated_proof().cloned() else {
            return Ok(None);
        };
        let atoms = application.effects.iter().try_fold(0_usize, |count, row| {
            count
                .checked_add(row.requirements.len())
                .and_then(|count| count.checked_add(row.parameters.len()))
                .ok_or_else(|| {
                    reference_resource(
                        "reference_associated_runtime_work",
                        "canonical effect key exceeds finite work",
                    )
                })
        })?;
        let key_cost = application
            .types
            .len()
            .saturating_add(application.implementations.len())
            .saturating_add(application.requirements.len())
            .saturating_add(atoms)
            .saturating_add(1);
        self.associated_step(key_cost.saturating_mul(
            (usize::BITS - proof.applications.len().leading_zeros()) as usize + 1,
        ))?;
        self.control.check()?;
        self.charge_items(
            application.types.len(),
            std::mem::size_of::<TypeObjectDigest>(),
        )?;
        self.charge_items(
            application.implementations.len(),
            std::mem::size_of::<usize>(),
        )?;
        self.charge_items(application.effects.len(), std::mem::size_of::<EffectRow>())?;
        self.charge_items(atoms, std::mem::size_of::<RequirementOperand>())?;
        self.charge_items(
            application.requirements.len(),
            std::mem::size_of::<RequirementReference>(),
        )?;
        if application
            .implementations
            .iter()
            .any(|selected| selected.canonical.is_none())
        {
            if self
                .associated_applications
                .last()
                .is_some_and(Option::is_some)
            {
                return Err(reference_type_error(
                    "canonical caller lost an exact witness context",
                ));
            }
            return Ok(None);
        }
        let mut witnesses = Vec::with_capacity(application.implementations.len());
        for selected in application.implementations {
            self.control.check()?;
            let canonical = selected.canonical.as_ref().ok_or_else(|| {
                reference_type_error("witness lacks an independent canonical selection")
            })?;
            let stored = proof.selection_by_id(canonical.identity, self.control)?;
            if !Arc::ptr_eq(stored, canonical) {
                return Err(reference_type_error(
                    "witness belongs to another canonical proof",
                ));
            }
            witnesses.push(canonical.identity);
        }
        let key = CanonicalApplication {
            function: declaration,
            types: application.types.to_vec(),
            witnesses,
            effects: application.effects.to_vec(),
            requirements: application
                .requirements
                .iter()
                .map(|operand| match operand {
                    RequirementOperand::Concrete(reference) => Ok(*reference),
                    RequirementOperand::Parameter(_) => Err(reference_type_error(
                        "canonical application requirements are not closed",
                    )),
                })
                .collect::<Result<_, _>>()?,
        };
        if let (Some(Some(caller)), Some((package, OwnerKey::Expression(expression)))) =
            (self.associated_applications.last(), self.type_occurrence)
            && package == caller.function.package
        {
            let expected = proof.call_target(caller, expression, self.control)?;
            if expected != &key {
                return Err(reference_type_error(
                    "call changed its exact canonical application",
                ));
            }
        }
        if proof.applications.contains_key(&key) {
            proof.application(&key, self.control)?;
            Ok(Some(key))
        } else {
            // Uncalled ordinary generic templates retain their original dynamic route.
            // A selected associated witness cannot use this fallback.
            if application.implementations.iter().any(|selected| {
                selected
                    .canonical
                    .as_ref()
                    .is_some_and(|selection| !selection.members.is_empty())
            }) {
                return Err(reference_type_error(
                    "associated call was not independently closed",
                ));
            }
            Ok(None)
        }
    }
}
